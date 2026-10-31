mod i18n;
mod settings;
mod translate;

use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use i18n::Translations;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_log::{Target, TargetKind};

#[tauri::command]
fn get_settings() -> settings::Settings {
    settings::current()
}

#[tauri::command]
fn update_settings(new_settings: settings::Settings) -> Result<(), String> {
    settings::replace(new_settings)
}

#[tauri::command]
fn register_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    bind_hotkey(&app, &hotkey)
}

/// Makes `hotkey` the only global shortcut: the previous one stops working.
fn bind_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut: Shortcut = hotkey.parse().map_err(|e| format!("{e:?}"))?;
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
                }
            });
        })
        .map_err(|e| e.to_string())
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

fn press_ctrl(key: char) {
    if let Ok(mut enigo) = Enigo::new(&EnigoSettings::default()) {
        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode(key), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
    }
}

fn wait(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Copies the text of the focused field, translates it and pastes the result back.
/// Blocking: runs on its own thread.
fn translate_and_replace(app: &AppHandle) -> Result<(), String> {
    let _busy = Busy::acquire().ok_or("Çeviri zaten devam ediyor")?;

    wait(100);
    press_ctrl('a');
    wait(100);
    press_ctrl('c');
    wait(200);

    let text = app.clipboard().read_text().map_err(|e| format!("Clipboard okunamadı: {e}"))?;
    if text.trim().is_empty() {
        return Err("Çevrilecek metin bulunamadı".to_string());
    }

    let s = settings::current();
    let translated = translate::translate_text(&text, &s.source_lang, &s.target_lang, &s.api_provider, &s.api_key, &s.translation_type)
        .map_err(|e| format!("Çeviri hatası: {e}"))?;

    app.clipboard().write_text(&translated).map_err(|e| format!("Clipboard yazılamadı: {e}"))?;
    wait(100);
    press_ctrl('a');
    wait(100);
    press_ctrl('v');
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &AppHandle, i18n: &Translations) -> Result<(), Box<dyn std::error::Error>> {
    let show_i = MenuItem::with_id(app, "show", i18n.t("show"), true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", i18n.t("settings"), true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", i18n.t("quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &settings_i, &quit_i])?;
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip(i18n.t("tooltip"))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "settings" => {
                show_main_window(app);
                let _ = app.emit("open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                match app.get_webview_window("main") {
                    Some(window) if window.is_visible().unwrap_or(false) => {
                        let _ = window.hide();
                    }
                    _ => show_main_window(app),
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([Target::new(TargetKind::Stdout), Target::new(TargetKind::LogDir { file_name: None })])
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![get_settings, update_settings, register_hotkey])
        .setup(|app| {
            let handle = app.handle().clone();
            setup_tray(&handle, &Translations::new())?;
            bind_hotkey(&handle, &settings::current().hotkey)?;

            // Started by autostart with --minimized: stay in the tray.
            if !std::env::args().any(|a| a == "--minimized") {
                show_main_window(&handle);
            }
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
