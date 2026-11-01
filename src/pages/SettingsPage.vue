<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { langName, os, t } from '../i18n';
import { languages, save, saveError, savedCount, settings } from '../store';
import Icon from '../components/Icon.vue';
import Keys from '../components/Keys.vue';

const s = settings.value!;
const recording = ref(false);
const hotkeyError = ref('');
const version = ref('');
const justSaved = ref(false);
let savedTimer: ReturnType<typeof setTimeout> | undefined;

void getVersion().then((v) => (version.value = v));

/** The newer version found, '' when up to date, null before a check. */
const update = ref<string | null>(null);
const updating = ref<'' | 'checking' | 'installing'>('');
const updateError = ref('');
// The background check found one while the window was open: offer it here.
void listen<string>('update-available', (e) => (update.value = e.payload));

async function checkUpdate() {
  updating.value = 'checking';
  updateError.value = '';
  try {
    update.value = (await invoke<string | null>('check_update')) ?? '';
  } catch (e) {
    updateError.value = String(e);
  }
  updating.value = '';
}

async function installUpdate() {
  updating.value = 'installing';
  updateError.value = '';
  try {
    await invoke('install_update'); // restarts the app when it works
  } catch (e) {
    updateError.value = String(e);
  }
  updating.value = '';
}
watch(savedCount, () => {
  justSaved.value = true;
  clearTimeout(savedTimer);
  savedTimer = setTimeout(() => (justSaved.value = false), 1600);
});

const named = computed(() => languages.value.map(([code, en]) => [code, langName(code, en)] as const));
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
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div>
        <h1>{{ t('settings.title') }}</h1>
      </div>
      <span v-if="justSaved && !saveError" class="saved-hint"><Icon name="check" />{{ t('settings.saved') }}</span>
    </header>

    <div class="rows">
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
          <Icon name="arrow" class="muted" style="margin-top: 20px" />
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
          <div class="name">{{ t('settings.autostart') }}</div>
          <div class="desc">{{ t('settings.autostart.desc') }}</div>
        </div>
        <button
          class="switch ctl"
          role="switch"
          :aria-checked="s.auto_start"
          :aria-label="t('settings.autostart')"
          @click="s.auto_start = !s.auto_start"
        />
      </div>

      <div v-if="os === 'windows'" class="row">
        <div class="info">
          <div class="name">{{ t('settings.discord') }}</div>
          <div class="desc">{{ t('settings.discord.desc') }}</div>
        </div>
        <button
          class="switch ctl"
          role="switch"
          :aria-checked="s.discord_send"
          :aria-label="t('settings.discord')"
          @click="s.discord_send = !s.discord_send"
        />
      </div>

      <div v-if="os === 'windows'" class="row">
        <div class="info">
          <div class="name">{{ t('settings.discordRead') }}</div>
          <div class="desc">{{ t('settings.discordRead.desc') }}</div>
        </div>
        <button
          class="switch ctl"
          role="switch"
          :aria-checked="s.discord_read"
          :aria-label="t('settings.discordRead')"
          @click="s.discord_read = !s.discord_read"
        />
      </div>

      <div class="row">
        <div class="info">
          <div class="name">{{ t('settings.autoUpdate') }}</div>
          <div class="desc">{{ t('settings.autoUpdate.desc') }}</div>
          <p v-if="updateError" class="note bad" style="margin: 6px 0 0"><Icon name="warning" />{{ updateError }}</p>
          <p v-else-if="update" class="small" style="margin: 6px 0 0">{{ t('settings.updateFound', { v: update }) }}</p>
          <p v-else-if="update === ''" class="small muted" style="margin: 6px 0 0">{{ t('settings.upToDate') }}</p>
        </div>
        <div style="display: flex; gap: 8px; align-items: center">
          <button v-if="update" class="btn primary ctl" :disabled="!!updating" @click="installUpdate">
            {{ updating === 'installing' ? t('settings.installing') : t('settings.install') }}
          </button>
          <button v-else class="btn ctl" :disabled="!!updating" @click="checkUpdate">
            {{ updating === 'checking' ? t('settings.checking') : t('settings.checkUpdate') }}
          </button>
          <button
            class="switch ctl"
            role="switch"
            :aria-checked="s.auto_update"
            :aria-label="t('settings.autoUpdate')"
            @click="s.auto_update = !s.auto_update"
          />
        </div>
      </div>
    </div>

    <p v-if="saveError && saveError !== hotkeyError" class="note bad" style="margin: 0"><Icon name="warning" />{{ saveError }}</p>
    <p class="small muted" style="margin: 0">QuickTranslate · {{ t('settings.version', { v: version }) }}</p>
  </section>
</template>
