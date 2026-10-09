// Keeps the main window and the overlay panel in step. Each window has its own copy of
// the app state, so when one saves something it tells the other to reload, and when
// appearance or language settings change the other window re-reads them.
import { emit, listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const DATA = 'nora://data';
const PREFS = 'nora://prefs';

const inTauri = () => '__TAURI_INTERNALS__' in window;
const label = () => (inTauri() ? getCurrentWindow().label : 'preview');

/**
 * Starts syncing. `data.version` is bumped by every write in this window (see state.svelte.ts);
 * `onPrefs` reloads theme, language and calendar preferences.
 */
export function initSync(data: { version: number }, onRemoteData: () => void, onPrefs: () => void) {
  if (!inTauri()) return;
  const me = label();
  let seen = data.version;
  let fromRemote = false;
  $effect.root(() => {
    $effect(() => {
      const v = data.version;
      if (v === seen) return;
      seen = v;
      // A reload we did because of the other window must not be sent back to it.
      if (fromRemote) {
        fromRemote = false;
        return;
      }
      emit(DATA, { from: me }).catch(() => {});
    });
  });
  listen<{ from: string }>(DATA, (e) => {
    if (e.payload?.from === me) return;
    fromRemote = true;
    data.version++;
    onRemoteData();
  });
  listen<{ from: string }>(PREFS, (e) => {
    if (e.payload?.from !== me) onPrefs();
  });
}

/** Tells the other window that appearance, language or calendar preferences changed. */
export function broadcastPrefs() {
  if (inTauri()) emit(PREFS, { from: label() }).catch(() => {});
}
