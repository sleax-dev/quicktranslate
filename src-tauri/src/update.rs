//! Updates from GitHub Releases: latest.json lists the newest version and its signed installers.
//! With automatic updates on, a found update is installed right away while the window is hidden
//! (the app restarts by itself); with the window open it waits for the button in Settings.

use crate::i18n::{Msg, msg};
use crate::{notify, settings};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// Checks shortly after startup, then every six hours.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(20)); // let the network come up after login
        loop {
            if settings::current().auto_update {
                tauri::async_runtime::block_on(auto(&app));
            }
            std::thread::sleep(EVERY);
        }
    });
}

async fn auto(app: &AppHandle) {
    let update = match app.updater().map_err(|e| e.to_string()) {
        Ok(updater) => updater.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e),
    };
    let update = match update {
        Ok(Some(update)) => update,
        Ok(None) => return,
        Err(e) => return log::warn!("update check: {e}"),
    };
    let visible = app
        .get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    if visible {
        let _ = app.emit("update-available", &update.version);
        return;
    }
    notify(app, &msg(Msg::Updating, &[&update.version]));
    match update.download_and_install(|_, _| {}, || {}).await {
        Ok(()) => app.restart(),
        Err(e) => log::warn!("update install: {e}"),
    }
}

/// The newer version on GitHub, or None when this one is the latest.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await;
    Ok(update.map_err(|e| e.to_string())?.map(|u| u.version))
}

/// Downloads, installs and restarts into the newer version.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await;
    let Some(update) = update.map_err(|e| e.to_string())? else {
        return Ok(());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}
