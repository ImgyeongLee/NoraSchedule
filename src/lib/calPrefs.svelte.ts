// Calendar preferences (Settings → Calendar), stored in the database so they are backed up.
import { api, type CalEvent } from './api';
import { isAllDayLane } from './dates';

export type TimePicker = 'list' | 'precise';

const SETTING = 'ui.calendar';

export const calPrefs = $state({
  /** List all-day events after timed ones on the same day (multi-day bars stay on top). */
  allDayLast: true,
  /** 'list': times every 15 minutes; 'precise': AM/PM, hour 1–12 and minute 0–59. */
  timePicker: 'list' as TimePicker,
});

export async function loadCalPrefs() {
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (saved && typeof saved === 'object') {
      if (typeof saved.allDayLast === 'boolean') calPrefs.allDayLast = saved.allDayLast;
      if (saved.timePicker === 'list' || saved.timePicker === 'precise') calPrefs.timePicker = saved.timePicker;
    }
  } catch {
    // Corrupt setting: keep the defaults.
  }
}

export function setCalPrefs(patch: Partial<typeof calPrefs>) {
  Object.assign(calPrefs, patch);
  api.setSetting(SETTING, JSON.stringify(calPrefs)).catch(() => {});
}

/** A day's events in display order: timed by start time, all-day ones last (or first). */
export function sortDayEvents(list: CalEvent[]): CalEvent[] {
  const lane = (e: CalEvent) => (isAllDayLane(e) ? (calPrefs.allDayLast ? 1 : -1) : 0);
  return [...list].sort((a, b) => lane(a) - lane(b) || a.start.localeCompare(b.start));
}
