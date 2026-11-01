//! The live user count on sleax.dev and in Settings (stats.sleax.dev). While the app runs, and the
//! "count me" setting is on, it sends a random id made once per install every five minutes: no name,
//! no system details, nothing typed. The service keeps ids in memory for eleven minutes and only counts them.

use crate::settings;
use reqwest::blocking::Client;
use serde_json::Value;
use std::hash::{BuildHasher, RandomState};
use std::sync::LazyLock;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const API: &str = "https://stats.sleax.dev/v1";
const APP: &str = "quicktranslate";
const EVERY: Duration = Duration::from_secs(5 * 60);

/// Its own client: short timeouts, and nothing shared with translation.
static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default()
});

pub fn start(app: &AppHandle) {
    let Some(id) = install_id(app) else { return };
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(15)); // let the network come up after login
        loop {
            if settings::current().usage_stats {
                let _ = CLIENT
                    .post(format!("{API}/ping"))
                    .json(&serde_json::json!({ "app": APP, "id": id }))
                    .send();
            }
            std::thread::sleep(EVERY);
        }
    });
}

/// A random id for this install, made on first use: 32 hex digits from the OS-seeded hasher keys.
fn install_id(app: &AppHandle) -> Option<String> {
    let path = app.path().app_data_dir().ok()?.join("install-id");
    if let Ok(id) = std::fs::read_to_string(&path)
        && id.trim().len() == 32
    {
        return Some(id.trim().to_string());
    }
    let id = format!(
        "{:016x}{:016x}",
        RandomState::new().hash_one(0u8),
        RandomState::new().hash_one(1u8)
    );
    let _ = std::fs::create_dir_all(path.parent()?);
    std::fs::write(&path, &id).ok()?;
    Some(id)
}

/// This app's numbers: live, downloads, stars, version.
#[tauri::command]
pub async fn app_stats() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let all: Value = CLIENT
            .get(format!("{API}/stats"))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json())
            .map_err(|e| e.to_string())?;
        Ok(all["apps"][APP].clone())
    })
    .await
    .map_err(|e| e.to_string())?
}
