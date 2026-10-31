<script setup lang="ts">
import { langName, t, uiLang } from '../i18n';
import { clearHistory, history } from '../history';
import { settings } from '../store';
import Icon from '../components/Icon.vue';
import Keys from '../components/Keys.vue';

const time = (at: number) => new Date(at).toLocaleTimeString(uiLang.value, { hour: '2-digit', minute: '2-digit' });
const copy = (text: string) => navigator.clipboard.writeText(text);
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div>
        <h1>{{ t('history.title') }}</h1>
        <p>{{ t('history.lead') }}</p>
      </div>
      <button v-if="history.length" class="btn" @click="clearHistory"><Icon name="trash" />{{ t('history.clear') }}</button>
    </header>

    <div v-if="history.length" class="rows">
      <article v-for="entry in history" :key="entry.at" class="entry">
        <div class="meta">
          <span>{{ time(entry.at) }}</span>
          <span aria-hidden="true">·</span>
          <span>{{ entry.detected ? langName(entry.detected) : '?' }}</span>
          <Icon name="arrow" />
          <span>{{ langName(entry.target) }}</span>
          <span aria-hidden="true">·</span>
          <span>{{ entry.provider }}</span>
        </div>
        <p class="orig selectable" style="margin: 0">{{ entry.original }}</p>
        <p class="tran selectable" style="margin: 0">{{ entry.text }}</p>
        <div class="acts">
          <button class="btn subtle icon-only" :title="t('history.copyOriginal')" :aria-label="t('history.copyOriginal')" @click="copy(entry.original)">
            <Icon name="arrow" style="transform: scaleX(-1)" />
          </button>
          <button class="btn subtle icon-only" :title="t('history.copyTranslation')" :aria-label="t('history.copyTranslation')" @click="copy(entry.text)">
            <Icon name="copy" />
          </button>
        </div>
      </article>
    </div>
    <div v-else class="rows empty-state">
      <Icon name="history" />
      <p style="margin: 0">{{ t('history.empty') }} <Keys :combo="settings!.hotkey" class="inline-keys" /></p>
    </div>
  </section>
</template>
