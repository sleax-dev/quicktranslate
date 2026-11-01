<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { LANGUAGES, langName, os, t, type Lang } from '../i18n';
import {
  checkUpdate, installUpdate, languages, save, saveError, savedCount, settings, update,
  type Settings, type Translation,
} from '../store';
import Icon from '../components/Icon.vue';
import Keys from '../components/Keys.vue';
import WordList from '../components/WordList.vue';

const s = settings.value!;
const version = ref('');
const justSaved = ref(false);
let savedTimer: ReturnType<typeof setTimeout> | undefined;
void getVersion().then((v) => (version.value = v));
watch(savedCount, () => {
  justSaved.value = true;
  clearTimeout(savedTimer);
  savedTimer = setTimeout(() => (justSaved.value = false), 1600);
});

const named = computed(() => languages.value.map(([code, en]) => [code, langName(code, en)] as const));
/** The system language's own name, for the "System (…)" choice. */
const systemName = ref('');
void invoke<Lang>('system_language').then((l) => (systemName.value = LANGUAGES[l]));

// ---- hotkey
const recording = ref(false);
const hotkeyError = ref('');
const MODIFIERS = ['Control', 'Shift', 'Alt', 'Meta'];

// Uses the physical key (e.code), so Shift+1 records "1", not "!", whatever the keyboard layout.
async function record(e: KeyboardEvent) {
  if (!recording.value) return;
  e.preventDefault();
  if (e.key === 'Escape') {
    recording.value = false;
    return;
  }
  if (MODIFIERS.some((m) => e.code.startsWith(m))) return;
  const cmd = os === 'macos' && e.metaKey; // ⌘ is a hotkey modifier on macOS only; Windows keeps Win for itself
  if (!e.ctrlKey && !e.altKey && !cmd && !(e.shiftKey && /^F\d+$/.test(e.code))) {
    hotkeyError.value = t('settings.needModifier');
    return;
  }
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (cmd) parts.push('Cmd');
  parts.push(e.code.replace(/^(Key|Digit)/, ''));
  recording.value = false;
  s.hotkey = parts.join('+');
  hotkeyError.value = await save();
}

// ---- translation service
type Provider = Settings['api_provider'];
type KeyField = 'gemini_key' | 'deepl_key' | 'openrouter_key';
const PROVIDERS: { id: Provider; key?: KeyField; keyUrl?: string; placeholder?: string; badge?: 'free' | 'style' }[] = [
  { id: 'google', badge: 'free' },
  { id: 'gemini', key: 'gemini_key', keyUrl: 'https://aistudio.google.com/apikey', placeholder: 'AIza…', badge: 'style' },
  { id: 'deepL', key: 'deepl_key', keyUrl: 'https://www.deepl.com/your-account/keys', placeholder: 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx:fx' },
  { id: 'openrouter', key: 'openrouter_key', keyUrl: 'https://openrouter.ai/settings/keys', placeholder: 'sk-or-v1-…' },
];
const current = computed(() => PROVIDERS.find((p) => p.id === s.api_provider) ?? PROVIDERS[0]);
const showKey = ref(false);
const test = ref<{ ok: boolean; text: string } | null>(null);
const testing = ref(false);

function pick(id: Provider) {
  s.api_provider = id;
  test.value = null;
}

/** Translates a sample with what is on screen, so a bad key shows up here and not mid-sentence in another app. */
async function runTest() {
  testing.value = true;
  test.value = null;
  const started = performance.now();
  try {
    const problem = await save();
    if (problem) throw problem;
    const sample = s.target_lang === 'en' ? 'Merhaba, nasılsın? Yarın toplantıya gelebilir misin?' : 'Hey, how are you? Can you make it to the meeting tomorrow?';
    const r = await invoke<Translation>('translate_text', { text: sample });
    test.value = { ok: true, text: t('service.testOk', { text: r.text, ms: Math.round(performance.now() - started) }) };
  } catch (e) {
    test.value = { ok: false, text: String(e) };
  } finally {
    testing.value = false;
  }
}
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div><h1>{{ t('settings.title') }}</h1></div>
      <span v-if="justSaved && !saveError" class="saved-hint"><Icon name="check" />{{ t('settings.saved') }}</span>
    </header>

    <!-- General -->
    <section class="stack">
      <div class="sect-head"><h2>{{ t('settings.general') }}</h2></div>
      <div class="rows">
        <div class="row">
          <div class="info"><div class="name">{{ t('settings.language') }}</div></div>
          <select v-model="s.ui_lang" class="select ctl" :aria-label="t('settings.language')">
            <option value="">{{ t('settings.language.system', { lang: systemName }) }}</option>
            <option v-for="(name, code) in LANGUAGES" :key="code" :value="code">{{ name }}</option>
          </select>
        </div>
        <div class="row">
          <div class="info">
            <div class="name">{{ t('settings.autostart') }}</div>
            <div class="desc">{{ t('settings.autostart.desc') }}</div>
          </div>
          <button class="switch ctl" role="switch" :aria-checked="s.auto_start" :aria-label="t('settings.autostart')" @click="s.auto_start = !s.auto_start" />
        </div>
      </div>
    </section>

    <!-- Translation -->
    <section class="stack">
      <div class="sect-head"><h2>{{ t('settings.translation') }}</h2></div>
      <div class="rows">
        <div class="row stack">
          <div class="info">
            <div class="name">{{ t('settings.languages') }}</div>
            <div class="desc">{{ t('settings.languages.desc') }}</div>
          </div>
          <div class="langbar" style="grid-template-columns: 1fr auto 1fr">
            <label class="stack-label">
              <span class="small muted">{{ t('translate.from') }}</span>
              <select v-model="s.source_lang" class="select">
                <option value="auto">{{ t('translate.auto') }}</option>
                <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
              </select>
            </label>
            <Icon name="arrow" class="muted flip-rtl" style="margin-top: 20px" />
            <label class="stack-label">
              <span class="small muted">{{ t('translate.to') }}</span>
              <select v-model="s.target_lang" class="select">
                <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
              </select>
            </label>
          </div>
        </div>
        <div class="row">
          <div class="info">
            <div class="name">{{ t('settings.hotkey') }}</div>
            <div class="desc">{{ t('settings.hotkey.desc') }}</div>
            <p v-if="hotkeyError" class="note bad" style="margin: 6px 0 0"><Icon name="warning" />{{ hotkeyError }}</p>
          </div>
          <button
            :class="['recorder', 'ctl', { recording }]"
            :title="t('settings.recordHint')"
            @click="recording = true; hotkeyError = ''"
            @keydown="record"
            @blur="recording = false"
          >
            <template v-if="recording"><Icon name="keyboard" />{{ t('settings.record') }}</template>
            <Keys v-else :combo="s.hotkey" />
          </button>
        </div>
        <div id="words" class="row stack">
          <div class="info">
            <div class="name">{{ t('words.title') }}</div>
            <div class="desc">{{ t('words.desc') }}</div>
          </div>
          <WordList />
        </div>
      </div>
    </section>

    <!-- Translation service -->
    <section class="stack">
      <div class="sect-head"><h2>{{ t('service.title') }}</h2></div>
      <div class="rows" role="radiogroup" :aria-label="t('service.title')">
        <button
          v-for="p in PROVIDERS"
          :key="p.id"
          class="choice"
          role="radio"
          :aria-checked="s.api_provider === p.id"
          @click="pick(p.id)"
        >
          <span class="radio" aria-hidden="true" />
          <span class="info">
            <span class="name">
              {{ t(`service.${p.id}`) }}
              <span v-if="p.badge === 'free'" class="badge accent">{{ t('service.free') }}</span>
              <span v-if="p.badge === 'style'" class="badge accent"><Icon name="sparkle" />{{ t('service.bestStyle') }}</span>
            </span>
            <span class="desc" style="display: block">{{ t(`service.${p.id}.desc`) }}</span>
          </span>
        </button>
      </div>

      <div class="rows">
        <div v-if="current.key" class="row stack">
          <div class="sect-head">
            <div class="name">{{ t('service.key') }}</div>
            <button class="link" @click="openUrl(current.keyUrl!)">{{ t('service.getKey') }} ↗</button>
          </div>
          <div class="key-field">
            <input
              v-model.trim="s[current.key]"
              class="input"
              :type="showKey ? 'text' : 'password'"
              :placeholder="current.placeholder"
              :aria-label="t('service.key')"
              spellcheck="false"
              autocomplete="off"
            />
            <button class="btn subtle icon-only" :title="showKey ? t('service.hide') : t('service.show')" :aria-label="showKey ? t('service.hide') : t('service.show')" @click="showKey = !showKey">
              <Icon :name="showKey ? 'eyeOff' : 'eye'" />
            </button>
          </div>
          <p v-if="!s[current.key]" class="note warn" style="margin: 0">
            <Icon name="warning" />{{ t('service.keyMissing', { provider: t(`service.${current.id}`) }) }}
          </p>
        </div>

        <div class="row">
          <div class="info">
            <div class="name">{{ t('service.style') }}</div>
            <div class="desc">{{ current.key ? t('service.style.desc') : t('service.style.none') }}</div>
          </div>
          <div class="ctl seg" role="radiogroup" :aria-label="t('service.style')" :aria-disabled="!current.key">
            <button role="radio" :disabled="!current.key" :aria-checked="s.translation_type === 'casual'" @click="s.translation_type = 'casual'">{{ t('service.casual') }}</button>
            <button role="radio" :disabled="!current.key" :aria-checked="s.translation_type === 'formal'" @click="s.translation_type = 'formal'">{{ t('service.formal') }}</button>
          </div>
        </div>

        <div class="row">
          <div class="info" aria-live="polite">
            <p v-if="test" :class="['note', test.ok ? 'ok' : 'bad']" style="margin: 0">
              <Icon :name="test.ok ? 'check' : 'warning'" /><span class="selectable">{{ test.text }}</span>
            </p>
            <span v-else class="muted">{{ t(`service.${current.id}`) }}</span>
          </div>
          <button class="btn primary ctl" :disabled="testing || (!!current.key && !s[current.key])" @click="runTest">
            {{ testing ? t('service.testing') : t('service.test') }}
          </button>
        </div>
      </div>
    </section>

    <!-- Updates -->
    <section class="stack">
      <div class="sect-head"><h2>{{ t('settings.updates') }}</h2></div>
      <div class="rows">
        <div class="row">
          <div class="info">
            <div class="name">{{ t('settings.version', { v: version }) }}</div>
            <div v-if="update.error" class="desc bad-text">{{ update.error }}</div>
            <div v-else-if="update.version" class="desc">{{ t('settings.updateFound', { v: update.version }) }}</div>
            <div v-else-if="update.checked" class="desc">{{ t('settings.upToDate') }}</div>
          </div>
          <button v-if="update.version" class="btn primary ctl" :disabled="!!update.busy" @click="installUpdate">
            {{ update.busy === 'installing' ? t('settings.installing') : t('settings.install') }}
          </button>
          <button v-else class="btn ctl" :disabled="!!update.busy" @click="checkUpdate()">
            {{ update.busy === 'checking' ? t('settings.checking') : t('settings.checkUpdate') }}
          </button>
        </div>
        <div class="row">
          <div class="info">
            <div class="name">{{ t('settings.autoUpdate') }}</div>
            <div class="desc">{{ t('settings.autoUpdate.desc') }}</div>
          </div>
          <button class="switch ctl" role="switch" :aria-checked="s.auto_update" :aria-label="t('settings.autoUpdate')" @click="s.auto_update = !s.auto_update" />
        </div>
      </div>
    </section>

    <!-- Tools -->
    <section class="stack">
      <div class="sect-head"><h2>{{ t('settings.tools') }}</h2></div>
      <div class="rows">
        <div class="row">
          <div class="info">
            <div class="name">{{ t('settings.quit') }}</div>
            <div class="desc">{{ t('settings.quit.desc') }}</div>
          </div>
          <button class="btn ctl" @click="invoke('quit_app')"><Icon name="power" />{{ t('settings.quit') }}</button>
        </div>
      </div>
    </section>

    <p v-if="saveError && saveError !== hotkeyError" class="note bad" style="margin: 0"><Icon name="warning" />{{ saveError }}</p>
  </section>
</template>
