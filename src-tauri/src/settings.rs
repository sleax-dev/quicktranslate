use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

/// The services a user can pick. MyMemory is not one of them: it only steps in when Google fails.
pub const PROVIDERS: &[&str] = &["google", "deepL", "openrouter"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)] // a field missing from an older settings file gets its default instead of resetting everything
pub struct Settings {
    pub source_lang: String, // "auto" or a code from translate::LANGUAGES
    pub target_lang: String,
    pub auto_start: bool,
    pub hotkey: String,
    pub translation_type: String, // "formal" | "casual"; DeepL and OpenRouter only
    pub api_provider: String,     // one of PROVIDERS
    pub deepl_key: String,
    pub openrouter_key: String,
    /// Windows: Enter in Discord translates the message into the target language before sending it.
    pub discord_send: bool,
    /// Windows: foreign messages in Discord get their translation drawn over them.
    pub discord_read: bool,
    pub auto_update: bool,
    /// The single key of versions before 1.2, used by whichever service was selected. Read, never written.
    #[serde(skip_serializing)]
    api_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source_lang: "auto".to_string(),
            target_lang: "en".to_string(),
            auto_start: false,
            hotkey: "Ctrl+Shift+T".to_string(),
            translation_type: "casual".to_string(),
            api_provider: "google".to_string(),
            deepl_key: String::new(),
            openrouter_key: String::new(),
            discord_send: false,
            discord_read: false,
            auto_update: true,
            api_key: String::new(),
        }
    }
}

impl Settings {
    /// Moves the old shared key to its service, and replaces a service that no longer exists
    /// (MyMemory, the old default) with Google.
    fn upgrade(mut self) -> Self {
        let old_key = std::mem::take(&mut self.api_key);
        match self.api_provider.as_str() {
            "deepL" if self.deepl_key.is_empty() => self.deepl_key = old_key,
            "openrouter" if self.openrouter_key.is_empty() => self.openrouter_key = old_key,
            _ => {}
        }
        if !PROVIDERS.contains(&self.api_provider.as_str()) {
            self.api_provider = "google".to_string();
        }
        self
    }
}

static SETTINGS: LazyLock<RwLock<Settings>> =
    LazyLock::new(|| RwLock::new(load_settings_from_file()));

/// A copy of the current settings.
pub fn current() -> Settings {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Replaces the settings and saves them to disk.
pub fn replace(new_settings: Settings) -> Result<(), String> {
    let new_settings = new_settings.upgrade();
    save_settings_to_file(&new_settings)?;
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = new_settings;
    Ok(())
}

/// %APPDATA%\QuickTranslate on Windows, ~/Library/Application Support/QuickTranslate on macOS,
/// $XDG_CONFIG_HOME/QuickTranslate (~/.config) on Linux.
fn get_settings_path() -> std::path::PathBuf {
    let var = |name| std::env::var_os(name).filter(|v| !v.is_empty());
    let home = || std::path::PathBuf::from(var("HOME").unwrap_or_else(|| ".".into()));
    let dir = if cfg!(windows) {
        std::path::PathBuf::from(var("APPDATA").unwrap_or_else(|| ".".into()))
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else {
        var("XDG_CONFIG_HOME").map_or_else(|| home().join(".config"), Into::into)
    };
    dir.join("QuickTranslate").join("settings.json")
}

fn load_settings_from_file() -> Settings {
    std::fs::read_to_string(get_settings_path())
        .ok()
        .and_then(|content| serde_json::from_str::<Settings>(&content).ok())
        .unwrap_or_default()
        .upgrade()
}

fn save_settings_to_file(settings: &Settings) -> Result<(), String> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_files_upgrade() {
        let old: Settings = serde_json::from_str(
            r#"{"source_lang":"en","target_lang":"tr","api_provider":"deepL","api_key":"k:fx"}"#,
        )
        .unwrap();
        let s = old.upgrade();
        assert_eq!(s.deepl_key, "k:fx");
        assert!(!serde_json::to_string(&s).unwrap().contains("api_key"));

        let mymemory: Settings = serde_json::from_str(r#"{"api_provider":"mymemory"}"#).unwrap();
        assert_eq!(mymemory.upgrade().api_provider, "google");
    }
}
