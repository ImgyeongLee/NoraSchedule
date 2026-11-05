// Calendar preferences (Settings → Calendar), stored in the database so they are backed up.
import { api, type CalEvent } from './api';
import { isAllDayLane } from './dates';
import { broadcastPrefs } from './sync.svelte';

export type TimePicker = 'list' | 'precise';

const SETTING = 'ui.calendar';

export const calPrefs = $state({
  /** List all-day events after timed ones on the same day (multi-day bars stay on top). */
  allDayLast: true,
  /** 'list': times every 15 minutes; 'precise': AM/PM, hour 1–12 and minute 0–59. */
  timePicker: 'list' as TimePicker,
  /** Show the table of upcoming plans under the calendar. */
  upcomingTable: true,
  /** How far ahead that table (and the Overview tile) looks, in months. */
  upcomingMonths: 3,
  /** Include repeating events in it (off: they would crowd out the one-off plans). */
  upcomingRepeat: false,
});

export async function loadCalPrefs() {
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (saved && typeof saved === 'object') {
      if (typeof saved.allDayLast === 'boolean') calPrefs.allDayLast = saved.allDayLast;
      if (saved.timePicker === 'list' || saved.timePicker === 'precise') calPrefs.timePicker = saved.timePicker;
      if (typeof saved.upcomingTable === 'boolean') calPrefs.upcomingTable = saved.upcomingTable;
      if ([1, 3, 6, 12].includes(saved.upcomingMonths)) calPrefs.upcomingMonths = saved.upcomingMonths;
      if (typeof saved.upcomingRepeat === 'boolean') calPrefs.upcomingRepeat = saved.upcomingRepeat;
    }
  } catch {
    // Corrupt setting: keep the defaults.
  }
}

export function setCalPrefs(patch: Partial<typeof calPrefs>) {
  Object.assign(calPrefs, patch);
  api.setSetting(SETTING, JSON.stringify(calPrefs))
    .catch(() => {})
    .then(broadcastPrefs);
}

/** A day's events in display order: timed by start time, all-day ones last (or first). */
export function sortDayEvents(list: CalEvent[]): CalEvent[] {
  const lane = (e: CalEvent) => (isAllDayLane(e) ? (calPrefs.allDayLast ? 1 : -1) : 0);
  return [...list].sort((a, b) => lane(a) - lane(b) || a.start.localeCompare(b.start));
}
