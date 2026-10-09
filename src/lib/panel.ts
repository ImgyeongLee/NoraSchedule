// Overlay panel (see src-tauri/src/panel.rs): a small always-available window with a few
// Overview widgets. These wrap its commands and the choice of widgets it shows.
import { invoke } from '@tauri-apps/api/core';
import { api } from './api';
import type { WidgetId } from './home.svelte';
import { PAGES } from './pages.svelte';
import type { Page } from './state.svelte';

export interface PanelStatus {
  open: boolean;
  /** Opens when the app starts. */
  enabled: boolean;
  on_top: boolean;
  /** Starts (as the panel) when the user signs in to Windows. */
  autostart: boolean;
  /** Folded up to its top bar. */
  collapsed: boolean;
}

export type Corner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

export const panelApi = {
  status: () => invoke<PanelStatus>('panel_status'),
  setEnabled: (enabled: boolean) => invoke<void>('panel_set_enabled', { enabled }),
  setOnTop: (on: boolean) => invoke<void>('panel_set_on_top', { on }),
  /** Folds the panel up to its top bar (or unfolds it). */
  setCollapsed: (collapsed: boolean) => invoke<void>('panel_set_collapsed', { collapsed }),
  place: (corner: Corner) => invoke<void>('panel_place', { corner }),
  /** Brings the main window forward, optionally on a page (and a calendar day or memo). */
  showMain: (page?: Page, day?: string | null, memo?: number | null) =>
    invoke<void>('panel_show_main', { page: page ?? null, day: day ?? null, memo: memo ?? null }),
  setAutostart: (enabled: boolean) => invoke<boolean>('set_autostart', { enabled }),
};

/** Pomodoro keeps its timer in the window that runs it, and image cards belong to an
 * Overview tile, so neither is offered in the panel. */
export const PANEL_WIDGETS: WidgetId[] = [
  'agenda', 'todos', 'calendar', 'ddays', 'dday', 'greeting', 'memos', 'working', 'weekChart', 'progress',
  'bookmarks', 'expenses', 'health',
];
export const DEFAULT_PANEL_WIDGETS: WidgetId[] = ['agenda', 'todos', 'calendar'];
const WIDGETS_SETTING = 'panel.widgets';

export async function loadPanelWidgets(): Promise<WidgetId[]> {
  try {
    const saved = JSON.parse((await api.getSetting(WIDGETS_SETTING).catch(() => null)) ?? 'null');
    if (Array.isArray(saved)) return saved.filter((id): id is WidgetId => PANEL_WIDGETS.includes(id));
  } catch {
    // Corrupt setting: use the default set.
  }
  return [...DEFAULT_PANEL_WIDGETS];
}

export const savePanelWidgets = (ids: WidgetId[]) => api.setSetting(WIDGETS_SETTING, JSON.stringify(ids));

/** Only real pages may be opened from a navigation message. */
export const isPage = (p: unknown): p is Page => p === 'settings' || PAGES.some((x) => x.id === p);
