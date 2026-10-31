use std::sync::LazyLock;

/// Whether Windows is set to Turkish. The tray menu, notifications and errors follow it, and so does the UI.
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
    false
});

/// Picks the Turkish or the English text.
pub fn t(tr: &str, en: &str) -> String {
    if *TURKISH { tr } else { en }.to_string()
}
