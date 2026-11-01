<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { langName, t } from '../i18n';
import { languages, save, settings, type Translation } from '../store';
import Icon from '../components/Icon.vue';
import Keys from '../components/Keys.vue';

const s = settings.value!;
const input = ref('');
const result = ref<Translation | null>(null);
const error = ref('');
const busy = ref(false);
const copied = ref(false);
let request = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

const named = computed(() => languages.value.map(([code, en]) => [code, langName(code, en)] as const));

/** Translates with the languages on screen; an answer that arrives after a newer request is dropped. */
async function run() {
  clearTimeout(timer);
  const text = input.value;
  const id = ++request;
  if (!text.trim()) {
    result.value = null;
    error.value = '';
    busy.value = false;
    return;
  }
  busy.value = true;
  // The Rust side translates with the saved settings, so a language just picked is saved first.
  const problem = await save();
  if (id !== request) return;
  if (problem) {
    error.value = problem;
    busy.value = false;
    return;
  }
  try {
    const r = await invoke<Translation>('translate_text', { text });
    if (id !== request) return;
    result.value = r;
    error.value = '';
  } catch (e) {
    if (id !== request) return;
    result.value = null;
    error.value = String(e);
  } finally {
    if (id === request) busy.value = false;
  }
}

watch(input, () => {
  clearTimeout(timer);
  timer = setTimeout(run, 450);
});
watch(() => [s.source_lang, s.target_lang, s.translation_type], run);

function swap() {
  const from = s.source_lang === 'auto' ? result.value?.detected ?? null : s.source_lang;
  if (!from || from === s.target_lang) return;
  if (result.value) input.value = result.value.text;
  [s.source_lang, s.target_lang] = [s.target_lang, from];
}

async function copy() {
  if (!result.value) return;
  await navigator.clipboard.writeText(result.value.text);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1400);
}

/** What the footer says about the answer: the detected language, or that it went the other way. */
const detail = computed(() => {
  const r = result.value;
  if (!r) return '';
  const via = t('translate.via', { provider: r.provider });
  if (r.target !== s.target_lang && r.detected)
    return `${t('translate.reversed', { lang: langName(r.detected), other: langName(r.target) })} · ${via}`;
  if (s.source_lang === 'auto' && r.detected) return `${t('translate.detected', { lang: langName(r.detected) })} · ${via}`;
  return via;
});
const canSwap = computed(() => s.source_lang !== 'auto' || !!result.value?.detected);
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div>
        <h1>{{ t('translate.title') }}</h1>
        <p>{{ t('translate.lead') }} <Keys :combo="s.hotkey" class="inline-keys" /></p>
      </div>
      <!-- Casual or formal, at hand where translating happens; Google has no style, so it is not offered there. -->
      <div v-if="s.api_provider !== 'google'" class="seg" role="radiogroup" :aria-label="t('service.style')">
        <button role="radio" :aria-checked="s.translation_type === 'casual'" @click="s.translation_type = 'casual'">{{ t('service.casual') }}</button>
        <button role="radio" :aria-checked="s.translation_type === 'formal'" @click="s.translation_type = 'formal'">{{ t('service.formal') }}</button>
      </div>
    </header>

    <div class="langbar">
      <select v-model="s.source_lang" class="select" :aria-label="t('translate.from')">
        <option value="auto">{{ t('translate.auto') }}</option>
        <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
      </select>
      <button class="btn subtle icon-only" :title="t('translate.swap')" :aria-label="t('translate.swap')" :disabled="!canSwap" @click="swap">
        <Icon name="swap" />
      </button>
      <select v-model="s.target_lang" class="select" :aria-label="t('translate.to')">
        <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
      </select>
    </div>

    <div class="panes">
      <div class="pane">
        <textarea
          v-model="input"
          :placeholder="t('translate.placeholder')"
          :aria-label="t('translate.placeholder')"
          spellcheck="false"
          autofocus
          @keydown.ctrl.enter.prevent="run"
        />
        <div class="pane-foot">
          <span class="grow">{{ input ? t('translate.chars', { n: input.length }) : '' }}</span>
          <button v-if="input" class="btn subtle icon-only" :title="t('translate.clear')" :aria-label="t('translate.clear')" @click="input = ''">
            <Icon name="close" />
          </button>
        </div>
      </div>
      <div class="pane" aria-live="polite">
        <p v-if="error" class="out note bad selectable">{{ error }}</p>
        <p v-else-if="result" class="out selectable">{{ result.text }}</p>
        <p v-else class="out empty">{{ t('translate.output') }}</p>
        <div class="pane-foot">
          <span v-if="busy" class="grow"><span class="dots" :aria-label="t('translate.working')"><i /><i /><i /></span></span>
          <span v-else class="grow" :title="detail">{{ detail }}</span>
          <button v-if="result" class="btn subtle" @click="copy">
            <Icon :name="copied ? 'check' : 'copy'" />{{ copied ? t('translate.copied') : t('translate.copy') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
