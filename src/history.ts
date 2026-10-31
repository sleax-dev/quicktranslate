import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Translation } from './store';

export interface HistoryEntry extends Translation {
  original: string;
  at: number;
}

/** Hotkey translations, newest first, as the Rust side keeps them. */
export const history = ref<HistoryEntry[]>([]);

export async function startHistory() {
  await listen<HistoryEntry>('history', (e) => {
    history.value = [e.payload, ...history.value].slice(0, 50);
  });
  history.value = await invoke<HistoryEntry[]>('get_history');
}

export async function clearHistory() {
  await invoke('clear_history');
  history.value = [];
}
