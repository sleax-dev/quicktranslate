//! Discord send mode (Windows): Enter in Discord translates the message box first, then sends it.
//! A low-level keyboard hook takes the Enter key away from Discord, the message is copied, translated
//! and pasted back, and an Enter of our own sends it. Our own Enter is marked as injected, so the hook
//! lets it through.

use crate::i18n::{Msg, msg};
use crate::{Busy, copy, notify, press_ctrl, remember, settings, translate, wait};
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::{
    GetCurrentThreadId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowTextW, GetWindowThreadProcessId,
    HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT,
};

static APP: Mutex<Option<AppHandle>> = Mutex::new(None);
/// The message whose translation failed: a second Enter sends it untranslated.
static FAILED: Mutex<Option<String>> = Mutex::new(None);
/// The thread running the hook, while it is installed.
static HOOK_THREAD: Mutex<Option<u32>> = Mutex::new(None);

pub fn init(app: &AppHandle) {
    *APP.lock().unwrap_or_else(|e| e.into_inner()) = Some(app.clone());
    sync();
}

/// Installs the hook while the setting is on and removes it when it goes off. A low-level hook sees every
/// key press on the system, so it only exists while it has a job.
pub fn sync() {
    let mut thread = HOOK_THREAD.lock().unwrap_or_else(|e| e.into_inner());
    let wanted = settings::current().discord_send;
    match (*thread, wanted) {
        (None, true) => {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || run_hook(tx));
            *thread = rx.recv().ok().flatten();
        }
        (Some(id), false) => {
            unsafe { PostThreadMessageW(id, WM_QUIT, 0, 0) };
            *thread = None;
        }
        _ => {}
    }
}

/// Installs the hook and runs the message loop it is called through, until WM_QUIT. Sends back the
/// thread id, or None when the hook could not be installed.
fn run_hook(started: std::sync::mpsc::Sender<Option<u32>>) {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        // Creates this thread's message queue before anyone posts WM_QUIT to it.
        PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), std::ptr::null_mut(), 0);
        if hook.is_null() {
            log::warn!("Discord send mode: keyboard hook could not be installed");
            let _ = started.send(None);
            return;
        }
        let _ = started.send(Some(GetCurrentThreadId()));
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
        UnhookWindowsHookEx(hook);
    }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam == WM_KEYDOWN as WPARAM {
        let key = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if key.vkCode == VK_RETURN as u32 && key.flags & LLKHF_INJECTED == 0 && take_enter() {
            return 1; // Discord never sees this Enter
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// Decides, quickly (Windows drops slow hooks), whether this Enter is ours, and starts the work if so.
fn take_enter() -> bool {
    let held = |vk| unsafe { GetAsyncKeyState(vk as i32) } < 0;
    // Shift+Enter is a new line; any modifier means the user wants Discord's own behaviour.
    if [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
        .into_iter()
        .any(held)
        || !settings::current().discord_send
        || !discord_in_front()
    {
        return false;
    }
    // Held Enter repeats while a message is being translated: swallow the repeats.
    let Some(busy) = Busy::acquire() else {
        return true;
    };
    let Some(app) = APP.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
        return false;
    };
    std::thread::spawn(move || {
        let _busy = busy;
        if let Err(e) = translate_and_send(&app) {
            log::warn!("{e}");
            notify(&app, &e);
        }
    });
    true
}

/// The Discord app (Discord, PTB, Canary) or a browser tab on discord.com ("… - Discord - Google Chrome").
fn discord_in_front() -> bool {
    discord_window().is_some()
}

/// The foreground window, when it is Discord.
pub(crate) fn discord_window() -> Option<HWND> {
    let window = unsafe { GetForegroundWindow() };
    (!window.is_null() && is_discord_window(window)).then_some(window)
}

pub(crate) fn is_discord_window(window: HWND) -> bool {
    unsafe {
        let mut title = [0u16; 512];
        let len = GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
        let title = String::from_utf16_lossy(&title[..len.max(0) as usize]);

        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        let mut exe = String::new();
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !process.is_null() {
            let mut path = [0u16; 1024];
            let mut size = path.len() as u32;
            if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut size)
                != 0
            {
                exe = String::from_utf16_lossy(&path[..size as usize]);
            }
            CloseHandle(process);
        }
        !title.is_empty() && is_discord(&title, &exe)
    }
}

fn is_discord(title: &str, exe: &str) -> bool {
    let name = exe.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
    name.starts_with("discord") || title.contains(" - Discord")
}

/// Slash commands use Enter to pick options, so they are sent untouched.
fn untouched(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

fn press(enigo: &mut Enigo, key: Key) {
    let _ = enigo.key(key, Direction::Click);
}

fn translate_and_send(app: &AppHandle) -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default()).map_err(|e| e.to_string())?;
    // Ctrl+A, Ctrl+C: the whole message box. Nothing copied means an empty box: Enter does what it always does.
    let Some(text) = copy(app, &mut enigo, true) else {
        press(&mut enigo, Key::Return);
        return Ok(());
    };
    let send_as_is = |enigo: &mut Enigo| {
        press(enigo, Key::RightArrow); // drop the selection, keep the text
        press(enigo, Key::Return);
    };
    let retry = FAILED.lock().unwrap_or_else(|e| e.into_inner()).take();
    if untouched(&text) || retry.as_deref() == Some(text.as_str()) {
        send_as_is(&mut enigo);
        return Ok(());
    }

    // Always into the target language: unlike the hotkey, a message already in it is sent as it is,
    // never turned back into the source language.
    let mut s = settings::current();
    s.source_lang = "auto".to_string();
    if !s.discord_send_lang.is_empty() {
        s.target_lang = s.discord_send_lang.clone(); // Discord's own language, set on the Discord page
    }
    match translate::translate(&text, &s) {
        Ok(result) if result.text.trim() != text.trim() => {
            app.clipboard()
                .write_text(&result.text)
                .map_err(|e| e.to_string())?;
            wait(60);
            press_ctrl(&mut enigo, 'v');
            wait(120); // let Discord take the pasted text before Enter
            press(&mut enigo, Key::Return);
            remember(app, text, result);
            Ok(())
        }
        Ok(_) => {
            send_as_is(&mut enigo);
            Ok(())
        }
        Err(e) => {
            press(&mut enigo, Key::RightArrow);
            *FAILED.lock().unwrap_or_else(|e| e.into_inner()) = Some(text);
            Err(msg(Msg::NotSent, &[&e]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_discord() {
        assert!(is_discord(
            "Discord",
            r"C:\Users\a\AppData\Local\Discord\app-1.0.9\Discord.exe"
        ));
        assert!(is_discord(
            "#genel | Sunucu - Discord",
            r"C:\x\DiscordPTB.exe"
        ));
        assert!(is_discord(
            "#general | Server - Discord - Google Chrome",
            r"C:\x\chrome.exe"
        ));
        assert!(!is_discord("Notepad", r"C:\Windows\notepad.exe"));
        assert!(untouched("  /giphy cat"));
        assert!(!untouched("hello /world"));
    }
}
