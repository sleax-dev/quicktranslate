# QuickTranslate

Instant translation app - Translate selected text instantly with a global hotkey and replace it.

![QuickTranslate](https://img.shields.io/badge/Version-1.2.0-blue)
![Platform](https://img.shields.io/badge/Platform-Windows-green)
![License](https://img.shields.io/badge/License-MIT-purple)

## Features

- 🌐 **Runs in System Tray** - Always running in the background
- ⌨️ **Global Hotkey** - Works in any application
- 🔄 **Instant Translation & Replace** - Automatically translates and pastes selected text
- ↔️ **Both directions with one hotkey** - Text already in the target language is translated back into your language
- 🪟 **Translate window** - Type or paste text, see the translation as you type; recent hotkey translations in History
- 🌐 **30 languages** - Auto-detects the source language
- 🆓 **Works without a key** - Google Translate by default, MyMemory as an automatic fallback; DeepL and OpenRouter with your own key
- 📝 **Translation Style** - Formal or Casual (DeepL, OpenRouter)

## Supported Languages

English, Turkish, German, French, Spanish, Italian, Portuguese, Russian, Ukrainian, Arabic, Japanese, Korean, Chinese, Hindi, Dutch, Polish, Czech, Romanian, Hungarian, Bulgarian, Greek, Swedish, Danish, Norwegian, Finnish, Hebrew, Thai, Vietnamese, Indonesian, Malay

## Installation

### Requirements

- Windows 10/11
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually pre-installed)

### Steps

1. Download the latest version from [Releases](https://github.com/sleax-dev/quicktranslate/releases)
2. Run `QuickTranslate_<version>_x64-setup.exe`
3. Follow the installation wizard
4. The app will start automatically when installation is complete

## Usage

### Translating Text

1. Select the text you want to translate in any application, or just click into a text box to translate all of it
2. Press `Ctrl+Shift+T`
3. The translation replaces the text. If something goes wrong (no text, no connection), a Windows notification says why

**Source** is the language you write in (or "Detect language"), **Target** the one to translate into.
With a source language set, text that is already in the target language goes the other way: with Turkish → English,
an English message is translated into Turkish.

### Settings

Click the tray icon to open the window; changes are saved as you make them.

- **Service**: Google (free), DeepL or OpenRouter, their API keys, the translation style, and a Test button
- **Settings**: the hotkey, the languages, and starting with Windows

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

- Windows 10 or later
- 50MB disk space
- WebView2 Runtime

## Building from source

Requires the current LTS version of [Node.js](https://nodejs.org), stable [Rust](https://rustup.rs) and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows.

```sh
npm install
npm run tauri dev     # run with hot reload
npm run tauri build   # installer in src-tauri/target/release/bundle
```

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
