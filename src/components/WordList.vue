<script setup lang="ts">
import { ref } from 'vue';
import { t } from '../i18n';
import { settings } from '../store';
import Icon from './Icon.vue';

// The same limits as the Rust side, which also cleans the list when it saves.
const MIN = 3;
const MAX = 40;
const COUNT = 200;

const s = settings.value!;
const draft = ref('');
const problem = ref('');

/** Adds what is typed; a comma or Enter ends a word, so a pasted "Umut, Ali, Zeynep" adds three. */
function add() {
  const words = draft.value.split(',').map((w) => w.trim().replace(/\s+/g, ' ')).filter(Boolean);
  problem.value = '';
  for (const word of words) {
    const length = [...word].length;
    if (length < MIN) problem.value = t('words.tooShort', { n: MIN });
    else if (length > MAX) problem.value = t('words.tooLong', { n: MAX });
    else if (s.protected_words.some((w) => w.toLocaleLowerCase() === word.toLocaleLowerCase())) problem.value = t('words.duplicate', { word });
    else if (s.protected_words.length >= COUNT) problem.value = t('words.full', { n: COUNT });
    else s.protected_words.push(word);
  }
  if (!problem.value) draft.value = '';
}

function remove(i: number) {
  s.protected_words.splice(i, 1);
}
</script>

<template>
  <div class="words">
    <div class="word-input">
      <input
        v-model="draft"
        class="input"
        :placeholder="t('words.placeholder')"
        :aria-label="t('words.title')"
        :maxlength="MAX * 4"
        spellcheck="false"
        @keydown.enter.prevent="add"
        @keydown.,.prevent="add"
        @input="problem = ''"
      />
      <button class="btn" :disabled="!draft.trim()" @click="add">{{ t('words.add') }}</button>
    </div>
    <p v-if="problem" class="note bad" style="margin: 0"><Icon name="warning" />{{ problem }}</p>
    <ul v-if="s.protected_words.length" class="chips" :aria-label="t('words.title')">
      <li v-for="(word, i) in s.protected_words" :key="word" class="chip">
        <span class="selectable">{{ word }}</span>
        <button :title="t('words.remove', { word })" :aria-label="t('words.remove', { word })" @click="remove(i)"><Icon name="close" /></button>
      </li>
    </ul>
  </div>
</template>
