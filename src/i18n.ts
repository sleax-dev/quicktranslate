import { ref, watch } from 'vue';
import en, { type Key, type Locale } from './locales/en';
import tr from './locales/tr';
import es from './locales/es';
import de from './locales/de';
import fr from './locales/fr';
import pt from './locales/pt';
import ru from './locales/ru';
import zh from './locales/zh';
import ja from './locales/ja';
import ar from './locales/ar';

/** The interface languages, as in Zanrei, each named in itself. Adding one: a file in ./locales and a line here. */
export const LANGUAGES = {
  en: 'English',
  tr: 'Türkçe',
  es: 'Español',
  de: 'Deutsch',
  fr: 'Français',
  pt: 'Português',
  ru: 'Русский',
  zh: '中文',
  ja: '日本語',
  ar: 'العربية',
} as const;
export type Lang = keyof typeof LANGUAGES;

const locales: Record<Lang, Locale> = { en, tr, es, de, fr, pt, ru, zh, ja, ar };

/** The interface language, from the Rust side at startup so it matches the tray menu. */
export const uiLang = ref<Lang>('en');

// Arabic reads right to left: the whole layout mirrors (the CSS uses logical properties).
watch(uiLang, (lang) => {
  document.documentElement.lang = lang;
  document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
}, { immediate: true });

/** The platform the UI runs on: a few texts and the key symbols differ per platform. */
export const os: 'windows' | 'macos' | 'linux' = /Mac/.test(navigator.userAgent)
  ? 'macos'
  : /Windows/.test(navigator.userAgent) ? 'windows' : 'linux';

type BaseKey = Exclude<Key, `${string}@${string}`>;

/** The text for `key` in the interface language; a `key@macos` / `key@linux` text wins on that platform. */
export function t(key: BaseKey, params: Record<string, string | number> = {}): string {
  const locale = locales[uiLang.value];
  const text = locale[`${key}@${os}` as Key] ?? locale[key] ?? en[key];
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
