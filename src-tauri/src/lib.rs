#[cfg(windows)]
mod discord;
#[cfg(any(windows, test))]
mod discord_parse;
#[cfg(windows)]
mod discord_read;
mod i18n;
mod settings;
mod translate;
mod update;

use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use i18n::{Msg, msg};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, Emitter, Manager, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::{Effect, EffectsBuilder},
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_notification::NotificationExt;
use translate::Translation;

const TRAY_ID: &str = "main";

/// Set at startup: Mica only exists on Windows 11; on Windows 10, macOS and Linux the UI uses a solid background.
static MICA: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn get_settings() -> settings::Settings {
    settings::current()
}

/// Saves the settings; a changed hotkey is registered first, so a hotkey Windows refuses is not saved.
#[tauri::command]
fn update_settings(app: AppHandle, new_settings: settings::Settings) -> Result<(), String> {
    let old = settings::current();
    if new_settings.hotkey != old.hotkey {
        bind_hotkey(&app, &new_settings.hotkey)?;
    }
    let lang_changed = new_settings.ui_lang != old.ui_lang;
    if let Err(e) = settings::replace(new_settings) {
        let _ = bind_hotkey(&app, &old.hotkey);
        return Err(e);
    }
    set_tray_tooltip(&app);
    if lang_changed && let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), tray_menu(&app)) {
        let _ = tray.set_menu(Some(menu));
    }
    #[cfg(windows)]
    discord::sync();
    Ok(())
}

#[tauri::command]
fn get_languages() -> &'static [(&'static str, &'static str)] {
    translate::LANGUAGES
}

/// The interface language ("tr", "de", …): the UI speaks the same language as the tray menu.
#[tauri::command]
fn ui_language() -> &'static str {
    i18n::lang()
}

/// The system's language, for the "System (…)" choice.
#[tauri::command]
fn system_language() -> &'static str {
    i18n::system()
}

/// Windows: writes the Discord report and shows it in its folder.
#[tauri::command]
fn discord_report(_app: AppHandle) {
    #[cfg(windows)]
    discord_read::diagnose(&_app);
}

#[tauri::command]
fn get_background() -> String {
    settings::background()
}

#[tauri::command]
fn set_background(data_url: String) -> Result<(), String> {
    settings::set_background(&data_url)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn mica_enabled() -> bool {
    MICA.load(Ordering::Relaxed)
}

/// Translates with the saved settings, off the main thread so the window stays responsive.
#[tauri::command]
async fn translate_text(text: String) -> Result<Translation, String> {
    tauri::async_runtime::spawn_blocking(move || translate::translate(&text, &settings::current()))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Clone, Serialize)]
struct HistoryEntry {
    original: String,
    #[serde(flatten)]
    translation: Translation,
    /// Unix time in milliseconds.
    at: u64,
}

/// The latest hotkey translations, newest first. Kept in memory only: nothing is written to disk.
static HISTORY: Mutex<VecDeque<HistoryEntry>> = Mutex::new(VecDeque::new());
const HISTORY_LEN: usize = 50;

#[tauri::command]
fn get_history() -> Vec<HistoryEntry> {
    HISTORY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .cloned()
        .collect()
}

#[tauri::command]
fn clear_history() {
    HISTORY.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

fn remember(app: &AppHandle, original: String, translation: Translation) {
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let entry = HistoryEntry {
        original,
        translation,
        at,
    };
    let mut history = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
    history.push_front(entry.clone());
    history.truncate(HISTORY_LEN);
    let _ = app.emit("history", entry);
}

/// Makes `hotkey` the only global shortcut: the previous one stops working.
fn bind_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut: Shortcut = hotkey
        .parse()
        .map_err(|_| msg(Msg::BadHotkey, &[&hotkey]))?;
    let shortcuts = app.global_shortcut();
    shortcuts.unregister_all().map_err(|e| e.to_string())?;
    shortcuts
        .on_shortcut(shortcut, |app, _shortcut, event| {
            // The handler also fires when the keys are released.
            if event.state != ShortcutState::Pressed {
                return;
            }
            let app = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = translate_and_replace(&app) {
                    log::warn!("{e}");
                    notify(&app, &e);
                }
            });
        })
        .map_err(|_| msg(Msg::HotkeyTaken, &[&hotkey]))
}

/// A system notification: the window is usually hidden when the hotkey is used.
fn notify(app: &AppHandle, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title("QuickTranslate")
        .body(body)
        .show();
}

static TRANSLATING: AtomicBool = AtomicBool::new(false);

/// Holds the "translation running" flag and clears it when dropped, on every return path.
struct Busy;

impl Busy {
    fn acquire() -> Option<Self> {
        TRANSLATING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| Busy)
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        TRANSLATING.store(false, Ordering::SeqCst);
    }
}

fn wait(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// The modifier of the copy/paste/select-all shortcuts: Cmd on macOS, Ctrl elsewhere.
const SHORTCUT_MOD: Key = if cfg!(target_os = "macos") {
    Key::Meta
} else {
    Key::Control
};

/// Sends Ctrl+`key` (Cmd+`key` on macOS).
fn press_ctrl(enigo: &mut Enigo, key: char) {
    let _ = enigo.key(SHORTCUT_MOD, Direction::Press);
    let _ = enigo.key(Key::Unicode(key), Direction::Click);
    let _ = enigo.key(SHORTCUT_MOD, Direction::Release);
}

/// Copies with Ctrl+C (after Ctrl+A when `select_all`) and returns the text, or None when nothing was copied.
/// The clipboard is emptied first, so an old clipboard is never mistaken for the copied text.
fn copy(app: &AppHandle, enigo: &mut Enigo, select_all: bool) -> Option<String> {
    let _ = app.clipboard().clear();
    if select_all {
        press_ctrl(enigo, 'a');
        wait(60);
    }
    press_ctrl(enigo, 'c');
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(600) {
        wait(30);
        if let Ok(text) = app.clipboard().read_text()
            && !text.trim().is_empty()
        {
            return Some(text);
        }
    }
    None
}

/// Copies the selected text (or, with nothing selected, the whole field), translates it and pastes the
/// result over it. Blocking: runs on its own thread.
fn translate_and_replace(app: &AppHandle) -> Result<(), String> {
    let _busy = Busy::acquire().ok_or_else(|| msg(Msg::Busy, &[]))?;
    let mut enigo = Enigo::new(&EnigoSettings::default()).map_err(|e| e.to_string())?;

    // The hotkey's own keys are still held down: Shift would turn Ctrl+C into Ctrl+Shift+C
    // (which copies nothing in most apps), so let go of every modifier first. Ctrl is held meanwhile:
    // a lone Alt or Win release would open the app's menu bar or the Start menu.
    let _ = enigo.key(Key::Control, Direction::Press);
    for key in [Key::Shift, Key::Alt, Key::Meta] {
        let _ = enigo.key(key, Direction::Release);
    }
    let _ = enigo.key(Key::Control, Direction::Release);
    wait(50);

    let text = copy(app, &mut enigo, false)
        .or_else(|| copy(app, &mut enigo, true))
        .ok_or_else(|| msg(Msg::NoText, &[]))?;

    let result = translate::translate(&text, &settings::current())?;
    app.clipboard()
        .write_text(&result.text)
        .map_err(|e| e.to_string())?;
    wait(60);
    press_ctrl(&mut enigo, 'v'); // the copied text is still selected, so this replaces it
    remember(app, text, result);
    Ok(())
}

/// Shows the window, on `page` when given. The window exists only while it is open: closing it destroys
/// its WebView, which gives its memory (about 100 MB) back while the app waits in the tray.
fn show_main_window(app: &AppHandle, page: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        if let Some(page) = page {
            let _ = app.emit_to("main", "open-page", page);
        }
        return;
    }
    if let Err(e) = create_main_window(app, page) {
        log::warn!("window: {e}");
    }
}

/// Builds the window from tauri.conf.json. It starts hidden; the page shows it once it has drawn, so it
/// never flashes up empty.
fn create_main_window(app: &AppHandle, page: Option<&str>) -> tauri::Result<()> {
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == "main") else {
        return Ok(());
    };
    let mut config = config.clone();
    if let Some(page) = page {
        config.url = tauri::WebviewUrl::App(format!("index.html#{page}").into());
    }
    let window = tauri::WebviewWindowBuilder::from_config(app, &config)?.build()?;
    let mica = cfg!(windows)
        && window
            .set_effects(EffectsBuilder::new().effect(Effect::Mica).build())
            .is_ok();
    MICA.store(mica, Ordering::Relaxed);
    efficiency_mode(false);
    // The close button closes the window, not the app: it keeps running in the tray.
    window.on_window_event(|event| {
        if let WindowEvent::Destroyed = event {
            efficiency_mode(true);
        }
    });
    Ok(())
}

/// Windows "efficiency mode" (EcoQoS) while only the tray is left: the scheduler runs the app on
/// efficient cores at low clock speeds, which a hotkey press followed by a network request never notices.
fn efficiency_mode(on: bool) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcess, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
            ProcessPowerThrottling, SetProcessInformation,
        };
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if on {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };
        SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            &state as *const _ as *const _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
    }
    #[cfg(not(windows))]
    let _ = on;
}

fn set_tray_tooltip(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let hotkey = settings::current().hotkey;
        let _ = tray.set_tooltip(Some(format!("QuickTranslate · {hotkey}")));
    }
}

fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let [open, settings, report, quit] = i18n::tray_texts();
    let open_i = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    if cfg!(windows) {
        // A report of what Discord shows to screen readers, for when reading its messages breaks.
        let report_i = MenuItem::with_id(app, "discord-report", report, true, None::<&str>)?;
        Menu::with_items(app, &[&open_i, &settings_i, &report_i, &separator, &quit_i])
    } else {
        Menu::with_items(app, &[&open_i, &settings_i, &separator, &quit_i])
    }
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = tray_menu(app)?;
    // The macOS menu bar wants a monochrome template icon, which it tints for light and dark mode.
    let (icon, template) = if cfg!(target_os = "macos") {
        (
            include_bytes!("../icons/tray-template.png").as_slice(),
            true,
        )
    } else {
        (include_bytes!("../icons/32x32.png").as_slice(), false)
    };

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(icon)?)
        .icon_as_template(template)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app, None),
            "settings" => show_main_window(app, Some("settings")),
            #[cfg(windows)]
            "discord-report" => discord_read::diagnose(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                match app.get_webview_window("main") {
                    Some(window)
                        if window.is_visible().unwrap_or(false)
                            && !window.is_minimized().unwrap_or(false) =>
                    {
                        let _ = window.destroy();
                    }
                    _ => show_main_window(app, None),
                }
            }
        })
        .build(app)?;
    set_tray_tooltip(app);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch (Start menu, autostart) shows the running copy instead of starting another.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app, None)
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_log::Builder::new()
                // Warnings and notes only: debug output (every HTTP connection) would fill the log file.
                .level(log::LevelFilter::Info)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            get_languages,
            ui_language,
            mica_enabled,
            translate_text,
            get_history,
            clear_history,
            quit_app,
            get_background,
            set_background,
            system_language,
            discord_report,
            update::check_update,
            update::install_update
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            setup_tray(&handle)?;
            update::start(&handle);
            #[cfg(windows)]
            {
                discord::init(&handle);
                discord_read::start(&handle);
            }
            efficiency_mode(true); // until the window opens
            // A hotkey the system refuses is reported, and the app still starts so it can be changed.
            let hotkey = settings::current().hotkey;
            if let Err(e) = bind_hotkey(&handle, &hotkey) {
                log::warn!("{e}");
                notify(&handle, &e);
            }

            // Started by autostart with --minimized: stay in the tray.
            if !std::env::args().any(|a| a == "--minimized") {
                show_main_window(&handle, None);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Closing the window is not quitting: only the tray's Quit (app.exit, with a code) ends the app.
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = &event
            {
                api.prevent_exit();
            }
            // macOS: clicking the Dock icon while the window is closed brings it back.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(_app, None);
            }
        });
}
