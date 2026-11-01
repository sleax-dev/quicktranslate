use crate::settings;
use std::sync::LazyLock;

/// The interface languages, as in Zanrei.
pub const UI_LANGUAGES: &[&str] = &["en", "tr", "es", "de", "fr", "pt", "ru", "zh", "ja", "ar"];

/// The system's language, two letters ("tr"), or "en" when it is not one of UI_LANGUAGES.
static SYSTEM: LazyLock<&'static str> = LazyLock::new(|| {
    let tag = system_tag().to_lowercase();
    let base = tag.split(['-', '_', '.']).next().unwrap_or("");
    UI_LANGUAGES
        .iter()
        .find(|l| **l == base)
        .copied()
        .unwrap_or("en")
});

#[cfg(target_os = "windows")]
fn system_tag() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..(len.max(1) - 1) as usize])
}

#[cfg(target_os = "macos")]
fn system_tag() -> String {
    // ( "tr-TR", "en-US" ): the first entry is the UI language.
    std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim_start_matches(|c: char| c == '(' || c == '"' || c.is_whitespace())
                .to_string()
        })
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn system_tag() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .unwrap_or_default()
}

pub fn system() -> &'static str {
    *SYSTEM
}

/// The interface language: the one picked in Settings, else the system's.
pub fn lang() -> &'static str {
    let picked = settings::current().ui_lang;
    UI_LANGUAGES
        .iter()
        .find(|l| **l == picked)
        .copied()
        .unwrap_or(*SYSTEM)
}

/// Picks the Turkish or the English text: notifications and errors come in these two, English for
/// every other interface language.
pub fn t(tr: &str, en: &str) -> String {
    if lang() == "tr" { tr } else { en }.to_string()
}

/// The tray menu: (open, settings, Discord report, quit) in every interface language.
pub fn tray_texts() -> [&'static str; 4] {
    match lang() {
        "tr" => ["Aç", "Ayarlar", "Discord tanı raporu", "Çıkış"],
        "es" => ["Abrir", "Ajustes", "Informe de Discord", "Salir"],
        "de" => ["Öffnen", "Einstellungen", "Discord-Bericht", "Beenden"],
        "fr" => ["Ouvrir", "Paramètres", "Rapport Discord", "Quitter"],
        "pt" => ["Abrir", "Configurações", "Relatório do Discord", "Sair"],
        "ru" => ["Открыть", "Настройки", "Отчёт Discord", "Выход"],
        "zh" => ["打开", "设置", "Discord 诊断报告", "退出"],
        "ja" => ["開く", "設定", "Discord 診断レポート", "終了"],
        "ar" => ["فتح", "الإعدادات", "تقرير Discord", "خروج"],
        _ => ["Open", "Settings", "Discord report", "Quit"],
    }
}
