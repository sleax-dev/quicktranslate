use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Turkish,
    English,
}

impl Language {
    pub fn from_system_locale() -> Self {
        // Get Windows locale
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000; // no console window flashes up at startup
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", "(Get-Culture).Name"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            if let Ok(output) = output {
                let locale = String::from_utf8_lossy(&output.stdout);
                if locale.starts_with("tr") {
                    return Language::Turkish;
                }
            }
        }
        Language::English
    }
}

pub struct Translations {
    pub lang: Language,
}

impl Translations {
    pub fn new() -> Self {
        Self {
            lang: Language::from_system_locale(),
        }
    }

    pub fn t(&self, key: &str) -> String {
        let tr = match self.lang {
            Language::Turkish => self.turkish(),
            Language::English => self.english(),
        };
        tr.get(key).map(|s| s.to_string()).unwrap_or_else(|| key.to_string())
    }

    fn turkish(&self) -> HashMap<&str, &str> {
        let mut m = HashMap::new();
        m.insert("show", "Arayüzü Aç");
        m.insert("settings", "Ayarlar");
        m.insert("quit", "Çıkış");
        m.insert("tooltip", "QuickTranslate");
        m.insert("save", "Kaydet");
        m.insert("saved", "Kaydedildi");
        m.insert("source", "Kaynak");
        m.insert("target", "Hedef");
        m.insert("type", "Çeviri Tarzı");
        m.insert("service", "Servis");
        m.insert("hotkey", "Kısayol");
        m.insert("autostart", "Otomatik başlat");
        m
    }

    fn english(&self) -> HashMap<&str, &str> {
        let mut m = HashMap::new();
        m.insert("show", "Open Interface");
        m.insert("settings", "Settings");
        m.insert("quit", "Quit");
        m.insert("tooltip", "QuickTranslate");
        m.insert("save", "Save");
        m.insert("saved", "Saved");
        m.insert("source", "Source");
        m.insert("target", "Target");
        m.insert("type", "Translation Type");
        m.insert("service", "Service");
        m.insert("hotkey", "Hotkey");
        m.insert("autostart", "Auto start");
        m
    }
}