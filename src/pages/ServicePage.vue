<script setup lang="ts">
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { t } from '../i18n';
import { save, settings, type Settings, type Translation } from '../store';
import Icon from '../components/Icon.vue';

const s = settings.value!;
type Provider = Settings['api_provider'];
type KeyField = 'deepl_key' | 'openrouter_key';

const PROVIDERS: { id: Provider; key?: KeyField; keyUrl?: string; free?: boolean }[] = [
  { id: 'google', free: true },
  { id: 'deepL', key: 'deepl_key', keyUrl: 'https://www.deepl.com/your-account/keys' },
  { id: 'openrouter', key: 'openrouter_key', keyUrl: 'https://openrouter.ai/settings/keys' },
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
    const r = await invoke<Translation>('translate_text', { text: s.target_lang === 'en' ? 'Merhaba, nasılsın?' : 'Hello, how are you?' });
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
      <div>
        <h1>{{ t('service.title') }}</h1>
        <p>{{ t('service.lead') }}</p>
      </div>
    </header>

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
            <span v-if="p.free" class="badge accent">{{ t('service.free') }}</span>
            <span v-if="p.free" class="badge">{{ t('service.recommended') }}</span>
          </span>
          <span class="desc" style="display: block">{{ t(`service.${p.id}.desc`) }}</span>
        </span>
      </button>
    </div>

    <div class="rows">
      <div v-if="current.key" class="row stack">
        <div class="sect-head">
          <h2>{{ t('service.key') }}</h2>
          <button class="link" @click="openUrl(current.keyUrl!)">{{ t('service.getKey') }} ↗</button>
        </div>
        <div class="key-field">
          <input
            v-model.trim="s[current.key]"
            class="input"
            :type="showKey ? 'text' : 'password'"
            :placeholder="current.id === 'deepL' ? 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx:fx' : 'sk-or-v1-…'"
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

      <div v-if="current.key" class="row">
        <div class="info">
          <div class="name">{{ t('service.style') }}</div>
          <div class="desc">{{ t('service.style.desc') }}</div>
        </div>
        <div class="ctl seg" role="radiogroup" :aria-label="t('service.style')">
          <button role="radio" :aria-checked="s.translation_type === 'casual'" @click="s.translation_type = 'casual'">{{ t('service.casual') }}</button>
          <button role="radio" :aria-checked="s.translation_type === 'formal'" @click="s.translation_type = 'formal'">{{ t('service.formal') }}</button>
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
</template>
