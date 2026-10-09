import { mount } from 'svelte';
import 'pretendard/dist/web/variable/pretendardvariable-dynamic-subset.css';
// Japanese text (bundled, split by character range so only the glyphs in use are loaded).
import '@fontsource-variable/noto-sans-jp';
import './app.css';
import App from './App.svelte';
import PanelApp from './PanelApp.svelte';
import { ui, type Page } from './lib/state.svelte';

async function start() {
  // Browser preview with sample data while developing the UI outside Tauri.
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    (await import('./lib/mock')).installMock();
    const params = new URLSearchParams(location.search);
    const page = params.get('page');
    if (page) ui.page = page as Page;
    const theme = params.get('theme');
    if (theme) localStorage.setItem('nora.previewTheme', theme);
    const accent = params.get('accent');
    if (accent) localStorage.setItem('nora.previewAccent', accent);
  }
  // The overlay panel window loads this same page; it shows the compact panel UI instead.
  const isPanel = '__TAURI_INTERNALS__' in window
    ? (await import('@tauri-apps/api/window')).getCurrentWindow().label === 'panel'
    : new URLSearchParams(location.search).has('panel');
  mount(isPanel ? PanelApp : App, { target: document.getElementById('app')! });
}

start();
