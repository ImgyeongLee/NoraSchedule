// Todo deadlines: a date plus an optional time.
import type { DateStr, Todo } from './api';
import { addDays, fmt, parseYmd, today } from './dates';
import { t } from './i18n.svelte';

/** The moment the deadline passes: the given time, or the end of the day. */
function deadlineMs(due: DateStr, time: string | null): number {
  const d = parseYmd(due);
  if (time) {
    const [h, m] = time.split(':').map(Number);
    d.setHours(h, m, 0, 0);
  } else {
    d.setHours(23, 59, 59, 999);
  }
  return d.getTime();
}

export type DeadlineState = 'overdue' | 'today' | 'soon' | 'later';

export function deadlineState(todo: Pick<Todo, 'due' | 'due_time' | 'done'>, now = Date.now()): DeadlineState | null {
  if (!todo.due) return null;
  if (!todo.done && deadlineMs(todo.due, todo.due_time) < now) return 'overdue';
  const d = today();
  if (todo.due === d) return 'today';
  if (todo.due <= addDays(d, 2)) return 'soon';
  return 'later';
}

export function isOverdue(todo: Pick<Todo, 'due' | 'due_time' | 'done'>): boolean {
  return deadlineState(todo) === 'overdue';
}

/** "Today 18:00", "Tomorrow", "Oct 12 09:30", … */
export function fmtDeadline(due: DateStr, time: string | null): string {
  const d = today();
  const day = due === d ? t('dl.today') : due === addDays(d, 1) ? t('dl.tomorrow') : fmt(due, { month: 'short', day: 'numeric' });
  return time ? `${day} ${time}` : day;
}

/** Sort key: earliest deadline first, todos without a deadline last. */
export const deadlineKey = (todo: Pick<Todo, 'due' | 'due_time'>) => (todo.due ? `${todo.due} ${todo.due_time ?? '99:99'}` : '9999');

/** Quick picks shown in the deadline picker. */
export function quickDates(): { label: string; date: DateStr }[] {
  const d = today();
  const dow = parseYmd(d).getDay(); // 0 = Sunday
  const saturday = addDays(d, dow === 6 ? 7 : 6 - dow);
  const nextMonday = addDays(d, ((8 - dow) % 7) || 7);
  return [
    { label: t('dl.today'), date: d },
    { label: t('dl.tomorrow'), date: addDays(d, 1) },
    { label: t('dl.weekend'), date: saturday },
    { label: t('dl.nextWeek'), date: nextMonday },
  ];
}
