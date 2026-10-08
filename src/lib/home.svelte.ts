// Overview dashboard: which tiles are shown, in what order and size.
import type { Component } from 'svelte';
import {
  Activity, Bookmark, CalendarCheck, CalendarDays, CalendarHeart, ChartColumn, Hand, ListTodo, NotebookPen, Target, Timer,
  Trophy, Wallet,
} from '@lucide/svelte';
import { api } from './api';
import type { Key } from './i18n.svelte';

export type TileSize = 'sm' | 'wide' | 'tall' | 'large';
export type WidgetId =
  | 'greeting' | 'agenda' | 'calendar' | 'todos' | 'ddays' | 'dday' | 'pomodoro' | 'working' | 'weekChart' | 'progress'
  | 'memos' | 'bookmarks' | 'expenses';

export interface Tile {
  /** Unique per tile, so the same widget can appear more than once. */
  uid: string;
  id: WidgetId;
  size: TileSize;
  /** "D-Day card" tiles: which D-Day to show (null = the next upcoming one). */
  ddayId?: number | null;
}

type IconComponent = Component<{ size?: number }>;

export const WIDGETS: Record<WidgetId, { name: Key; desc: Key; icon: IconComponent; defaultSize: TileSize; multiple?: boolean }> = {
  greeting: { name: 'w.greeting', desc: 'w.greeting.desc', icon: Hand, defaultSize: 'wide' },
  agenda: { name: 'w.agenda', desc: 'w.agenda.desc', icon: CalendarCheck, defaultSize: 'tall' },
  calendar: { name: 'w.calendar', desc: 'w.calendar.desc', icon: CalendarDays, defaultSize: 'tall' },
  todos: { name: 'w.todos', desc: 'w.todos.desc', icon: ListTodo, defaultSize: 'tall' },
  ddays: { name: 'w.ddays', desc: 'w.ddays.desc', icon: Target, defaultSize: 'sm' },
  dday: { name: 'w.dday', desc: 'w.dday.desc', icon: CalendarHeart, defaultSize: 'sm', multiple: true },
  pomodoro: { name: 'w.pomodoro', desc: 'w.pomodoro.desc', icon: Timer, defaultSize: 'sm' },
  working: { name: 'w.working', desc: 'w.working.desc', icon: Activity, defaultSize: 'sm' },
  weekChart: { name: 'w.weekChart', desc: 'w.weekChart.desc', icon: ChartColumn, defaultSize: 'wide' },
  progress: { name: 'w.progress', desc: 'w.progress.desc', icon: Trophy, defaultSize: 'sm' },
  memos: { name: 'w.memos', desc: 'w.memos.desc', icon: NotebookPen, defaultSize: 'sm' },
  bookmarks: { name: 'w.bookmarks', desc: 'w.bookmarks.desc', icon: Bookmark, defaultSize: 'tall' },
  expenses: { name: 'w.expenses', desc: 'w.expenses.desc', icon: Wallet, defaultSize: 'sm' },
};

export const SIZES: { id: TileSize; label: Key }[] = [
  { id: 'sm', label: 'home.sizeSm' },
  { id: 'wide', label: 'home.sizeWide' },
  { id: 'tall', label: 'home.sizeTall' },
  { id: 'large', label: 'home.sizeLarge' },
];

let uidCounter = 0;
const newUid = () => `${Date.now().toString(36)}-${(uidCounter++).toString(36)}`;

const DEFAULT_LAYOUT: Omit<Tile, 'uid'>[] = [
  { id: 'greeting', size: 'wide' },
  { id: 'agenda', size: 'tall' },
  { id: 'todos', size: 'tall' },
  { id: 'ddays', size: 'sm' },
  { id: 'pomodoro', size: 'sm' },
  { id: 'weekChart', size: 'wide' },
  { id: 'calendar', size: 'tall' },
  { id: 'working', size: 'sm' },
  { id: 'progress', size: 'sm' },
  { id: 'memos', size: 'sm' },
];

const SETTING = 'home.layout';

export const home = $state({ tiles: [] as Tile[], loaded: false });

export async function loadLayout() {
  const saved = await api.getSetting(SETTING).catch(() => null);
  let tiles: Omit<Tile, 'uid'>[] = DEFAULT_LAYOUT;
  if (saved) {
    try {
      const parsed = JSON.parse(saved) as Tile[];
      tiles = parsed.filter((t) => t.id in WIDGETS && SIZES.some((s) => s.id === t.size));
    } catch {
      // Corrupt setting: fall back to the default layout.
    }
  }
  // Layouts saved by older versions have no uid.
  home.tiles = tiles.map((t) => ({ uid: (t as Tile).uid ?? newUid(), ...t }));
  home.loaded = true;
}

export function saveLayout() {
  api.setSetting(SETTING, JSON.stringify(home.tiles)).catch(() => {});
}

export function resetLayout() {
  home.tiles = DEFAULT_LAYOUT.map((t) => ({ uid: newUid(), ...t }));
  saveLayout();
}

export function addTile(id: WidgetId, ddayId: number | null = null) {
  home.tiles.push({ uid: newUid(), id, size: WIDGETS[id].defaultSize, ...(id === 'dday' ? { ddayId } : {}) });
  saveLayout();
}

export function setTileDday(index: number, ddayId: number | null) {
  home.tiles[index].ddayId = ddayId;
  saveLayout();
}

export function removeTile(index: number) {
  home.tiles.splice(index, 1);
  saveLayout();
}

export function moveTile(from: number, to: number) {
  if (from === to || to < 0 || to >= home.tiles.length) return;
  const [tile] = home.tiles.splice(from, 1);
  home.tiles.splice(to, 0, tile);
}

export function setTileSize(index: number, size: TileSize) {
  home.tiles[index].size = size;
  saveLayout();
}
