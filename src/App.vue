<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { t } from './i18n';
import type { IconName } from './icons';
import { load, mica, page, settings, type Page } from './store';
import { history, startHistory } from './history';
import Icon from './components/Icon.vue';
import Keys from './components/Keys.vue';
import TitleBar from './components/TitleBar.vue';
import TranslatePage from './pages/TranslatePage.vue';
import HistoryPage from './pages/HistoryPage.vue';
import ServicePage from './pages/ServicePage.vue';
import SettingsPage from './pages/SettingsPage.vue';

const NAV: { page: Page; icon: IconName; iconOn: IconName }[] = [
  { page: 'translate', icon: 'translate', iconOn: 'translateOn' },
  { page: 'history', icon: 'history', iconOn: 'historyOn' },
  { page: 'service', icon: 'cloud', iconOn: 'cloudOn' },
];
const PAGES = { translate: TranslatePage, history: HistoryPage, service: ServicePage, settings: SettingsPage };

const failed = ref('');

onMounted(async () => {
  // The tray's "Settings" item opens that page.
  void listen<Page>('open-page', (e) => (page.value = e.payload));
  try {
    await Promise.all([load(), startHistory()]);
  } catch (e) {
    failed.value = String(e);
  }
});
</script>

<template>
  <div :class="mica ? 'app-bg mica' : 'app-bg'">
    <TitleBar />
    <div v-if="settings" class="body">
      <nav class="sidebar"  aria-label="QuickTranslate">
        <button
          v-for="item in NAV"
          :key="item.page"
          :class="['nav-item', { active: page === item.page }]"
          :aria-current="page === item.page ? 'page' : undefined"
          @click="page = item.page"
        >
          <span v-if="page === item.page" class="ind" />
          <Icon :name="page === item.page ? item.iconOn : item.icon" />
          <span class="label">{{ t(`nav.${item.page}`) }}</span>
          <span v-if="item.page === 'history' && history.length" class="count">{{ history.length }}</span>
        </button>
        <span class="spacer" />
        <button class="hotkey-chip" @click="page = 'settings'">
          <span class="label"><span class="live" aria-hidden="true" />{{ t('nav.hotkey') }}</span>
          <Keys :combo="settings.hotkey" />
        </button>
        <button
          :class="['nav-item', { active: page === 'settings' }]"
          :aria-current="page === 'settings' ? 'page' : undefined"
          @click="page = 'settings'"
        >
          <span v-if="page === 'settings'" class="ind" />
          <Icon :name="page === 'settings' ? 'settingsOn' : 'settings'" />
          <span class="label">{{ t('nav.settings') }}</span>
        </button>
      </nav>
      <main class="content">
        <Transition name="page" mode="out-in">
          <KeepAlive>
            <component :is="PAGES[page]" :key="page" />
          </KeepAlive>
        </Transition>
      </main>
    </div>
    <div v-else-if="failed" class="body" style="padding: 24px"><p class="note bad">{{ failed }}</p></div>
  </div>
</template>
