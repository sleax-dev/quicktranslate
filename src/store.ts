import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { disable, enable } from '@tauri-apps/plugin-autostart';
import { t, uiLang } from './i18n';

export interface Settings {
  source_lang: string;
  target_lang: string;
  auto_start: boolean;
  hotkey: string;
  translation_type: 'casual' | 'formal';
  api_provider: 'google' | 'deepL' | 'openrouter';
  deepl_key: string;
  openrouter_key: string;
  discord_send: boolean;
  auto_update: boolean;
}

export interface Translation {
  text: string;
  detected: string | null;
  target: string;
  provider: string;
}

export type Page = 'translate' | 'history' | 'service' | 'settings';

export const page = ref<Page>('translate');
export const settings = ref<Settings | null>(null);
/** [code, English name] pairs, from the Rust side so the list exists once. */
export const languages = ref<[string, string][]>([]);
export const mica = ref(false);
/** Why the last change could not be saved, or ''. */
export const saveError = ref('');
/** Bumped after every successful save, for the "Saved" hint. */
export const savedCount = ref(0);

let saved: Settings | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export async function load() {
  const [s, langs, lang, hasMica] = await Promise.all([
    invoke<Settings>('get_settings'),
    invoke<[string, string][]>('get_languages'),
    invoke<'tr' | 'en'>('ui_language'),
    invoke<boolean>('mica_enabled'),
  ]);
  uiLang.value = lang;
  languages.value = langs;
  mica.value = hasMica;
  saved = { ...s };
  settings.value = s;
  // Every change is saved shortly after it is made; typing an API key saves once, not per letter.
  watch(settings, () => {
    clearTimeout(timer);
    timer = setTimeout(() => void save(), 400);
  }, { deep: true });
}

/** Saves now and returns why it failed, or ''. Changes are also saved on their own shortly after they are made. */
export async function save(): Promise<string> {
  clearTimeout(timer);
  const s = settings.value;
  if (!s || !saved) return '';
  if (s.source_lang === s.target_lang) return (saveError.value = t('settings.sameLang'));
  const next = { ...s };
  try {
    await invoke('update_settings', { newSettings: next });
  } catch (e) {
    // Windows refused the hotkey: the old one is still the one that works, so show that one.
    if (next.hotkey !== saved.hotkey) s.hotkey = saved.hotkey;
    return (saveError.value = String(e));
  }
  const autostartChanged = next.auto_start !== saved.auto_start;
  saved = next;
  if (autostartChanged) {
    try {
      await (next.auto_start ? enable() : disable());
    } catch (e) {
      return (saveError.value = String(e));
    }
  }
  saveError.value = '';
  savedCount.value++;
  return '';
}
