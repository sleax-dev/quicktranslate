<script setup lang="ts">
import { nextTick, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';

/** One translation over its Discord message, in physical pixels from the overlay's corner. */
interface Item {
  key: string;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
  bg: string;
  fg: string;
}

const items = ref<Item[]>([]);
const boxes = ref<HTMLElement[]>([]);
const px = (n: number) => `${n / window.devicePixelRatio}px`;

void listen<Item[]>('overlay', async (e) => {
  items.value = e.payload;
  await nextTick();
  // A translation longer than the original shrinks until it fits over it.
  for (const box of boxes.value) {
    let size = 15;
    box.style.fontSize = `${size}px`;
    while (box.scrollHeight > box.clientHeight + 1 && size > 10) box.style.fontSize = `${--size}px`;
  }
});
</script>

<template>
  <div
    v-for="i in items"
    :key="i.key"
    ref="boxes"
    class="qt-overlay-item"
    :style="{ left: px(i.x), top: px(i.y), width: px(i.w), height: px(i.h), background: i.bg, color: i.fg }"
  >{{ i.text }}</div>
</template>

<style>
html, body, #app { margin: 0; background: transparent !important; overflow: hidden; }
.qt-overlay-item {
  position: fixed;
  overflow: hidden;
  font-family: 'gg sans', 'Segoe UI', sans-serif;
  line-height: 1.375;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
