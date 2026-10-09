// Sidebar pages: the order they appear in and which ones are turned off (Settings → Pages).
// Turning a page off only hides it from the sidebar; its data stays in the database.
import type { Component } from 'svelte';
import {
  Activity, Bookmark, CalendarDays, ChartColumn, Dices, House, ListTodo, NotebookPen, Target, Timer, Wallet,
} from '@lucide/svelte';
import { api } from './api';
import type { Key } from './i18n.svelte';
import { ui, type Page } from './state.svelte';

export type NavPage = Exclude<Page, 'settings'>;

export const PAGES: { id: NavPage; label: Key; icon: Component<{ size?: number }> }[] = [
  { id: 'home', label: 'nav.home', icon: House },
  { id: 'calendar', label: 'nav.calendar', icon: CalendarDays },
  { id: 'ddays', label: 'nav.ddays', icon: Target },
  { id: 'todos', label: 'nav.todos', icon: ListTodo },
  { id: 'pomodoro', label: 'nav.pomodoro', icon: Timer },
  { id: 'memos', label: 'nav.memos', icon: NotebookPen },
  { id: 'bookmarks', label: 'nav.bookmarks', icon: Bookmark },
  { id: 'expenses', label: 'nav.expenses', icon: Wallet },
  { id: 'trpg', label: 'nav.trpg', icon: Dices },
  { id: 'tracking', label: 'nav.tracking', icon: Activity },
  { id: 'analytics', label: 'nav.analytics', icon: ChartColumn },
];

const SETTING = 'ui.pages';
const DEFAULT_ORDER = PAGES.map((p) => p.id);

export const pagePrefs = $state({
  order: [...DEFAULT_ORDER] as NavPage[],
  hidden: [] as NavPage[],
});

const known = (id: unknown): id is NavPage => DEFAULT_ORDER.includes(id as NavPage);
const pageInfo = (id: NavPage) => PAGES.find((p) => p.id === id)!;

/** Every page in the user's order (hidden ones included), for Settings. */
export const orderedPages = () => pagePrefs.order.map(pageInfo);

/** The pages shown in the sidebar, in order. */
export const visiblePages = () => orderedPages().filter((p) => !pagePrefs.hidden.includes(p.id));

export const isPageVisible = (id: Page) => id === 'settings' || !pagePrefs.hidden.includes(id);

export async function initPages() {
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (saved && Array.isArray(saved.order)) {
      const order = [...new Set((saved.order as unknown[]).filter(known))];
      // Pages added in a later version go after the user's own order.
      pagePrefs.order = [...order, ...DEFAULT_ORDER.filter((id) => !order.includes(id))];
      pagePrefs.hidden = Array.isArray(saved.hidden) ? (saved.hidden as unknown[]).filter(known) : [];
    }
  } catch {
    // Corrupt setting: keep the defaults.
  }
  leaveHiddenPage();
}

/** If the open page was turned off, go to the first visible one. */
function leaveHiddenPage() {
  if (!isPageVisible(ui.page)) ui.page = visiblePages()[0]?.id ?? 'settings';
}

async function save() {
  await api.setSetting(SETTING, JSON.stringify({ order: pagePrefs.order, hidden: pagePrefs.hidden })).catch(() => {});
}

/** Moves the page at `from` to position `to` in the order. */
export function movePage(from: number, to: number) {
  if (from === to || to < 0 || to >= pagePrefs.order.length) return;
  const order = [...pagePrefs.order];
  const [id] = order.splice(from, 1);
  order.splice(to, 0, id);
  pagePrefs.order = order;
  save();
}

/** Shows or hides a page. The last visible page cannot be hidden. */
export function setPageVisible(id: NavPage, visible: boolean) {
  if (!visible && visiblePages().length <= 1) return;
  pagePrefs.hidden = visible ? pagePrefs.hidden.filter((x) => x !== id) : [...pagePrefs.hidden, id];
  leaveHiddenPage();
  save();
}

export function resetPages() {
  pagePrefs.order = [...DEFAULT_ORDER];
  pagePrefs.hidden = [];
  save();
}
