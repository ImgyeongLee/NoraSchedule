import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
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
  mount(App, { target: document.getElementById('app')! });
}

start();
