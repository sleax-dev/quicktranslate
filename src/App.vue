<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { disable, enable } from '@tauri-apps/plugin-autostart';

interface Settings {
  source_lang: string;
  target_lang: string;
  auto_start: boolean;
  hotkey: string;
  translation_type: string;
  api_provider: string;
  api_key: string;
}

const API_PROVIDERS = [
  { code: 'mymemory', name: 'MyMemory (Free)', needKey: false },
  { code: 'deepL', name: 'DeepL (API Key required)', needKey: true },
  { code: 'openrouter', name: 'OpenRouter (API Key required)', needKey: true },
];

const TRANSLATION_TYPES = [
  { code: 'casual', name: 'Casual' },
  { code: 'formal', name: 'Formal' },
];

const MODIFIER_KEYS = ['Control', 'Shift', 'Alt', 'Meta'];

const settings = ref<Settings | null>(null);
/** [code, name] pairs, from the Rust side so the list exists once. */
const languages = ref<[string, string][]>([]);
const saved = ref(false);
const recordingHotkey = ref(false);
const hotkeyError = ref('');
const saveError = ref('');
const showApiKey = ref(false);

const selectedProvider = computed(() => API_PROVIDERS.find(p => p.code === settings.value?.api_provider));

/** Why the settings can't be saved yet, or '' when they can. */
const problem = computed(() => {
  const s = settings.value;
  if (!s) return '';
  if (hotkeyError.value) return hotkeyError.value;
  if (s.source_lang === s.target_lang) return 'Source and target language are the same';
  if (selectedProvider.value?.needKey && !s.api_key.trim()) return 'Enter an API key for this service';
  return '';
});

onMounted(async () => {
  [settings.value, languages.value] = await Promise.all([
    invoke<Settings>('get_settings'),
    invoke<[string, string][]>('get_languages'),
  ]);
});

const startRecording = () => {
  recordingHotkey.value = true;
  hotkeyError.value = '';
};

// Uses the physical key (e.code), so Shift+1 records "1", not "!", whatever the keyboard layout.
const handleHotkeyRecord = (e: KeyboardEvent) => {
  if (!recordingHotkey.value || !settings.value) return;
  e.preventDefault();
  if (MODIFIER_KEYS.some(m => e.code.startsWith(m))) return;

  if (!e.ctrlKey && !e.shiftKey && !e.altKey) {
    hotkeyError.value = 'Ctrl, Shift or Alt required';
    return;
  }
  hotkeyError.value = '';

  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');
  parts.push(e.code.replace(/^(Key|Digit)/, ''));

  settings.value.hotkey = parts.join('+');
  recordingHotkey.value = false;
};

const handleSave = async () => {
  if (!settings.value || problem.value) return;
  saveError.value = '';
  try {
    await invoke('update_settings', { newSettings: settings.value });
    await invoke('register_hotkey', { hotkey: settings.value.hotkey }).catch((e) => {
      throw new Error(`Hotkey could not be registered: ${e}`);
    });
    await (settings.value.auto_start ? enable() : disable());
    saved.value = true;
    setTimeout(() => (saved.value = false), 1500);
  } catch (e) {
    saveError.value = e instanceof Error ? e.message : String(e);
  }
};
</script>

<template>
  <div v-if="settings" class="h-screen bg-black text-white flex flex-col">
    <!-- Header -->
    <header class="h-12 border-b border-zinc-900 flex items-center gap-2 px-4 flex-shrink-0">
      <img src="/icons/32x32.png" alt="Logo" class="w-6 h-6 rounded" />
      <span class="font-medium text-sm">QuickTranslate</span>
    </header>

    <!-- Content -->
    <main class="flex-1 p-4">
      <div class="flex flex-col gap-4">
        <!-- Source & Target Language -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="field-label">Source</label>
            <select
              v-model="settings.source_lang"
              class="field"
            >
              <option value="auto">Auto</option>
              <option v-for="[code, name] in languages" :key="code" :value="code">{{ name }}</option>
            </select>
          </div>

          <div>
            <label class="field-label">Target</label>
            <select
              v-model="settings.target_lang"
              class="field"
            >
              <option v-for="[code, name] in languages" :key="code" :value="code">{{ name }}</option>
            </select>
          </div>
        </div>

        <!-- Translation Type -->
        <div>
          <label class="field-label">Style</label>
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="type in TRANSLATION_TYPES"
              :key="type.code"
              @click="settings.translation_type = type.code"
              :class="[
                'p-2 rounded-lg border text-center text-sm transition-colors',
                settings.translation_type === type.code
                  ? 'border-blue-500 bg-blue-500/10 text-white'
                  : 'border-zinc-800 bg-zinc-950 text-zinc-400 hover:border-zinc-600'
              ]"
            >
              {{ type.name }}
            </button>
          </div>
        </div>

        <!-- API Provider -->
        <div>
          <label class="field-label">Service</label>
          <select
            v-model="settings.api_provider"
            class="field"
          >
            <option v-for="provider in API_PROVIDERS" :key="provider.code" :value="provider.code">
              {{ provider.name }}
            </option>
          </select>
        </div>

        <!-- API Key -->
        <div v-if="selectedProvider?.needKey">
          <label class="field-label">
            <span class="flex items-center gap-1.5">API Key</span>
          </label>
          <div class="relative">
            <input
              :type="showApiKey ? 'text' : 'password'"
              v-model="settings.api_key"
              :placeholder="settings.api_provider === 'deepL' ? 'DeepL API Key' : 'OpenRouter API Key'"
              class="field pr-16 placeholder-zinc-600"
            />
            <button
              type="button"
              @click="showApiKey = !showApiKey"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-zinc-500 hover:text-zinc-300"
            >
              {{ showApiKey ? 'Hide' : 'Show' }}
            </button>
          </div>
        </div>

        <!-- Hotkey -->
        <div>
          <label class="field-label">Hotkey</label>
          <button
            @click="startRecording"
            @keydown="handleHotkeyRecord"
            @blur="recordingHotkey = false"
            :class="[
              'w-full bg-zinc-950 border rounded-lg px-3 py-2 text-sm transition-colors',
              recordingHotkey
                ? 'border-blue-500 text-blue-400'
                : 'border-zinc-800 text-white hover:border-zinc-600'
            ]"
          >
            {{ recordingHotkey ? 'Press a key...' : settings.hotkey }}
          </button>
        </div>

        <!-- Auto Start -->
        <div class="flex items-center justify-between p-2.5 bg-zinc-950 rounded-lg border border-zinc-900">
          <span class="text-sm text-white">Auto start</span>
          <button
            @click="settings.auto_start = !settings.auto_start"
            :class="[
              'relative w-9 h-5 rounded-full transition-colors',
              settings.auto_start ? 'bg-white' : 'bg-zinc-700'
            ]"
          >
            <div
              :class="[
                'absolute top-0.5 w-4 h-4 bg-black rounded-full transition-transform',
                settings.auto_start ? 'translate-x-4' : 'translate-x-0.5'
              ]"
            />
          </button>
        </div>

        <!-- Save Button -->
        <button
          @click="handleSave"
          :disabled="!!problem"
          :class="[
            'w-full py-2.5 rounded-lg font-medium text-sm transition-colors',
            saved
              ? 'bg-green-600 text-white'
              : 'bg-white text-black hover:bg-zinc-200 disabled:opacity-50 disabled:cursor-not-allowed'
          ]"
        >
          {{ saved ? '✓ Saved' : 'Save' }}
        </button>
        <p v-if="problem || saveError" class="-mt-2 text-xs text-amber-500">{{ problem || saveError }}</p>
      </div>
    </main>

    <!-- Footer -->
    <footer class="p-3 text-center border-t border-zinc-900 flex-shrink-0">
      <p class="text-xs text-zinc-600">{{ settings.hotkey }} to translate</p>
    </footer>
  </div>

  <!-- Loading -->
  <div v-else class="h-screen bg-black flex items-center justify-center">
    <div class="w-5 h-5 border-2 border-zinc-800 border-t-zinc-400 rounded-full animate-spin" />
  </div>
</template>
