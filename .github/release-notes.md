## Downloads

| System | File |
|---|---|
| **Windows** 10 / 11, 64-bit | `QuickTranslate_<version>_x64-setup.exe` |
| **macOS** 10.15+, Universal (Apple Silicon + Intel) | `QuickTranslate_<version>_universal.dmg` |
| **Linux** Ubuntu / Debian / Mint | `QuickTranslate_<version>_amd64.deb` |
| **Linux** Fedora / openSUSE / RHEL | `QuickTranslate-<version>-1.x86_64.rpm` |
| **Linux** any distribution | `QuickTranslate_<version>_amd64.AppImage` |

Default hotkey: `Ctrl+Shift+T` (macOS: `⌃⇧T`).

### Windows
- Installs for the current user, no admin rights needed. Settings from earlier versions are kept.
- Needs the [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/), which Windows 10/11 already have.
- The installer is not code-signed, so SmartScreen may warn on first run: **More info → Run anyway**.

### macOS
- Open the dmg and drag QuickTranslate into Applications.
- **First launch:** the app is not notarized, so macOS blocks it once. Open it, then **System Settings → Privacy & Security → Open Anyway**, or run `xattr -cr /Applications/QuickTranslate.app` in Terminal.
- **Accessibility permission:** QuickTranslate copies and pastes the selected text for you, which macOS only allows with permission: **System Settings → Privacy & Security → Accessibility → turn on QuickTranslate**, then restart the app.
- Lives in the menu bar; the Dock icon brings the window back.

### Linux
- `.deb`: `sudo apt install ./QuickTranslate_<version>_amd64.deb`
- `.rpm`: `sudo dnf install ./QuickTranslate-<version>-1.x86_64.rpm`
- `.AppImage`: `chmod +x QuickTranslate_*.AppImage && ./QuickTranslate_*.AppImage`
- **X11 session needed for the hotkey.** Wayland does not let apps register global hotkeys or send keystrokes; pick "Ubuntu on Xorg" / "Plasma (X11)" at login. The translate window itself works on Wayland.
- The tray icon needs a system tray: built into KDE, Cinnamon and Xfce; on GNOME install the *AppIndicator* extension (preinstalled on Ubuntu).
