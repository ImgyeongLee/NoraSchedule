// Lays out calendar items over a row of consecutive days (a week in the month view, the
// all-day row in the week view) so an event covering several days is one continuous bar.
import type { CalEvent, DDay, Todo } from './api';
import { ddayOn, diffDays, eventSpan, isAllDayLane } from './dates';

/** A todo shown on its due day, in its group's color. */
export type CalTodo = { todo: Todo; color: number };

type Item = {
  /** First day column and number of days covered within the row. */
  col: number;
  span: number;
  /** The event continues from before / past the end of the row. */
  fromPrev: boolean;
  toNext: boolean;
} & ({ kind: 'dday'; d: DDay } | { kind: 'event'; e: CalEvent } | { kind: 'todo'; t: CalTodo });

/** An item placed in the row; `lane` is the row inside the day cells. */
export type Placed = Item & { lane: number };

/** Places D-Days, events and todos (those touching `days`) into the lowest free lanes. */
export function layoutLanes(
  days: string[],
  ddays: DDay[],
  events: CalEvent[],
  todos: CalTodo[] = [],
  /** Put single-day all-day events after timed ones (multi-day bars stay on top). */
  allDayLast = false,
): { placed: Placed[]; lanesPerDay: number[] } {
  const first = days[0];
  const last = days[days.length - 1];
  const items: Item[] = [];
  for (const d of ddays) {
    // Yearly D-Days show on every anniversary.
    days.forEach((day, col) => {
      if (ddayOn(d, day)) items.push({ kind: 'dday', d, col, span: 1, fromPrev: false, toNext: false });
    });
  }
  for (const e of events) {
    const [a, b] = eventSpan(e);
    if (b < first || a > last) continue;
    const s = a < first ? first : a;
    const end = b > last ? last : b;
    items.push({ kind: 'event', e, col: diffDays(s, first), span: diffDays(end, s) + 1, fromPrev: a < first, toNext: b > last });
  }
  for (const t of todos) {
    const due = t.todo.due;
    if (due && due >= first && due <= last) {
      items.push({ kind: 'todo', t, col: diffDays(due, first), span: 1, fromPrev: false, toNext: false });
    }
  }
  // Earlier and longer first, so bars stack the way people expect: D-Days and multi-day bars on top,
  // then timed events, single-day all-day events (above or below the timed ones), and todos last.
  const rank = (i: Item) => {
    if (i.kind === 'dday') return 0;
    if (i.kind === 'todo') return 4;
    const [a, b] = eventSpan(i.e);
    if (a !== b) return 1;
    return isAllDayLane(i.e) ? (allDayLast ? 3 : 1) : 2;
  };
  items.sort(
    (x, y) =>
      x.col - y.col ||
      y.span - x.span ||
      rank(x) - rank(y) ||
      (x.kind === 'event' && y.kind === 'event' ? x.e.start.localeCompare(y.e.start) : 0) ||
      (x.kind === 'todo' && y.kind === 'todo' ? (x.t.todo.due_time ?? '').localeCompare(y.t.todo.due_time ?? '') : 0),
  );

  const used: boolean[][] = days.map(() => []);
  const placed = items.map((item): Placed => {
    const taken = (l: number) => used.slice(item.col, item.col + item.span).some((lanes) => lanes[l]);
    let lane = 0;
    while (taken(lane)) lane++;
    for (let k = 0; k < item.span; k++) used[item.col + k][lane] = true;
    return { ...item, lane };
  });
  return { placed, lanesPerDay: used.map((lanes) => lanes.length) };
}
