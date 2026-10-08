// Window size presets and up to three saved custom sizes (Settings → Window size).
import { currentMonitor, getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
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

/** The content area's size in logical pixels (what `setSize` restores). */
export const currentSize = (): WindowSize => ({ width: window.innerWidth, height: window.innerHeight });

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
