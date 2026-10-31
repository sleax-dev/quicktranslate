use std::sync::LazyLock;

/// Whether the system is set to Turkish. The tray menu, notifications and errors follow it, and so does the UI.
pub static TURKISH: LazyLock<bool> = LazyLock::new(|| {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000; // no console window flashes up at startup
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", "(Get-Culture).Name"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            return String::from_utf8_lossy(&output.stdout).starts_with("tr");
        }
    }
    #[cfg(target_os = "macos")]
    if let Ok(output) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
    {
        // ( "tr-TR", "en-US" ): the first entry is the UI language.
        let out = String::from_utf8_lossy(&output.stdout);
        let first = out.trim_start_matches(|c: char| c == '(' || c == '"' || c.is_whitespace());
        return first.starts_with("tr");
    }
    #[cfg(target_os = "linux")]
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(var)
            && !value.is_empty()
        {
            return value.starts_with("tr");
        }
    }
    false
});

/// Picks the Turkish or the English text.
pub fn t(tr: &str, en: &str) -> String {
    if *TURKISH { tr } else { en }.to_string()
}
