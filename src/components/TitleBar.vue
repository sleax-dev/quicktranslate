<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { t } from '../i18n';
import Icon from './Icon.vue';

const win = getCurrentWindow();
</script>

<!-- Windows 11 style caption area: drag region, minimize, and close, which hides to the tray (see on_window_event). -->
<template>
  <div class="titlebar" data-tauri-drag-region>
    <span class="brand">
      <img src="/icons/32x32.png" alt="" />
      <span>QuickTranslate</span>
    </span>
    <span class="grow" data-tauri-drag-region />
    <button class="tb-btn" :title="t('titlebar.minimize')" :aria-label="t('titlebar.minimize')" @click="win.minimize()">
      <Icon name="minimize" />
    </button>
    <button class="tb-btn close" :title="t('titlebar.close')" :aria-label="t('titlebar.close')" @click="win.close()">
      <Icon name="close" />
    </button>
  </div>
</template>
