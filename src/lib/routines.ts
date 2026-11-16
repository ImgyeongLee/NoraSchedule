// When a routine comes back: the same rules as src-tauri/src/routines.rs, for previews
// ("next: Mon 30") and the browser preview.
import type { DateStr, Routine, RoutineFreq } from './api';
import { addDays, diffDays, parseYmd } from './dates';

export const FREQS: RoutineFreq[] = ['weekly', 'biweekly', 'monthly'];

/** Monday of the week `day` is in. */
const weekOf = (day: DateStr) => addDays(day, -((parseYmd(day).getDay() + 6) % 7));

export function routineDue(r: Routine, day: DateStr): boolean {
  if (!r.active) return false;
  const date = parseYmd(day);
  const onWeekday = !r.weekdays.length || r.weekdays.includes(date.getDay());
  const freq = r.freq ?? 'weekly';
  if (freq === 'monthly') {
    const d = date.getDate();
    const last = new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
    return (r.month_days ?? []).some((m) => m === d || (m > last && d === last));
  }
  if (freq === 'biweekly') {
    const weeks = Math.round(diffDays(weekOf(day), weekOf(r.start ?? day)) / 7);
    return onWeekday && ((weeks % 2) + 2) % 2 === 0;
  }
  return onWeekday;
}

/** The next `count` days the routine comes back on, from `from` on (within a year). */
export function nextDates(r: Routine, from: DateStr, count = 3): DateStr[] {
  const out: DateStr[] = [];
  for (let i = 0; i < 400 && out.length < count; i++) {
    const day = addDays(from, i);
    if (routineDue({ ...r, active: true }, day)) out.push(day);
  }
  return out;
}
