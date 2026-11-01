//! Discord read mode (Windows): messages in a foreign language get their translation drawn over them.
//!
//! Discord shows its messages to screen readers through UI Automation, the accessibility interface of
//! Windows (see discord_parse for what it looks like). Four times a second, while Discord is in front,
//! the visible messages are read and new ones are translated into the user's language; between reads the
//! overlay window over Discord follows the mouse, painting each translation onto its message in the colour
//! behind it. Nothing in Discord is changed or injected.
//! The overlay lets every click through; hovering a message shows the original.

use crate::discord::{discord_window, is_discord_window};
use crate::discord_parse::{self, Message, Node};
use crate::i18n::{self, Msg, msg};
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
use uiautomation::core::{UICacheRequest, UICondition};
use uiautomation::types::{ControlType, Handle, TreeScope, UIProperty};
use uiautomation::variants::Variant;
use uiautomation::{UIAutomation, UIElement};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetDC, GetPixel, ReleaseDC};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetCursorPos, IsIconic, IsWindowVisible,
};

const OVERLAY: &str = "overlay";
/// How often the messages are read (a UI Automation pass, ~15 ms), and how often the overlay follows the
/// mouse in between: Discord lights a row up the moment the mouse is on it, so the overlay must too.
const POLL: Duration = Duration::from_millis(250);
const FRAME: Duration = Duration::from_millis(25);
/// After a click, translations stay hidden this long while Discord opens whatever was clicked (a profile,
/// a menu), then come back only where no popup covers them.
const CLICK_HOLD: Duration = Duration::from_millis(220);
/// Frames the message list must stay still after scrolling before translations come back.
const STILL_FRAMES: u32 = 3;
const FRAMES_PER_POLL: u32 = (POLL.as_millis() / FRAME.as_millis()) as u32;

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

/// Translations by (message id, text): one per text block (None for a block that needs none), or None
/// when the whole message needs none. The text is in the key so an edited message is translated again.
type Cache = HashMap<Key, Option<Vec<Option<String>>>>;
type Key = (String, String);
/// A message to translate: its cache key and its text blocks.
type Job = (Key, Vec<String>);
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Mutex::default);
/// Background colours sampled behind each message, by id.
static COLOURS: LazyLock<Mutex<HashMap<String, (String, String)>>> = LazyLock::new(Mutex::default);
/// The colour Discord lights each background colour up to under the mouse, learned the first time it is
/// seen: from then on the overlay changes with Discord in the same frame, without sampling.
static HOVER: LazyLock<Mutex<HashMap<String, (String, String)>>> = LazyLock::new(Mutex::default);

/// The language messages are translated into: "From" in Settings (the user's language), or the
/// interface language when that is set to detect.
fn my_language() -> String {
    let s = settings::current();
    if !s.discord_read_lang.is_empty() {
        s.discord_read_lang
    } else if s.source_lang != "auto" {
        s.source_lang
    } else {
        i18n::lang().into()
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
        let mut frame = 0;
        loop {
            if frame == 0 {
                reader.tick();
            } else {
                reader.render();
            }
            if reader.on {
                frame = (frame + 1) % FRAMES_PER_POLL;
                std::thread::sleep(FRAME);
            } else {
                // Off: a glance at the setting once a second costs nothing measurable.
                frame = 0;
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    });
}

/// Translates queued messages one at a time, so a busy channel never floods the translation service.
fn translator(jobs: Receiver<Job>) {
    let mut queued = HashSet::new();
    while let Ok((key, parts)) = jobs.recv() {
        if !queued.insert(key.clone()) || CACHE.lock().unwrap().contains_key(&key) {
            continue;
        }
        let result = match translate_parts(&parts) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Discord read mode: {e}");
                queued.remove(&key); // try again when it is seen next
                std::thread::sleep(Duration::from_secs(3)); // not at once: a busy service needs a moment
                continue;
            }
        };
        let mut cache = CACHE.lock().unwrap();
        if cache.len() > 2000 {
            cache.clear();
            queued.clear();
        }
        cache.insert(key, result);
    }
}

/// One translation per block. The blocks go in one request, a line each; when the service does not
/// give back as many lines, each block is sent on its own. A block left (nearly) as it was gets none:
/// it is in my language already, or a name or a link.
fn translate_parts(parts: &[String]) -> Result<Option<Vec<Option<String>>>, String> {
    let mine = my_language();
    let mut s = settings::current();
    s.source_lang = "auto".into();
    s.target_lang = mine.clone();
    let joined = parts
        .iter()
        .map(|p| p.replace('\n', " "))
        .collect::<Vec<_>>()
        .join("\n");
    let r = translate::translate(&joined, &s)?;
    if r.detected.as_deref() == Some(mine.as_str()) {
        return Ok(None);
    }
    let lines: Vec<String> = r
        .text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    let lines = if lines.len() == parts.len() {
        lines
    } else {
        parts
            .iter()
            .map(|p| {
                if translatable(p) {
                    translate::translate(p, &s).map(|r| r.text)
                } else {
                    Ok(p.clone())
                }
            })
            .collect::<Result<_, _>>()?
    };
    let out: Vec<Option<String>> = parts
        .iter()
        .zip(lines)
        .map(|(p, t)| (translatable(p) && !discord_parse::same_words(&t, p)).then_some(t))
        .collect();
    Ok(out.iter().any(Option::is_some).then_some(out))
}

/// Worth translating: has letters, is not just a link.
fn translatable(text: &str) -> bool {
    let text = text.trim();
    text.chars().any(char::is_alphabetic) && !(text.starts_with("http") && !text.contains(' '))
}

struct Reader {
    automation: UIAutomation,
    cache_request: Option<UICacheRequest>,
    /// Discord's popups (profile cards, menus, the emoji picker): Window or Menu elements inside it.
    popup_condition: Option<UICondition>,
    /// Where those popups are: translations never cover them.
    popups: Vec<(i32, i32, i32, i32)>,
    /// What had the keyboard focus last time: popups take it, so a change means "look again".
    last_focus: String,
    scans: u32,
    /// The message list itself (inside `list`, the scroller). Its place on screen is checked every frame:
    /// it moves the moment Discord scrolls or a new message pushes the others up.
    list_inner: Option<UIElement>,
    last_list_rect: Option<(i32, i32, i32, i32)>,
    still_frames: u32,
    window: Option<HWND>,
    hold_until: Option<Instant>,
    app: AppHandle,
    to_translate: Sender<Job>,
    /// The element holding the message list, found by a full scan and reused while it keeps working.
    list: Option<UIElement>,
    last_messages: Vec<Message>,
    last_items: Vec<Item>,
    last_bounds: (i32, i32, i32, i32),
    /// Read mode is on (the loop runs at frame pace).
    on: bool,
    /// The messages are moving (scrolling): draw nothing until they settle.
    moving: bool,
    /// Discord's client area on screen, when it is in front.
    origin: Option<(i32, i32, i32, i32)>,
    frames: u32,
}

impl Reader {
    fn new(automation: UIAutomation, app: AppHandle, to_translate: Sender<Job>) -> Self {
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
        let popup_condition = popup_condition(&automation);
        Self {
            automation,
            popup_condition,
            popups: Vec::new(),
            last_focus: String::new(),
            scans: 0,
            list_inner: None,
            last_list_rect: None,
            still_frames: 0,
            window: None,
            hold_until: None,
            cache_request,
            app,
            to_translate,
            list: None,
            last_messages: Vec::new(),
            last_items: Vec::new(),
            last_bounds: (0, 0, 0, 0),
            on: false,
            moving: false,
            origin: None,
            frames: 0,
        }
    }

    /// Reads the messages on screen and queues new ones for translation.
    fn tick(&mut self) {
        let enabled = settings::current().discord_read;
        self.on = enabled;
        let window = enabled.then(discord_window).flatten();
        self.window = window;
        let Some(window) = window else {
            self.last_messages.clear();
            self.origin = None;
            self.list_inner = None;
            if !enabled {
                // Turned off: the overlay's WebView goes too, and with it its memory.
                self.list = None;
                if let Some(overlay) = self.app.get_webview_window(OVERLAY) {
                    let _ = overlay.destroy();
                    self.last_items.clear();
                    self.last_bounds = (0, 0, 0, 0);
                }
                return;
            }
            return self.show(Vec::new());
        };
        let messages = self.read(window);
        self.find_popups(window, false);
        // Moving messages mean scrolling (or a new message pushing them up): stay out of the way until
        // they settle, rather than trail behind them. With the list element at hand, render() sees that
        // every frame instead.
        if self.list_inner.is_none() {
            self.moving = messages.iter().any(|m| {
                self.last_messages
                    .iter()
                    .any(|old| old.id == m.id && old.row != m.row)
            });
        }
        self.last_messages = messages;
        self.origin = client_rect(window);
        if let Some(origin) = self.origin {
            self.place_overlay(origin);
        }
        let cache = CACHE.lock().unwrap();
        for m in &self.last_messages {
            let key = (m.id.clone(), m.text());
            if !cache.contains_key(&key) {
                let parts = m.parts.iter().map(|p| p.text.clone()).collect();
                let _ = self.to_translate.send((key, parts));
            }
        }
        drop(cache);
        self.render();
    }

    /// Draws the translations of the last read messages, following the mouse: cheap enough for every frame.
    fn render(&mut self) {
        if self.hold() {
            return self.show(Vec::new());
        }
        let Some(origin) = self.origin.filter(|_| !self.moving) else {
            return self.show(Vec::new());
        };
        let cursor = cursor();
        let inside = |(l, t, r, b): (i32, i32, i32, i32)| {
            (l..r).contains(&cursor.0) && (t..b).contains(&cursor.1)
        };
        let cache = CACHE.lock().unwrap();
        let mut items = Vec::new();
        for m in &self.last_messages {
            let Some(Some(translations)) = cache.get(&(m.id.clone(), m.text())) else {
                continue;
            };
            let hovered = inside(m.row);
            for (i, (part, translation)) in m.parts.iter().zip(translations).enumerate() {
                let Some(translation) = translation else {
                    continue;
                };
                // On the text: show the original. Under a Discord popup: leave the popup alone.
                if inside(part.rect) || self.popups.iter().any(|p| overlaps(*p, part.rect)) {
                    continue;
                }
                let (l, t, r, b) = part.rect;
                // Elsewhere on the message Discord lights the row up: follow it.
                let (bg, fg) = colours(&format!("{}:{i}", m.id), part.rect, m.row, hovered);
                items.push(Item {
                    key: format!("{}:{i}:{t}", m.id),
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
        drop(cache);
        self.show(items);
    }

    /// Whether to draw nothing this frame: a mouse button is down or was just let go (Discord may be opening
    /// a popup), or the message list is moving. Once it is still again the messages are read afresh at once.
    fn hold(&mut self) -> bool {
        let pressed = |key: u16| unsafe { GetAsyncKeyState(key as i32) as u16 & 0x8000 != 0 };
        if pressed(VK_LBUTTON) || pressed(VK_RBUTTON) {
            self.hold_until = Some(Instant::now() + CLICK_HOLD);
            return true;
        }
        if let Some(until) = self.hold_until {
            if Instant::now() < until {
                return true;
            }
            self.hold_until = None;
            if let Some(window) = self.window {
                self.find_popups(window, true);
            }
        }
        let Some(rect) = self
            .list_inner
            .as_ref()
            .and_then(|l| l.get_bounding_rectangle().ok())
            .map(|r| (r.get_left(), r.get_top(), r.get_right(), r.get_bottom()))
        else {
            return false;
        };
        if self.last_list_rect != Some(rect) {
            let first = self.last_list_rect.is_none();
            self.last_list_rect = Some(rect);
            if !first {
                self.moving = true;
                self.still_frames = 0;
            }
            return self.moving;
        }
        if self.moving {
            self.still_frames += 1;
            if self.still_frames < STILL_FRAMES {
                return true;
            }
            // Still: read where everything is now, then draw again.
            self.moving = false;
            self.tick();
            return true; // tick() has drawn
        }
        false
    }

    /// Finds Discord's open popups. A full search costs about as much as reading the messages, so it runs
    /// when the keyboard focus moved (opening a popup moves it) and once a second otherwise.
    fn find_popups(&mut self, window: HWND, force: bool) {
        self.scans = self.scans.wrapping_add(1);
        let focus = self
            .automation
            .get_focused_element()
            .ok()
            .map(|f| {
                let r = f.get_bounding_rectangle().ok();
                format!(
                    "{:?}{:?}{:?}",
                    f.get_control_type().ok(),
                    f.get_name().ok(),
                    r.map(|r| (r.get_left(), r.get_top()))
                )
            })
            .unwrap_or_default();
        if !force && focus == self.last_focus && !self.scans.is_multiple_of(4) {
            return;
        }
        self.last_focus = focus;
        let (Some(condition), Ok(root)) = (
            &self.popup_condition,
            self.automation
                .element_from_handle(Handle::from(window as isize)),
        ) else {
            return self.popups.clear();
        };
        self.popups = root
            .find_all(TreeScope::Descendants, condition)
            .unwrap_or_default()
            .iter()
            .filter_map(|e| e.get_bounding_rectangle().ok())
            .map(|r| (r.get_left(), r.get_top(), r.get_right(), r.get_bottom()))
            .filter(|r| r.2 > r.0 && r.3 > r.1)
            .collect();
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
                self.list_inner = inner_list(&list);
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
        self.list_inner = self.list.as_ref().and_then(inner_list);
        if self.list_inner.is_none() {
            self.last_list_rect = None;
        }
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
        self.frames = self.frames.wrapping_add(1);
        if items != self.last_items || (!items.is_empty() && self.frames.is_multiple_of(80)) {
            let _ = self.app.emit_to(OVERLAY, "overlay", &items);
            self.last_items = items;
        }
    }
}

fn popup_condition(automation: &UIAutomation) -> Option<UICondition> {
    let of = |t: ControlType| {
        automation
            .create_property_condition(UIProperty::ControlType, Variant::from(t as i32), None)
            .ok()
    };
    automation
        .create_or_condition(of(ControlType::Window)?, of(ControlType::Menu)?)
        .ok()
}

fn overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
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

/// A plain copy of a cached UI Automation subtree, for discord_parse.
fn snapshot(e: &UIElement) -> Node {
    let rect = e.get_cached_bounding_rectangle().ok();
    Node {
        kind: e
            .get_cached_control_type()
            .map(|k| format!("{k:?}"))
            .unwrap_or_default(),
        id: e.get_cached_automation_id().unwrap_or_default(),
        name: e.get_cached_name().unwrap_or_default(),
        rect: rect.map_or((0, 0, 0, 0), |r| {
            (r.get_left(), r.get_top(), r.get_right(), r.get_bottom())
        }),
        children: e
            .get_cached_children()
            .unwrap_or_default()
            .iter()
            .map(snapshot)
            .collect(),
    }
}

/// The messages worth translating in a cached subtree, and the path to the message list.
fn collect(root: &UIElement) -> (Vec<Message>, Option<Vec<usize>>) {
    let (mut messages, list) = discord_parse::messages(&snapshot(root));
    messages.retain(|m| m.parts.iter().any(|p| translatable(&p.text)));
    (messages, list)
}

/// The List element of chat messages inside the scroller.
fn inner_list(scroller: &UIElement) -> Option<UIElement> {
    scroller
        .get_cached_children()
        .ok()?
        .into_iter()
        .find(|c| c.get_cached_control_type().ok() == Some(ControlType::List))
}

/// The cached element at `path`: child indexes from `root`.
fn element_at(root: &UIElement, path: &[usize]) -> Option<UIElement> {
    let mut element = root.clone();
    for &i in path {
        element = element.get_cached_children().ok()?.into_iter().nth(i)?;
    }
    Some(element)
}

/// The tree as text, in the format discord_parse's tests read.
fn dump(node: &Node, depth: usize, out: &mut String, count: &mut usize) {
    *count += 1;
    if *count > 6000 {
        return;
    }
    let name: String = node.name.chars().take(100).collect();
    let _ = writeln!(
        out,
        "{}Some({}) id={:?} name={:?} rect=Some({:?})",
        "  ".repeat(depth),
        node.kind,
        node.id,
        name.replace('\n', " "),
        node.rect,
    );
    for child in &node.children {
        dump(child, depth + 1, out, count);
    }
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

/// The colour behind the message and a text colour that reads on it. Sampled at the row's right edge,
/// beside the translation box (so never the overlay's own paint), once per message. Under the mouse
/// Discord shows the row lighter: that colour is learned the first time and reused from then on.
fn colours(
    key: &str,
    rect: (i32, i32, i32, i32),
    row: (i32, i32, i32, i32),
    hovered: bool,
) -> (String, String) {
    let base = COLOURS.lock().unwrap().get(key).cloned();
    if !hovered && let Some(c) = base {
        return c;
    }
    if hovered
        && let Some(base) = &base
        && let Some(c) = HOVER.lock().unwrap().get(&base.0)
    {
        return c.clone();
    }
    let (_, t, _, b) = rect;
    let x = row.2 - 6;
    let samples: Vec<u32> = unsafe {
        let dc = GetDC(std::ptr::null_mut());
        let s = [t + 2, (t + b) / 2, b - 2]
            .into_iter()
            .map(|y| GetPixel(dc, x, y))
            .collect();
        ReleaseDC(std::ptr::null_mut(), dc);
        s
    };
    let result = describe(most_common(&samples));
    match (hovered, base) {
        (false, _) => {
            let mut cache = COLOURS.lock().unwrap();
            if cache.len() > 2000 {
                cache.clear();
            }
            cache.insert(key.to_string(), result.clone());
        }
        // Until Discord has repainted, the sample is still the plain colour: learn only a real change.
        (true, Some(base)) if base.0 != result.0 => {
            HOVER.lock().unwrap().insert(base.0, result.clone());
        }
        _ => {}
    }
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
                msg(Msg::ReportWritten, &[])
            }
            Err(e) => e.clone(),
        };
        notify(&app, &message);
    });
}

fn write_report(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let window = find_discord().ok_or_else(|| msg(Msg::NoDiscord, &[]))?;
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
        let tree_root = snapshot(&root);
        let (messages, list) = discord_parse::messages(&tree_root);
        let mut nodes = 0;
        let mut tree = String::new();
        dump(&tree_root, 0, &mut tree, &mut nodes);
        let _ = writeln!(
            out,
            "== attempt {attempt}: {nodes} elements in {took:?}, {} messages, list path {list:?}",
            messages.len()
        );
        for m in &messages {
            for p in &m.parts {
                let _ = writeln!(out, "message {} {:?}: {:?}", m.id, p.rect, p.text);
            }
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
