use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)] // a field missing from an older settings file gets its default instead of resetting everything
pub struct Settings {
    pub source_lang: String,
    pub target_lang: String,
    pub auto_start: bool,
    pub hotkey: String,
    pub translation_type: String, // "formal" | "casual"
    pub api_provider: String,     // "mymemory" | "deepL" | "openrouter"
    pub api_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source_lang: "en".to_string(),
            target_lang: "tr".to_string(),
            auto_start: false,
            hotkey: "Ctrl+Shift+T".to_string(),
            translation_type: "casual".to_string(),
            api_provider: "mymemory".to_string(),
            api_key: String::new(),
        }
    }
}

static SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| RwLock::new(load_settings_from_file()));

/// A copy of the current settings.
pub fn current() -> Settings {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Replaces the settings and saves them to disk.
pub fn replace(new_settings: Settings) -> Result<(), String> {
    save_settings_to_file(&new_settings)?;
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = new_settings;
    Ok(())
}

fn get_settings_path() -> std::path::PathBuf {
    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(app_data).join("QuickTranslate").join("settings.json")
}

fn load_settings_from_file() -> Settings {
    std::fs::read_to_string(get_settings_path())
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_settings_to_file(settings: &Settings) -> Result<(), String> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}
