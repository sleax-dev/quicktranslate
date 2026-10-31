<script setup lang="ts">
defineProps<{ combo: string }>();
</script>

<template>
  <span class="keys" :aria-label="combo">
    <template v-for="(key, i) in combo.split('+')" :key="i">
      <span v-if="i" class="plus" aria-hidden="true">+</span>
      <kbd aria-hidden="true">{{ key }}</kbd>
    </template>
  </span>
</template>
