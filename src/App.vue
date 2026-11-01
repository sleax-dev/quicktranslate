<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { os, t } from './i18n';
import type { IconName } from './icons';
import { load, mica, page, settings, startUpdates, update, type Page } from './store';
import { history, startHistory } from './history';
import Icon from './components/Icon.vue';
import Keys from './components/Keys.vue';
import TitleBar from './components/TitleBar.vue';
import TranslatePage from './pages/TranslatePage.vue';
import HistoryPage from './pages/HistoryPage.vue';
import DiscordPage from './pages/DiscordPage.vue';
import SettingsPage from './pages/SettingsPage.vue';

interface NavItem { page: Page; icon: IconName; iconOn: IconName }
// Grouped as in Zanrei: the main tools, then the apps QuickTranslate works inside (Discord is Windows-only).
const MAIN: NavItem[] = [
  { page: 'translate', icon: 'translate', iconOn: 'translateOn' },
  { page: 'history', icon: 'history', iconOn: 'historyOn' },
];
const APPS: NavItem[] = os === 'windows' ? [{ page: 'discord', icon: 'chat', iconOn: 'chatOn' }] : [];
const SETTINGS: NavItem = { page: 'settings', icon: 'settings', iconOn: 'settingsOn' };
const PAGES = { translate: TranslatePage, history: HistoryPage, discord: DiscordPage, settings: SettingsPage };

const failed = ref('');

onMounted(async () => {
  // The tray's "Settings" opens that page: in the URL when it opened the window, as an event when it was open.
  const start = location.hash.slice(1) as Page;
  if (start in PAGES) page.value = start;
  void listen<Page>('open-page', (e) => (page.value = e.payload));
  try {
    await Promise.all([load(), startHistory()]);
    startUpdates();
  } catch (e) {
    failed.value = String(e);
  }
  // The window starts hidden and shows itself once there is something to see.
  const win = getCurrentWindow();
  await win.show();
  await win.setFocus();
});
</script>

<template>
  <div :class="mica ? 'app-bg mica' : 'app-bg'">
    <TitleBar />
    <div v-if="settings" class="body">
      <nav class="sidebar" aria-label="QuickTranslate">
        <div class="nav-group">
          <button
            v-for="item in MAIN"
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
        </div>
        <div v-if="APPS.length" class="nav-group">
          <div class="nav-label">{{ t('nav.group.apps') }}</div>
          <button
            v-for="item in APPS"
            :key="item.page"
            :class="['nav-item', { active: page === item.page }]"
            :aria-current="page === item.page ? 'page' : undefined"
            @click="page = item.page"
          >
            <span v-if="page === item.page" class="ind" />
            <Icon :name="page === item.page ? item.iconOn : item.icon" />
            <span class="label">{{ t(`nav.${item.page}`) }}</span>
            <span v-if="settings.discord_send || settings.discord_read" class="on-dot" aria-hidden="true" />
          </button>
        </div>

        <span class="spacer" />
        <button v-if="update.version" class="update-chip" @click="page = 'settings'">
          <Icon name="download" />{{ t('nav.updateTo', { v: update.version }) }}
        </button>
        <button class="hotkey-chip" @click="page = 'settings'">
          <span class="label"><span class="live" aria-hidden="true" />{{ t('nav.hotkey') }}</span>
          <Keys :combo="settings.hotkey" />
        </button>
        <button
          :class="['nav-item', { active: page === SETTINGS.page }]"
          :aria-current="page === SETTINGS.page ? 'page' : undefined"
          @click="page = SETTINGS.page"
        >
          <span v-if="page === SETTINGS.page" class="ind" />
          <Icon :name="page === SETTINGS.page ? SETTINGS.iconOn : SETTINGS.icon" />
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
