<script setup lang="ts">
import { os } from '../i18n';

defineProps<{ combo: string }>();

// macOS shows modifiers as symbols: ⌃ ⌥ ⇧ ⌘.
const MAC: Record<string, string> = { Ctrl: '⌃', Control: '⌃', Alt: '⌥', Option: '⌥', Shift: '⇧', Cmd: '⌘', Command: '⌘', Super: '⌘', Meta: '⌘' };
const label = (key: string) => (os === 'macos' ? MAC[key] ?? key : key);
</script>

<template>
  <span class="keys" :aria-label="combo">
    <template v-for="(key, i) in combo.split('+')" :key="i">
      <span v-if="i" class="plus" aria-hidden="true">+</span>
      <kbd aria-hidden="true">{{ label(key) }}</kbd>
    </template>
  </span>
</template>
