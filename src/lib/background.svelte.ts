// A picture behind every page (Settings → Background image). A veil of the theme's background
// color sits over it, so text stays readable in both light and dark mode.
import { api } from './api';

const SETTING = 'ui.background';

export const background = $state({
  image: null as string | null,
  /** How much the theme color covers the picture, 0–90 (%). */
  veil: 45,
  /** Blur in pixels, 0–20. */
  blur: 0,
});

const clamp = (n: unknown, lo: number, hi: number, fallback: number) =>
  typeof n === 'number' && Number.isFinite(n) ? Math.min(hi, Math.max(lo, n)) : fallback;

export async function loadBackground() {
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (!saved || typeof saved !== 'object') return;
    background.image = typeof saved.image === 'string' ? saved.image : null;
    background.veil = clamp(saved.veil, 0, 90, 45);
    background.blur = clamp(saved.blur, 0, 20, 0);
  } catch {
    /* no background */
  }
}

let timer: ReturnType<typeof setTimeout> | undefined;
/** Saves soon (sliders fire many updates); a removed picture is then cleaned up. */
export function saveBackground() {
  clearTimeout(timer);
  timer = setTimeout(async () => {
    await api.setSetting(SETTING, JSON.stringify(background)).catch(() => {});
    api.removeUnusedImages().catch(() => {});
  }, 300);
}
