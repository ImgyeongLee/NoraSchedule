// App-wide reactive state: navigation, theme, toasts and a data version counter
// that pages watch to reload after any change.
import { api } from './api';
import { fromHex } from './colors';
import { broadcastPrefs } from './sync.svelte';
import { CUSTOM_VAR_NAMES, customThemeVars, DEFAULT_CUSTOM_THEME, parseCustomTheme, type CustomTheme } from './customTheme';

export type Page =
  | 'home' | 'calendar' | 'ddays' | 'todos' | 'pomodoro' | 'memos' | 'bookmarks' | 'expenses' | 'health' | 'reading' | 'trpg' | 'tracking'
  | 'analytics' | 'settings';
export type ThemePref = 'light' | 'dark' | 'system';
export type Accent = 'default' | 'mono' | 'pink' | 'blue' | 'green' | 'brown' | 'custom';
/** Built-in color themes; 'custom' is the user's own (see customTheme.ts). */
export type PresetAccent = Exclude<Accent, 'custom'>;
export const ACCENTS: PresetAccent[] = ['default', 'mono', 'pink', 'blue', 'green', 'brown'];

export const ui = $state({
  page: 'home' as Page,
  theme: 'system' as ThemePref,
  accent: 'default' as Accent,
  customTheme: { ...DEFAULT_CUSTOM_THEME } as CustomTheme,
  /** The user's choice; the sidebar also collapses on its own in narrow windows. */
  sidebarCollapsed: false,
  /** Date the Calendar page should open on (set by Overview tiles). */
  calendarFocus: null as string | null,
  /** Memo the Memos page should open (set by Overview tiles). */
  memoFocus: null as number | null,
  toast: null as null | { text: string; kind: 'info' | 'error' | 'success'; id: number; action?: ToastAction },
});

/** Bumped after every successful write; pages re-fetch when it changes. */
export const data = $state({ version: 0 });

let toastTimer: ReturnType<typeof setTimeout> | undefined;

export interface ToastAction {
  label: string;
  run: () => void;
}

export function toast(text: string, kind: 'info' | 'error' | 'success' = 'info', action?: ToastAction) {
  ui.toast = { text, kind, id: Date.now(), action };
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (ui.toast = null), kind === 'error' || action ? 6000 : 3200);
}

/** Runs a write, bumps the data version, and reports failures as a toast. */
export async function mutate<T>(p: Promise<T>, success?: string): Promise<T | undefined> {
  try {
    const result = await p;
    data.version++;
    if (success) toast(success, 'success');
    return result;
  } catch (e) {
    toast(String(e), 'error');
    return undefined;
  }
}

/** Runs a read, reporting failures as a toast. */
export async function load<T>(p: Promise<T>, fallback: T): Promise<T> {
  try {
    return await p;
  } catch (e) {
    toast(String(e), 'error');
    return fallback;
  }
}

const darkQuery = window.matchMedia('(prefers-color-scheme: dark)');

export function applyTheme() {
  const dark = ui.theme === 'dark' || (ui.theme === 'system' && darkQuery.matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  document.documentElement.dataset.accent = ui.accent;
  const root = document.documentElement.style;
  if (ui.accent === 'custom') {
    for (const [name, value] of Object.entries(customThemeVars(ui.customTheme, dark))) root.setProperty(name, value);
  } else {
    for (const name of CUSTOM_VAR_NAMES) root.removeProperty(name);
  }
  // Match the native title bar (Windows) to the themed app background.
  const css = getComputedStyle(document.documentElement);
  const color = (name: string) => fromHex(css.getPropertyValue(name).trim());
  const caption = color('--bg');
  const text = color('--text');
  if (!Number.isNaN(caption) && !Number.isNaN(text)) api.setTitlebarColors(caption, text, dark).catch(() => {});
}

export async function setAccent(accent: Accent) {
  ui.accent = accent;
  applyTheme();
  await api.setSetting('ui.accent', accent).catch(() => {});
  broadcastPrefs();
}

let customSaveTimer: ReturnType<typeof setTimeout> | undefined;

/** Switches to (and updates) the user's own color theme. Saving is debounced for live color pickers. */
export function setCustomTheme(theme: CustomTheme) {
  ui.customTheme = theme;
  ui.accent = 'custom';
  applyTheme();
  clearTimeout(customSaveTimer);
  customSaveTimer = setTimeout(async () => {
    await Promise.all([
      api.setSetting('ui.customTheme', JSON.stringify(theme)).catch(() => {}),
      api.setSetting('ui.accent', 'custom').catch(() => {}),
    ]);
    broadcastPrefs();
  }, 400);
}

export async function setTheme(theme: ThemePref) {
  ui.theme = theme;
  applyTheme();
  await api.setSetting('ui.theme', theme).catch(() => {});
  broadcastPrefs();
}

export async function initTheme() {
  const saved = (await api.getSetting('ui.theme').catch(() => null)) ?? localStorage.getItem('nora.previewTheme');
  if (saved === 'light' || saved === 'dark' || saved === 'system') ui.theme = saved;
  const accent = (await api.getSetting('ui.accent').catch(() => null)) ?? localStorage.getItem('nora.previewAccent');
  ui.customTheme = parseCustomTheme(await api.getSetting('ui.customTheme').catch(() => null));
  if (accent === 'custom' || ACCENTS.includes(accent as PresetAccent)) ui.accent = accent as Accent;
  applyTheme();
  darkQuery.addEventListener('change', applyTheme);
}

export async function setSidebarCollapsed(collapsed: boolean) {
  ui.sidebarCollapsed = collapsed;
  await api.setSetting('ui.sidebarCollapsed', String(collapsed)).catch(() => {});
}

export async function initSidebar() {
  ui.sidebarCollapsed = (await api.getSetting('ui.sidebarCollapsed').catch(() => null)) === 'true';
}

/** Opens the Calendar page on a given day. */
export function openCalendar(day: string) {
  ui.calendarFocus = day;
  ui.page = 'calendar';
}
