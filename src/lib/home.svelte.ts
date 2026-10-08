// Overview dashboard: which tiles are shown, in what order and size.
import type { Component } from 'svelte';
import {
  Activity, Bookmark, CalendarCheck, CalendarDays, CalendarHeart, ChartColumn, Hand, ListTodo, NotebookPen, Target, Timer,
  Image as ImageIcon, Trophy, Wallet,
} from '@lucide/svelte';
import { api } from './api';
import { cleanFraming, DEFAULT_FRAMING, type Framing } from './framing';
import type { Key } from './i18n.svelte';

export type TileSize = 'sm' | 'wide' | 'tall' | 'large';
export type WidgetId =
  | 'greeting' | 'agenda' | 'calendar' | 'todos' | 'ddays' | 'dday' | 'pomodoro' | 'working' | 'weekChart' | 'progress'
  | 'memos' | 'bookmarks' | 'expenses' | 'image';

export interface Tile {
  /** Unique per tile, so the same widget can appear more than once. */
  uid: string;
  id: WidgetId;
  size: TileSize;
  /** "D-Day card" tiles: which D-Day to show (null = the next upcoming one). */
  ddayId?: number | null;
  /** Image cards: the picture and how it is framed. */
  image?: string | null;
  framing?: Framing;
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
  image: { name: 'w.image', desc: 'w.image.desc', icon: ImageIcon, defaultSize: 'sm', multiple: true },
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

export type HeaderHeight = 'sm' | 'md' | 'lg';
export const HEADER_HEIGHTS: Record<HeaderHeight, number> = { sm: 140, md: 200, lg: 280 };

export interface HomeHeader {
  image: string | null;
  framing: Framing;
  height: HeaderHeight;
}

const HEADER_SETTING = 'home.header';

export const home = $state({
  tiles: [] as Tile[],
  loaded: false,
  header: { image: null, framing: { ...DEFAULT_FRAMING }, height: 'md' } as HomeHeader,
});

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
  await loadHeader();
}

async function loadHeader() {
  try {
    const saved = JSON.parse((await api.getSetting(HEADER_SETTING).catch(() => null)) ?? 'null');
    if (saved && typeof saved === 'object') {
      home.header = {
        image: typeof saved.image === 'string' ? saved.image : null,
        framing: cleanFraming(saved.framing),
        height: saved.height in HEADER_HEIGHTS ? saved.height : 'md',
      };
    }
  } catch {
    // Corrupt setting: no header.
  }
}

/** Saves the header; an image that is no longer used is cleaned up afterwards. */
export async function setHeader(header: HomeHeader) {
  home.header = header;
  await api.setSetting(HEADER_SETTING, JSON.stringify(header)).catch(() => {});
  api.removeUnusedImages().catch(() => {});
}

export function saveLayout() {
  return api.setSetting(SETTING, JSON.stringify(home.tiles)).catch(() => {});
}

/** Image cards: sets the picture and its framing (by tile uid). */
export async function setTileImage(uid: string, image: string | null, framing: Framing) {
  const tile = home.tiles.find((t) => t.uid === uid);
  if (!tile) return;
  tile.image = image;
  tile.framing = framing;
  await saveLayout();
  api.removeUnusedImages().catch(() => {});
}

export async function resetLayout() {
  home.tiles = DEFAULT_LAYOUT.map((t) => ({ uid: newUid(), ...t }));
  await saveLayout();
  api.removeUnusedImages().catch(() => {});
}

export function addTile(id: WidgetId, ddayId: number | null = null) {
  home.tiles.push({ uid: newUid(), id, size: WIDGETS[id].defaultSize, ...(id === 'dday' ? { ddayId } : {}) });
  saveLayout();
}

export function setTileDday(index: number, ddayId: number | null) {
  home.tiles[index].ddayId = ddayId;
  saveLayout();
}

export async function removeTile(index: number) {
  const [removed] = home.tiles.splice(index, 1);
  await saveLayout();
  if (removed?.image) api.removeUnusedImages().catch(() => {});
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
