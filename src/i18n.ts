import { ref } from 'vue';

const en = {
  'nav.translate': 'Translate',
  'nav.history': 'History',
  'nav.service': 'Service',
  'nav.settings': 'Settings',
  'nav.hotkey': 'Hotkey',
  'titlebar.minimize': 'Minimize',
  'titlebar.close': 'Close to tray',

  'translate.title': 'Translate',
  'translate.lead': 'Type here, or select text in any app and press',
  'translate.from': 'From',
  'translate.to': 'To',
  'translate.auto': 'Detect language',
  'translate.swap': 'Swap languages',
  'translate.placeholder': 'Type or paste text',
  'translate.output': 'Translation',
  'translate.detected': 'Detected: {lang}',
  'translate.reversed': 'Already in {lang}, so translated into {other}',
  'translate.copy': 'Copy',
  'translate.copied': 'Copied',
  'translate.clear': 'Clear',
  'translate.via': 'via {provider}',
  'translate.working': 'Translating…',
  'translate.chars': '{n} characters',

  'history.title': 'History',
  'history.lead': 'The last 50 hotkey translations. Kept in memory only, gone when the app closes.',
  'history.empty': 'Nothing yet. Select text in any app and press',
  'history.clear': 'Clear',
  'history.copyOriginal': 'Copy original',
  'history.copyTranslation': 'Copy translation',

  'service.title': 'Service',
  'service.lead': 'Who does the translating. Google works out of the box.',
  'service.google': 'Google Translate',
  'service.google.desc': 'Free, no key needed, every language. If it is unavailable, MyMemory steps in automatically.',
  'service.deepL': 'DeepL',
  'service.deepL.desc': 'The most natural translations. Needs an API key; the free plan covers 500,000 characters a month.',
  'service.openrouter': 'OpenRouter',
  'service.openrouter.desc': 'An AI model translates, with tone and context. Needs an API key and credit.',
  'service.free': 'Free',
  'service.recommended': 'Recommended',
  'service.key': 'API key',
  'service.keyMissing': 'Enter an API key to use {provider}',
  'service.getKey': 'Get a key',
  'service.show': 'Show',
  'service.hide': 'Hide',
  'service.style': 'Style',
  'service.style.desc': 'How the translation should sound.',
  'service.casual': 'Casual',
  'service.formal': 'Formal',
  'service.test': 'Test',
  'service.testing': 'Testing…',
  'service.testOk': 'Works: "{text}" ({ms} ms)',

  'settings.title': 'Settings',
  'settings.hotkey': 'Hotkey',
  'settings.hotkey.desc': 'Translates the selected text, or the whole text box when nothing is selected, and puts the translation in its place.',
  'settings.record': 'Press the new keys…',
  'settings.recordHint': 'Click, then press the keys',
  'settings.needModifier': 'Use Ctrl, Alt or Shift with a key',
  'settings.languages': 'Languages',
  'settings.languages.desc': 'From is your language. Text already in the target language is translated back into it, so one hotkey works both ways.',
  'settings.autostart': 'Start with Windows',
  'settings.autostart.desc': 'Runs in the tray, ready for the hotkey.',
  'settings.saved': 'Saved',
  'settings.version': 'Version {v}',
  'settings.sameLang': 'From and To are the same language',
} as const;

type Key = keyof typeof en;

const tr: Record<Key, string> = {
  'nav.translate': 'Çevir',
  'nav.history': 'Geçmiş',
  'nav.service': 'Servis',
  'nav.settings': 'Ayarlar',
  'nav.hotkey': 'Kısayol',
  'titlebar.minimize': 'Simge durumuna küçült',
  'titlebar.close': 'Tepsiye gizle',

  'translate.title': 'Çevir',
  'translate.lead': 'Buraya yazın ya da herhangi bir uygulamada metni seçip kısayola basın:',
  'translate.from': 'Kaynak',
  'translate.to': 'Hedef',
  'translate.auto': 'Dili algıla',
  'translate.swap': 'Dilleri değiştir',
  'translate.placeholder': 'Metin yazın ya da yapıştırın',
  'translate.output': 'Çeviri',
  'translate.detected': 'Algılanan: {lang}',
  'translate.reversed': 'Metin zaten {lang}, bu yüzden {other} diline çevrildi',
  'translate.copy': 'Kopyala',
  'translate.copied': 'Kopyalandı',
  'translate.clear': 'Temizle',
  'translate.via': '{provider} ile',
  'translate.working': 'Çevriliyor…',
  'translate.chars': '{n} karakter',

  'history.title': 'Geçmiş',
  'history.lead': 'Kısayolla yapılan son 50 çeviri. Yalnızca bellekte tutulur, uygulama kapanınca silinir.',
  'history.empty': 'Henüz bir şey yok. Herhangi bir uygulamada metni seçip kısayola basın:',
  'history.clear': 'Temizle',
  'history.copyOriginal': 'Asıl metni kopyala',
  'history.copyTranslation': 'Çeviriyi kopyala',

  'service.title': 'Servis',
  'service.lead': 'Çeviriyi kimin yapacağı. Google hiçbir ayar gerektirmez.',
  'service.google': 'Google Çeviri',
  'service.google.desc': 'Ücretsiz, anahtar gerekmez, tüm diller. Ulaşılamazsa otomatik olarak MyMemory devreye girer.',
  'service.deepL': 'DeepL',
  'service.deepL.desc': 'En doğal çeviriler. API anahtarı gerekir; ücretsiz plan ayda 500.000 karakter.',
  'service.openrouter': 'OpenRouter',
  'service.openrouter.desc': 'Bir yapay zekâ modeli üslubu ve bağlamı koruyarak çevirir. API anahtarı ve kredi gerekir.',
  'service.free': 'Ücretsiz',
  'service.recommended': 'Önerilen',
  'service.key': 'API anahtarı',
  'service.keyMissing': '{provider} kullanmak için API anahtarı girin',
  'service.getKey': 'Anahtar al',
  'service.show': 'Göster',
  'service.hide': 'Gizle',
  'service.style': 'Üslup',
  'service.style.desc': 'Çevirinin nasıl bir dille yazılacağı.',
  'service.casual': 'Günlük',
  'service.formal': 'Resmî',
  'service.test': 'Dene',
  'service.testing': 'Deneniyor…',
  'service.testOk': 'Çalışıyor: "{text}" ({ms} ms)',

  'settings.title': 'Ayarlar',
  'settings.hotkey': 'Kısayol',
  'settings.hotkey.desc': 'Seçili metni, hiçbir şey seçili değilse metin kutusunun tamamını çevirir ve çeviriyi yerine yazar.',
  'settings.record': 'Yeni tuşlara basın…',
  'settings.recordHint': 'Tıklayın, sonra tuşlara basın',
  'settings.needModifier': 'Bir tuşla birlikte Ctrl, Alt ya da Shift kullanın',
  'settings.languages': 'Diller',
  'settings.languages.desc': 'Kaynak sizin dilinizdir. Zaten hedef dilde olan metin kaynak dile geri çevrilir; tek kısayol iki yönde çalışır.',
  'settings.autostart': 'Windows ile başlat',
  'settings.autostart.desc': 'Tepside çalışır, kısayola hazır bekler.',
  'settings.saved': 'Kaydedildi',
  'settings.version': 'Sürüm {v}',
  'settings.sameLang': 'Kaynak ve hedef aynı dil',
};

/** The UI language, set from the Rust side at startup so it matches the tray and the errors. */
export const uiLang = ref<'tr' | 'en'>(navigator.language.startsWith('tr') ? 'tr' : 'en');

/** The platform the UI runs on: a few texts and the key symbols differ per platform. */
export const os: 'windows' | 'macos' | 'linux' = /Mac/.test(navigator.userAgent)
  ? 'macos'
  : /Windows/.test(navigator.userAgent) ? 'windows' : 'linux';

type Texts = Partial<Record<Key, string>>;
const platformTexts: Partial<Record<typeof os, { en: Texts; tr: Texts }>> = {
  macos: {
    en: {
      'settings.needModifier': 'Use ⌘, ⌃ or ⌥ with a key',
      'settings.autostart': 'Open at login',
      'settings.autostart.desc': 'Runs in the menu bar, ready for the hotkey.',
    },
    tr: {
      'settings.needModifier': 'Bir tuşla birlikte ⌘, ⌃ ya da ⌥ kullanın',
      'settings.autostart': 'Oturum açılınca başlat',
      'settings.autostart.desc': 'Menü çubuğunda çalışır, kısayola hazır bekler.',
    },
  },
  linux: {
    en: { 'settings.autostart': 'Start at login' },
    tr: { 'settings.autostart': 'Oturum açılınca başlat' },
  },
};

export function t(key: Key, params: Record<string, string | number> = {}): string {
  const text: string = platformTexts[os]?.[uiLang.value]?.[key] ?? (uiLang.value === 'tr' ? tr : en)[key];
  return text.replace(/\{(\w+)\}/g, (_, name) => String(params[name] ?? `{${name}}`));
}

const names = new Map<string, Intl.DisplayNames>();

/** A language's name in the UI language ("tr" → "Türkçe"), with the English name as a fallback. */
export function langName(code: string, fallback = code): string {
  let dn = names.get(uiLang.value);
  if (!dn) names.set(uiLang.value, (dn = new Intl.DisplayNames([uiLang.value], { type: 'language' })));
  const name = dn.of(code === 'no' ? 'nb' : code);
  return name && name !== code ? name.charAt(0).toLocaleUpperCase(uiLang.value) + name.slice(1) : fallback;
}
