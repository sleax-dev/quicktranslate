<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { os, t } from '../i18n';
import Icon from './Icon.vue';

const win = getCurrentWindow();
</script>

<!-- Windows 11 style caption area: drag region, minimize, and close, which hides to the tray (see on_window_event).
     On macOS the native traffic lights sit over the left of this bar (tauri.macos.conf.json), so the own buttons go. -->
<template>
  <div :class="['titlebar', os]" data-tauri-drag-region>
    <span class="brand">
      <img src="/icons/32x32.png" alt="" />
      <span>QuickTranslate</span>
    </span>
    <span class="grow" data-tauri-drag-region />
    <template v-if="os !== 'macos'">
      <button class="tb-btn" :title="t('titlebar.minimize')" :aria-label="t('titlebar.minimize')" @click="win.minimize()">
        <Icon name="minimize" />
      </button>
      <button class="tb-btn close" :title="t('titlebar.close')" :aria-label="t('titlebar.close')" @click="win.close()">
        <Icon name="close" />
      </button>
    </template>
  </div>
</template>
