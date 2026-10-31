<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import { langName, t } from '../i18n';
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
  if (!e.ctrlKey && !e.altKey && !(e.shiftKey && /^F\d+$/.test(e.code))) {
    hotkeyError.value = t('settings.needModifier');
    return;
  }
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
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
    </div>

    <p v-if="saveError && saveError !== hotkeyError" class="note bad" style="margin: 0"><Icon name="warning" />{{ saveError }}</p>
    <p class="small muted" style="margin: 0">QuickTranslate · {{ t('settings.version', { v: version }) }}</p>
  </section>
</template>
