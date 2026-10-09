// Window size presets and up to three saved custom sizes (Settings → Window size),
// plus the app-wide zoom level (Settings → Window size → Zoom, ⌘+ / ⌘-).
import { currentMonitor, getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { api } from './api';

export interface WindowSize {
  width: number;
  height: number;
}

/** Must match `minWidth`/`minHeight` in tauri.conf.json. */
const MIN = { width: 640, height: 560 };
export const MAX_SAVED = 3;
const SETTING = 'ui.windowSizes';

async function resizeTo(size: WindowSize) {
  const win = getCurrentWindow();
  if (await win.isMaximized()) await win.unmaximize();
  await win.setSize(new LogicalSize(Math.max(MIN.width, size.width), Math.max(MIN.height, size.height)));
  await win.center();
}

/** Resizes the window to `fraction` of the screen's usable area (without the dock/taskbar). */
export async function applyPreset(fraction: number) {
  const monitor = await currentMonitor();
  if (!monitor) return;
  const area = monitor.workArea?.size ?? monitor.size;
  await resizeTo({
    width: Math.round((area.width / monitor.scaleFactor) * fraction),
    height: Math.round((area.height / monitor.scaleFactor) * fraction),
  });
}

export async function maximize() {
  await getCurrentWindow().maximize();
}

/** The content area's size in logical pixels (what `setSize` restores). Zoom shrinks `innerWidth`, so undo it. */
export const currentSize = (): WindowSize => ({
  width: Math.round(window.innerWidth * zoom.level),
  height: Math.round(window.innerHeight * zoom.level),
});

export const applySaved = (size: WindowSize) => resizeTo(size);

export async function loadSavedSizes(): Promise<WindowSize[]> {
  try {
    const raw = await api.getSetting(SETTING);
    const list = raw ? (JSON.parse(raw) as WindowSize[]) : [];
    return list.filter((s) => s.width > 0 && s.height > 0).slice(0, MAX_SAVED);
  } catch {
    return [];
  }
}

export async function storeSavedSizes(sizes: WindowSize[]) {
  await api.setSetting(SETTING, JSON.stringify(sizes.slice(0, MAX_SAVED))).catch(() => {});
}

// ---- zoom

export const ZOOM_STEPS = [0.7, 0.8, 0.9, 1, 1.1, 1.25, 1.4, 1.6];
const ZOOM_SETTING = 'ui.zoom';

export const zoom = $state({ level: 1 });

const clampZoom = (z: number) => Math.min(ZOOM_STEPS[ZOOM_STEPS.length - 1], Math.max(ZOOM_STEPS[0], z));

/** Scales the whole app like browser zoom; the layout reflows to the new size. */
async function applyZoom(level: number) {
  try {
    await getCurrentWebview().setZoom(level);
  } catch {
    // Browser preview (mocked Tauri): CSS zoom is close enough.
    document.documentElement.style.zoom = String(level);
  }
}

export async function setZoom(level: number) {
  zoom.level = clampZoom(Math.round(level * 100) / 100);
  await applyZoom(zoom.level);
  await api.setSetting(ZOOM_SETTING, String(zoom.level)).catch(() => {});
}

/** One step bigger (`dir` 1) or smaller (-1). */
export function stepZoom(dir: 1 | -1) {
  const next = dir > 0 ? ZOOM_STEPS.find((z) => z > zoom.level + 0.001) : [...ZOOM_STEPS].reverse().find((z) => z < zoom.level - 0.001);
  if (next !== undefined) return setZoom(next);
}

export async function initZoom() {
  const saved = Number(await api.getSetting(ZOOM_SETTING).catch(() => null));
  if (saved > 0 && saved !== 1) {
    zoom.level = clampZoom(saved);
    await applyZoom(zoom.level).catch(() => {});
  }
}
