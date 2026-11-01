import { createApp } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import '@fontsource-variable/inter';
import './style.css';
import App from './App.vue';
import Overlay from './Overlay.vue';

// A desktop app, not a web page: no browser menu on right click (as in Zanrei).
addEventListener('contextmenu', (e) => e.preventDefault());

// The Discord translation overlay is a second window of the same page.
createApp(getCurrentWindow().label === 'overlay' ? Overlay : App).mount('#app');
