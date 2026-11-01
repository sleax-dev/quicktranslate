//! Discord read mode (Windows): messages in a foreign language get their translation drawn over them.
//!
//! Discord shows its messages to screen readers through UI Automation, the accessibility interface of
//! Windows. Each message's text sits in an element whose id is `message-content-<message id>`, with its
//! place on the screen. Twice a second, while Discord is in front, the visible messages are read,
//! new ones are translated into the user's language, and an overlay window over Discord paints each
//! translation onto its message, in the colour behind it. Nothing in Discord is changed or injected.
//! The overlay lets every click through; hovering a message shows the original.

use crate::discord::{discord_window, is_discord_window};
use crate::i18n::{TURKISH, t};
use crate::{notify, settings, translate};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use uiautomation::core::UICacheRequest;
use uiautomation::types::{Handle, TreeScope, UIProperty};
use uiautomation::{UIAutomation, UIElement};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetDC, GetPixel, ReleaseDC};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetCursorPos, IsIconic, IsWindowVisible,
};

const OVERLAY: &str = "overlay";
const POLL: Duration = Duration::from_millis(250);

/// One message as read from Discord: screen pixels.
#[derive(Clone, PartialEq)]
struct Message {
    id: String,
    text: String,
    rect: (i32, i32, i32, i32), // left, top, right, bottom
}

/// What the overlay draws: one translation, in pixels relative to the overlay's top-left corner.
#[derive(Clone, PartialEq, Serialize)]
struct Item {
    key: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    text: String,
    bg: String,
    fg: String,
}

/// Translations by (message id, text): Some(translation), or None when the message needs none.
/// The text is in the key so an edited message is translated again.
type Cache = HashMap<(String, String), Option<String>>;
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Mutex::default);
/// Background colours sampled behind each message, by id.
static COLOURS: LazyLock<Mutex<HashMap<String, (String, String)>>> = LazyLock::new(Mutex::default);

/// The language messages are translated into: "From" in Settings (the user's language), or the
/// system's language when that is set to detect.
fn my_language() -> String {
    let s = settings::current();
    if s.source_lang != "auto" {
        s.source_lang
    } else if *TURKISH {
        "tr".into()
    } else {
        "en".into()
    }
}

pub fn start(app: &AppHandle) {
    let (to_translate, jobs) = channel();
    std::thread::spawn(move || translator(jobs));
    let app = app.clone();
    std::thread::spawn(move || {
        let automation = match UIAutomation::new() {
            Ok(a) => a,
            Err(e) => return log::warn!("Discord read mode: UI Automation unavailable: {e}"),
        };
        let mut reader = Reader::new(automation, app, to_translate);
        loop {
            std::thread::sleep(POLL);
            reader.tick();
        }
    });
}

/// Translates queued messages one at a time, so a busy channel never floods the translation service.
fn translator(jobs: Receiver<(String, String)>) {
    let mut queued = HashSet::new();
    while let Ok(job) = jobs.recv() {
        if !queued.insert(job.clone()) || CACHE.lock().unwrap().contains_key(&job) {
            continue;
        }
        let mine = my_language();
        let mut s = settings::current();
        s.source_lang = "auto".into();
        s.target_lang = mine.clone();
        let result = match translate::translate(&job.1, &s) {
            Ok(r)
                if r.detected.as_deref() != Some(mine.as_str())
                    && r.text.trim() != job.1.trim() =>
            {
                Some(r.text)
            }
            Ok(_) => None,
            Err(e) => {
                log::warn!("Discord read mode: {e}");
                queued.remove(&job); // try again when it is seen next
                continue;
            }
        };
        let mut cache = CACHE.lock().unwrap();
        if cache.len() > 2000 {
            cache.clear();
            queued.clear();
        }
        cache.insert(job, result);
    }
}

/// Worth translating: has letters, is not just a link.
fn translatable(text: &str) -> bool {
    let text = text.trim();
    text.chars().any(char::is_alphabetic) && !(text.starts_with("http") && !text.contains(' '))
}

struct Reader {
    automation: UIAutomation,
    cache_request: Option<UICacheRequest>,
    app: AppHandle,
    to_translate: Sender<(String, String)>,
    /// The element holding the message list, found by a full scan and reused while it keeps working.
    list: Option<UIElement>,
    last_messages: Vec<Message>,
    last_items: Vec<Item>,
    last_bounds: (i32, i32, i32, i32),
    ticks: u32,
}

impl Reader {
    fn new(
        automation: UIAutomation,
        app: AppHandle,
        to_translate: Sender<(String, String)>,
    ) -> Self {
        let cache_request = automation.create_cache_request().ok().and_then(|r| {
            for p in [
                UIProperty::AutomationId,
                UIProperty::Name,
                UIProperty::BoundingRectangle,
                UIProperty::ControlType,
            ] {
                r.add_property(p).ok()?;
            }
            r.set_tree_scope(TreeScope::Subtree).ok()?;
            Some(r)
        });
        Self {
            automation,
            cache_request,
            app,
            to_translate,
            list: None,
            last_messages: Vec::new(),
            last_items: Vec::new(),
            last_bounds: (0, 0, 0, 0),
            ticks: 0,
        }
    }

    fn tick(&mut self) {
        let enabled = settings::current().discord_read;
        let window = enabled.then(discord_window).flatten();
        let Some(window) = window else {
            if !enabled {
                self.list = None;
            }
            return self.show(Vec::new());
        };
        let messages = self.read(window);
        // Moving messages mean scrolling (or a new message pushing them up): stay out of the way until
        // they settle, rather than trail behind them.
        let moved = messages.iter().any(|m| {
            self.last_messages
                .iter()
                .any(|old| old.id == m.id && old.rect != m.rect)
        });
        self.last_messages = messages.clone();
        if moved {
            return self.show(Vec::new());
        }

        let Some(origin) = client_rect(window) else {
            return self.show(Vec::new());
        };
        self.place_overlay(origin);
        let cursor = cursor();
        let cache = CACHE.lock().unwrap().clone();
        let mut items = Vec::new();
        for m in messages {
            let key = (m.id.clone(), m.text.clone());
            match cache.get(&key) {
                None => {
                    let _ = self.to_translate.send(key);
                }
                Some(None) => {}
                Some(Some(translation)) => {
                    let (l, t, r, b) = m.rect;
                    // Under the mouse: show the original.
                    if (l..r).contains(&cursor.0) && (t..b).contains(&cursor.1) {
                        continue;
                    }
                    let (bg, fg) = colours(&m);
                    items.push(Item {
                        key: format!("{}:{t}", m.id),
                        x: l - origin.0,
                        y: t - origin.1,
                        w: r - l,
                        h: b - t,
                        text: translation.clone(),
                        bg,
                        fg,
                    });
                }
            }
        }
        self.show(items);
    }

    /// The visible messages, from the remembered list element when it still works, else from a full scan.
    fn read(&mut self, window: HWND) -> Vec<Message> {
        let Some(request) = &self.cache_request else {
            return Vec::new();
        };
        if let Some(list) = &self.list
            && let Ok(list) = list.build_updated_cache(request)
        {
            let messages = collect(&list).0;
            if !messages.is_empty() {
                return messages;
            }
        }
        let Ok(root) = self
            .automation
            .element_from_handle_build_cache(Handle::from(window as isize), request)
        else {
            return Vec::new();
        };
        let (messages, path) = collect(&root);
        self.list = path.and_then(|path| element_at(&root, &path));
        messages
    }

    /// Moves the overlay onto Discord, creating it the first time.
    fn place_overlay(&mut self, bounds: (i32, i32, i32, i32)) {
        let overlay = match self.app.get_webview_window(OVERLAY) {
            Some(w) => w,
            None => match create_overlay(&self.app) {
                Ok(w) => w,
                Err(e) => return log::warn!("Discord read mode: overlay: {e}"),
            },
        };
        if bounds != self.last_bounds {
            let (l, t, r, b) = bounds;
            let _ = overlay.set_position(PhysicalPosition::new(l, t));
            let _ = overlay.set_size(PhysicalSize::new(
                (r - l).max(1) as u32,
                (b - t).max(1) as u32,
            ));
            self.last_bounds = bounds;
        }
    }

    fn show(&mut self, items: Vec<Item>) {
        // Sent again every two seconds too: a just-created overlay misses what came before it loaded.
        self.ticks = self.ticks.wrapping_add(1);
        if items != self.last_items || (!items.is_empty() && self.ticks.is_multiple_of(8)) {
            let _ = self.app.emit_to(OVERLAY, "overlay", &items);
            self.last_items = items;
        }
    }
}

/// A transparent, click-through, always-on-top window that never takes the focus from Discord.
/// It stays shown: hiding means drawing nothing, so showing it again cannot steal the focus.
fn create_overlay(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let overlay = WebviewWindowBuilder::new(app, OVERLAY, WebviewUrl::App("index.html".into()))
        .title("QuickTranslate overlay")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .focusable(false)
        .position(-10.0, -10.0)
        .inner_size(1.0, 1.0)
        .build()?;
    overlay.set_ignore_cursor_events(true)?;
    Ok(overlay)
}

/// The messages in a cached subtree, and the path to the smallest element holding all of them.
fn collect(root: &UIElement) -> (Vec<Message>, Option<Vec<usize>>) {
    let mut messages = Vec::new();
    let mut common: Option<Vec<usize>> = None;
    let mut path = Vec::new();
    walk(root, &mut path, &mut |element, path| {
        let id = element.get_cached_automation_id().unwrap_or_default();
        let Some(message_id) = id.strip_prefix("message-content-") else {
            return true;
        };
        let Ok(rect) = element.get_cached_bounding_rectangle() else {
            return false;
        };
        let rect = (
            rect.get_left(),
            rect.get_top(),
            rect.get_right(),
            rect.get_bottom(),
        );
        let text = text_of(element);
        if rect.2 > rect.0 && rect.3 > rect.1 && translatable(&text) {
            messages.push(Message {
                id: message_id.to_string(),
                text,
                rect,
            });
            // The parent of the message's own element: shared prefix of all of them is the list.
            let parent = &path[..path.len().saturating_sub(1)];
            common = Some(match common.take() {
                None => parent.to_vec(),
                Some(c) => c
                    .iter()
                    .zip(parent)
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| *a)
                    .collect(),
            });
        }
        false // no messages inside a message
    });
    (messages, common)
}

/// The cached element at `path`: child indexes from `root`.
fn element_at(root: &UIElement, path: &[usize]) -> Option<UIElement> {
    let mut element = root.clone();
    for &i in path {
        element = element.get_cached_children().ok()?.into_iter().nth(i)?;
    }
    Some(element)
}

/// Depth-first over the cached tree; `visit` returns whether to go into the element's children.
fn walk(
    element: &UIElement,
    path: &mut Vec<usize>,
    visit: &mut dyn FnMut(&UIElement, &[usize]) -> bool,
) {
    if !visit(element, path) {
        return;
    }
    for (i, child) in element
        .get_cached_children()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        path.push(i);
        walk(child, path, visit);
        path.pop();
    }
}

/// The message's text: its name, or the text of everything inside it.
fn text_of(element: &UIElement) -> String {
    let name = element.get_cached_name().unwrap_or_default();
    if !name.trim().is_empty() {
        return name.trim().to_string();
    }
    let mut parts = Vec::new();
    walk(element, &mut Vec::new(), &mut |e, _| {
        let children = e.get_cached_children().map(|c| c.len()).unwrap_or(0);
        if children == 0 {
            let name = e.get_cached_name().unwrap_or_default();
            if !name.trim().is_empty() {
                parts.push(name);
            }
        }
        true
    });
    parts
        .join("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Discord's client area in screen pixels.
fn client_rect(window: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetClientRect(window, &mut rect) == 0 {
            return None;
        }
        let mut origin = POINT { x: 0, y: 0 };
        ClientToScreen(window, &mut origin);
        Some((
            origin.x,
            origin.y,
            origin.x + rect.right,
            origin.y + rect.bottom,
        ))
    }
}

fn cursor() -> (i32, i32) {
    let mut p = POINT { x: -1, y: -1 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

/// The colour behind the message (sampled once per message along its right edge, where text rarely
/// reaches) and a text colour that reads on it.
fn colours(m: &Message) -> (String, String) {
    let mut cache = COLOURS.lock().unwrap();
    if let Some(c) = cache.get(&m.id) {
        return c.clone();
    }
    let (_, t, r, b) = m.rect;
    let samples: Vec<u32> = unsafe {
        let dc = GetDC(std::ptr::null_mut());
        let s = [t + 2, (t + b) / 2, b - 2]
            .into_iter()
            .map(|y| GetPixel(dc, r - 3, y))
            .collect();
        ReleaseDC(std::ptr::null_mut(), dc);
        s
    };
    let colour = most_common(&samples);
    let result = describe(colour);
    if cache.len() > 2000 {
        cache.clear();
    }
    cache.insert(m.id.clone(), result.clone());
    result
}

fn most_common(samples: &[u32]) -> u32 {
    samples
        .iter()
        .copied()
        .max_by_key(|c| samples.iter().filter(|s| *s == c).count())
        .unwrap_or(0x0031_3338)
}

/// A COLORREF (0x00BBGGRR) as CSS, with Discord's own text colour for light or dark backgrounds.
fn describe(colour: u32) -> (String, String) {
    let colour = if colour == u32::MAX {
        0x0031_3338
    } else {
        colour
    }; // CLR_INVALID: off screen
    let (r, g, b) = (colour & 0xff, (colour >> 8) & 0xff, (colour >> 16) & 0xff);
    let light = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32 > 140.0;
    let fg = if light { "#313338" } else { "#dbdee1" };
    (format!("#{r:02x}{g:02x}{b:02x}"), fg.to_string())
}

/// Writes what UI Automation shows of the Discord window to a file, for fixing the reader when Discord
/// changes. Discord does not need to be in front: the first visible Discord window is used.
pub fn diagnose(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let result = write_report(&app);
        let message = match &result {
            Ok(path) => {
                use tauri_plugin_opener::OpenerExt;
                let _ = app.opener().reveal_item_in_dir(path);
                t(
                    "Discord tanı raporu oluşturuldu. Bu dosya ekranda görünen mesajları içerir.",
                    "Discord report written. The file contains the messages that were on screen.",
                )
            }
            Err(e) => e.clone(),
        };
        notify(&app, &message);
    });
}

fn write_report(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let window = find_discord().ok_or_else(|| {
        t(
            "Açık bir Discord penceresi bulunamadı",
            "No open Discord window found",
        )
    })?;
    let automation = UIAutomation::new().map_err(|e| e.to_string())?;
    let request = automation
        .create_cache_request()
        .map_err(|e| e.to_string())?;
    for p in [
        UIProperty::AutomationId,
        UIProperty::Name,
        UIProperty::BoundingRectangle,
        UIProperty::ControlType,
    ] {
        request.add_property(p).map_err(|e| e.to_string())?;
    }
    request
        .set_tree_scope(TreeScope::Subtree)
        .map_err(|e| e.to_string())?;

    let mut out = String::new();
    // The first read can come back thin while Chromium switches its accessibility on.
    for attempt in 1..=2 {
        let started = Instant::now();
        let root = automation
            .element_from_handle_build_cache(Handle::from(window as isize), &request)
            .map_err(|e| e.to_string())?;
        let took = started.elapsed();
        let (messages, list) = collect(&root);
        let mut nodes = 0;
        let mut tree = String::new();
        walk(&root, &mut Vec::new(), &mut |e, path| {
            nodes += 1;
            if nodes <= 6000 {
                let r = e.get_cached_bounding_rectangle().ok();
                let name: String = e
                    .get_cached_name()
                    .unwrap_or_default()
                    .chars()
                    .take(100)
                    .collect();
                let _ = writeln!(
                    tree,
                    "{}{:?} id={:?} name={:?} rect={:?}",
                    "  ".repeat(path.len()),
                    e.get_cached_control_type().ok(),
                    e.get_cached_automation_id().unwrap_or_default(),
                    name.replace('\n', " "),
                    r.map(|r| (r.get_left(), r.get_top(), r.get_right(), r.get_bottom())),
                );
            }
            true
        });
        let _ = writeln!(
            out,
            "== attempt {attempt}: {nodes} elements in {took:?}, {} messages, list path {list:?}",
            messages.len()
        );
        for m in &messages {
            let _ = writeln!(out, "message {} {:?}: {:?}", m.id, m.rect, m.text);
        }
        if attempt == 2 || !messages.is_empty() {
            out.push_str(&tree);
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("discord-report.txt");
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(path)
}

/// The first visible, not minimised Discord window.
fn find_discord() -> Option<HWND> {
    unsafe extern "system" fn each(window: HWND, found: isize) -> i32 {
        unsafe {
            if IsWindowVisible(window) != 0 && IsIconic(window) == 0 && is_discord_window(window) {
                *(found as *mut HWND) = window;
                return 0; // stop
            }
        }
        1
    }
    let mut found: HWND = std::ptr::null_mut();
    unsafe { EnumWindows(Some(each), &mut found as *mut HWND as isize) };
    (!found.is_null()).then_some(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_and_filters() {
        assert_eq!(describe(0x0038_3331).0, "#313338"); // Discord dark
        assert_eq!(describe(0x00ff_ffff).1, "#313338"); // light theme: dark text
        assert_eq!(most_common(&[1, 2, 2]), 2);
        assert!(translatable("Hello there"));
        assert!(!translatable("https://example.com/x"));
        assert!(!translatable("😀 👍"));
    }
}
