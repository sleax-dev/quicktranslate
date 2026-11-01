<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="QuickTranslate logo">
</p>

<h1 align="center">QuickTranslate</h1>

<p align="center">
  Select text in any app, press a hotkey, and it is translated and replaced in place.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-1.3.0-blue" alt="Version 1.3.0">
  <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows&logoColor=white" alt="Windows 10 and 11">
  <img src="https://img.shields.io/badge/macOS-10.15%2B%20Universal-000000?logo=apple&logoColor=white" alt="macOS 10.15+ Universal">
  <img src="https://img.shields.io/badge/Linux-deb%20%7C%20rpm%20%7C%20AppImage-FCC624?logo=linux&logoColor=black" alt="Linux deb, rpm, AppImage">
  <img src="https://img.shields.io/badge/license-MIT-purple" alt="MIT License">
</p>

## Download

Every release holds the Windows, macOS and Linux installers together. Get them from the [latest release](https://github.com/sleax-dev/quicktranslate/releases/latest):

| Platform | File |
|---|---|
| **Windows** 10 / 11, 64-bit | `QuickTranslate_1.3.0_x64-setup.exe` |
| **macOS** 10.15+, Apple Silicon and Intel | `QuickTranslate_1.3.0_universal.dmg` |
| **Linux** x86_64 | `.deb`, `.rpm` or `.AppImage` |

Versions up to 1.2.0 were released for Windows only.

## Features

- 🌐 **Runs in the tray / menu bar** - Always running in the background
- ⌨️ **Global Hotkey** - Works in any application
- 🔄 **Instant Translation & Replace** - Automatically translates and pastes selected text
- ↔️ **Both directions with one hotkey** - Text already in the target language is translated back into your language
- 🪟 **Translate window** - Type or paste text, see the translation as you type; recent hotkey translations in History
- 🌐 **30 languages** - Auto-detects the source language
- 🆓 **Works without a key** - Google Translate by default, MyMemory as an automatic fallback; DeepL and OpenRouter with your own key
- 📝 **Translation Style** - Formal or Casual (DeepL, OpenRouter)
- 💬 **Translate before sending in Discord** (Windows, off by default) - Press Enter as usual; the message goes out in the target language. Turn it on in Settings
- 🗨️ **Translated Discord messages** (Windows, beta, off by default) - Messages in other languages show their translation right over them, in Discord's own colours; hover to see the original. Reads what Discord shows to screen readers: no login, no client mod
- ⬆️ **Automatic updates** - New versions install themselves while the window is closed (from 1.4.0 on)

## Supported Languages

English, Turkish, German, French, Spanish, Italian, Portuguese, Russian, Ukrainian, Arabic, Japanese, Korean, Chinese, Hindi, Dutch, Polish, Czech, Romanian, Hungarian, Bulgarian, Greek, Swedish, Danish, Norwegian, Finnish, Hebrew, Thai, Vietnamese, Indonesian, Malay

## Installation

### Windows

1. Download `QuickTranslate_<version>_x64-setup.exe` from the [Windows release](https://github.com/sleax-dev/quicktranslate/releases/tag/v1.3.0-windows)
2. Run it and follow the wizard (no admin rights needed); the app starts when it finishes
3. The installer is not code-signed, so SmartScreen may warn on first run: **More info → Run anyway**

Needs the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/), which Windows 10/11 already have.

### macOS

1. Download `QuickTranslate_<version>_universal.dmg` from the [macOS release](https://github.com/sleax-dev/quicktranslate/releases/tag/v1.3.0-macos), open it and drag **QuickTranslate** into **Applications**
2. The app is not notarized, so macOS blocks the first launch: open it once, then **System Settings → Privacy & Security → Open Anyway**
   (or run `xattr -cr /Applications/QuickTranslate.app` once in Terminal)
3. Allow it to copy and paste for you: **System Settings → Privacy & Security → Accessibility → QuickTranslate on**, then restart the app

One universal build runs natively on Apple Silicon (M1–M4) and Intel Macs.

### Linux

Download from the [Linux release](https://github.com/sleax-dev/quicktranslate/releases/tag/v1.3.0-linux):

```sh
sudo apt install ./QuickTranslate_1.3.0_amd64.deb          # Ubuntu, Debian, Mint, Pop!_OS
sudo dnf install ./QuickTranslate-1.3.0-1.x86_64.rpm       # Fedora, openSUSE, RHEL
chmod +x QuickTranslate_1.3.0_amd64.AppImage && ./QuickTranslate_1.3.0_amd64.AppImage   # any distribution
```

- **The hotkey needs an X11 session.** Wayland does not let apps register global hotkeys or send keystrokes; pick "Ubuntu on Xorg" / "Plasma (X11)" at login. The translate window works everywhere.
- The tray icon needs a system tray: KDE, Cinnamon and Xfce have one; on GNOME use the *AppIndicator* extension (preinstalled on Ubuntu).

## Usage

### Translating Text

1. Select the text you want to translate in any application, or just click into a text box to translate all of it
2. Press `Ctrl+Shift+T` (`⌃⇧T` on macOS)
3. The translation replaces the text. If something goes wrong (no text, no connection), a notification says why

**Source** is the language you write in (or "Detect language"), **Target** the one to translate into.
With a source language set, text that is already in the target language goes the other way: with Turkish → English,
an English message is translated into Turkish.

### Settings

Click the tray icon (the menu bar icon on macOS) to open the window; changes are saved as you make them.

- **Service**: Google (free), DeepL or OpenRouter, their API keys, the translation style, and a Test button
- **Settings**: the hotkey (on macOS ⌘ combinations work too), the languages, and starting at login

Settings are stored in `%APPDATA%\QuickTranslate` on Windows, `~/Library/Application Support/QuickTranslate` on macOS and `~/.config/QuickTranslate` on Linux.

## API Services

### Google Translate (default, free)
- No API key. Uses the public endpoint behind Google's own translate widgets
- If it is unreachable or rate-limited, MyMemory's machine translation steps in (no key, about 5,000 characters a day)

### DeepL
- API key required (deepl.com/pro-api). Free and Pro keys both work
- Quality: Very Good
- Supports formal/casual style

### OpenRouter
- API key required (openrouter.ai/keys)
- Quality: Very Good (AI-powered translation)
- Natural and fluent translations

## System Requirements

| | Windows | macOS | Linux |
|---|---|---|---|
| OS | Windows 10 / 11, 64-bit | macOS 10.15 Catalina or later | x86_64, glibc 2.35+ (Ubuntu 22.04, Debian 12, Fedora 36 or newer) |
| CPU | x64 | Apple Silicon or Intel | x86_64 |
| Runtime | WebView2 (built in) | — | WebKitGTK 4.1, X11 session for the hotkey |
| Disk | ~15 MB | ~20 MB | ~20 MB |

## Building from source

Requires the current LTS version of [Node.js](https://nodejs.org), stable [Rust](https://rustup.rs) and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.
On Debian/Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev`.

```sh
npm install
npm run tauri dev     # run with hot reload
npm run tauri build   # installers in src-tauri/target/release/bundle
```

`npm run tauri build` makes the packages of the platform it runs on: the NSIS installer on Windows, `.app` and `.dmg` on macOS
(`-- --target universal-apple-darwin` for both architectures), `.deb`, `.rpm` and `.AppImage` on Linux.
Pushing a `v*` tag builds all three on GitHub Actions and publishes one release per platform.

## Contributing

1. Fork the repository
2. Create a new branch (`git checkout -b feature/new-feature`)
3. Commit your changes (`git commit -m 'Add new feature'`)
4. Push to the branch (`git push origin feature/new-feature`)
5. Open a Pull Request

## License

[MIT License](LICENSE)

---

**Note**: If you find a bug or have suggestions, please report them via [Issues](https://github.com/sleax-dev/quicktranslate/issues).
