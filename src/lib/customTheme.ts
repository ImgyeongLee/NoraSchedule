// The user's own color theme (Settings → Color theme → Custom). Three choices — a main
// color, a second accent and how much the background is tinted — expand into the full set
// of CSS variables, with separate light and dark variants so both stay readable.
import { mix, hex, textOn } from './colors';

export interface CustomTheme {
  /** Buttons, highlights, today marker. 0xRRGGBB. */
  primary: number;
  /** Second accent for gradients (logo, progress bars, greeting). 0xRRGGBB. */
  accent2: number;
  /** How strongly the main color tints the background: 0 (none) to 3 (strong). */
  tint: number;
}

export const DEFAULT_CUSTOM_THEME: CustomTheme = { primary: 0x6c63ff, accent2: 0xb084f6, tint: 1 };
export const TINT_LEVELS = [0, 1, 2, 3] as const;
const TINT_WEIGHT = [0, 0.04, 0.08, 0.14];

// The built-in backgrounds the tint is mixed into (see :root in app.css).
const BASE = {
  light: { bg: 0xf4f5fa, sidebar: 0xfbfbfe },
  dark: { bg: 0x131419, sidebar: 0x17181f, surface: 0x1c1e26 },
};
const WHITE = 0xffffff;
const BLACK = 0x000000;

/** The custom properties to set on <html> for `theme` in light or dark mode. */
export function customThemeVars(theme: CustomTheme, dark: boolean): Record<string, string> {
  const tint = TINT_WEIGHT[Math.min(3, Math.max(0, Math.round(theme.tint)))];
  if (dark) {
    // Lift the colors a little so they read well on dark surfaces.
    const primary = mix(theme.primary, WHITE, 0.85);
    return {
      '--primary': hex(primary),
      '--primary-hover': hex(mix(primary, WHITE, 0.85)),
      '--primary-soft': hex(mix(primary, BASE.dark.surface, 0.22)),
      '--primary-text': textOn(primary),
      '--accent-2': hex(mix(theme.accent2, WHITE, 0.85)),
      '--bg': hex(mix(theme.primary, BASE.dark.bg, tint)),
      '--sidebar': hex(mix(theme.primary, BASE.dark.sidebar, tint * 0.8)),
    };
  }
  return {
    '--primary': hex(theme.primary),
    '--primary-hover': hex(mix(theme.primary, BLACK, 0.88)),
    '--primary-soft': hex(mix(theme.primary, WHITE, 0.12)),
    '--primary-text': textOn(theme.primary),
    '--accent-2': hex(theme.accent2),
    '--bg': hex(mix(theme.primary, BASE.light.bg, tint)),
    '--sidebar': hex(mix(theme.primary, BASE.light.sidebar, tint * 0.6)),
  };
}

export const CUSTOM_VAR_NAMES = Object.keys(customThemeVars(DEFAULT_CUSTOM_THEME, false));

export function parseCustomTheme(raw: string | null): CustomTheme {
  try {
    const v = JSON.parse(raw ?? '');
    const num = (x: unknown, fallback: number) => (typeof x === 'number' && Number.isFinite(x) ? x : fallback);
    return {
      primary: num(v.primary, DEFAULT_CUSTOM_THEME.primary) & 0xffffff,
      accent2: num(v.accent2, DEFAULT_CUSTOM_THEME.accent2) & 0xffffff,
      tint: Math.min(3, Math.max(0, Math.round(num(v.tint, DEFAULT_CUSTOM_THEME.tint)))),
    };
  } catch {
    return { ...DEFAULT_CUSTOM_THEME };
  }
}
