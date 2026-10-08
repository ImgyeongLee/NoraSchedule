// Stickers: the user's own images placed on top of any page as decoration.
//
// They are click-through (pointer-events: none) except in decorate mode, so they can
// never get in the way of buttons, typing or drag-and-drop. Positions are fractions of
// the page area, so stickers stay in place when the window is resized.
import { api } from './api';
import { importImageFile } from './images';
import type { Page } from './state.svelte';

export interface PlacedSticker {
  id: string;
  image: string;
  page: Page;
  /** Center of the sticker as a fraction (0–1) of the page area. */
  x: number;
  y: number;
  /** Width in pixels; the height follows the image. */
  w: number;
  /** Rotation in degrees. */
  rot: number;
}

const SETTING = 'stickers';
export const MIN_W = 32;
export const MAX_W = 720;
const MAX_LIBRARY = 60;

export const stickers = $state({
  /** Uploaded sticker images, newest first. */
  library: [] as string[],
  placed: [] as PlacedSticker[],
  /** Decorate mode: stickers can be added, moved, resized and removed. */
  editing: false,
  selected: null as string | null,
});

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Number.isFinite(n) ? n : lo));

export async function loadStickers() {
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (!saved || typeof saved !== 'object') return;
    stickers.library = Array.isArray(saved.library) ? saved.library.filter((s: unknown) => typeof s === 'string') : [];
    stickers.placed = (Array.isArray(saved.placed) ? saved.placed : [])
      .filter((p: PlacedSticker) => p && typeof p.image === 'string' && typeof p.page === 'string')
      .map((p: PlacedSticker) => ({
        id: String(p.id),
        image: p.image,
        page: p.page,
        x: clamp(p.x, 0, 1),
        y: clamp(p.y, 0, 1),
        w: clamp(p.w, MIN_W, MAX_W),
        rot: Number.isFinite(p.rot) ? p.rot % 360 : 0,
      }));
  } catch {
    // Corrupt setting: start without stickers.
  }
}

function persist() {
  return api.setSetting(SETTING, JSON.stringify({ library: stickers.library, placed: stickers.placed })).catch(() => {});
}

let timer: ReturnType<typeof setTimeout> | undefined;
/** Saves soon (moving/resizing fires many updates). */
function saveSoon() {
  clearTimeout(timer);
  timer = setTimeout(persist, 400);
}

/** Saves now, then deletes image files nothing uses any more. */
async function saveAndClean() {
  clearTimeout(timer);
  await persist();
  api.removeUnusedImages().catch(() => {});
}

/** Uploads a sticker image into the library (saved right away so it is never cleaned up). */
export async function uploadSticker(file: File): Promise<string> {
  const name = await importImageFile(file, 'sticker');
  stickers.library = [name, ...stickers.library].slice(0, MAX_LIBRARY);
  await persist();
  return name;
}

/** Removes an image from the library and every page it was placed on. */
export async function deleteFromLibrary(image: string) {
  stickers.library = stickers.library.filter((s) => s !== image);
  stickers.placed = stickers.placed.filter((p) => p.image !== image);
  await saveAndClean();
}

let counter = 0;

/** Puts a sticker on `page`, near the middle (slightly offset so repeats don't stack exactly). */
export function placeSticker(image: string, page: Page) {
  const jitter = () => (Math.random() - 0.5) * 0.12;
  const sticker: PlacedSticker = {
    id: `${Date.now().toString(36)}-${(counter++).toString(36)}`,
    image,
    page,
    x: clamp(0.5 + jitter(), 0.05, 0.95),
    y: clamp(0.45 + jitter(), 0.05, 0.95),
    w: 140,
    rot: 0,
  };
  stickers.placed.push(sticker);
  stickers.selected = sticker.id;
  saveSoon();
}

export function updateSticker(id: string, patch: Partial<Pick<PlacedSticker, 'x' | 'y' | 'w' | 'rot'>>) {
  const s = stickers.placed.find((p) => p.id === id);
  if (!s) return;
  if (patch.x !== undefined) s.x = clamp(patch.x, 0, 1);
  if (patch.y !== undefined) s.y = clamp(patch.y, 0, 1);
  if (patch.w !== undefined) s.w = clamp(patch.w, MIN_W, MAX_W);
  if (patch.rot !== undefined) s.rot = patch.rot;
  saveSoon();
}

export function removeSticker(id: string) {
  stickers.placed = stickers.placed.filter((p) => p.id !== id);
  if (stickers.selected === id) stickers.selected = null;
  saveSoon();
}

/** Draws the sticker above the others on its page. */
export function bringToFront(id: string) {
  const i = stickers.placed.findIndex((p) => p.id === id);
  if (i < 0 || i === stickers.placed.length - 1) return;
  const [s] = stickers.placed.splice(i, 1);
  stickers.placed.push(s);
  saveSoon();
}

export function setDecorating(on: boolean) {
  stickers.editing = on;
  stickers.selected = null;
  if (!on) saveAndClean();
}
