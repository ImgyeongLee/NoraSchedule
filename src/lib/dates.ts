// Date helpers. Dates travel as local `YYYY-MM-DD` strings and date-times as
// `YYYY-MM-DDTHH:MM:SS`, so string comparison equals chronological comparison.
import type { CalEvent, DateStr, DateTime } from './api';
import { intlLocale, t } from './i18n.svelte';

export const pad = (n: number) => String(n).padStart(2, '0');

export const ymd = (d: Date): DateStr => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

export function parseYmd(s: DateStr): Date {
  const [y, m, d] = s.split('-').map(Number);
  return new Date(y, m - 1, d);
}

export const today = (): DateStr => ymd(new Date());

export function addDays(s: DateStr, n: number): DateStr {
  const d = parseYmd(s);
  d.setDate(d.getDate() + n);
  return ymd(d);
}

export function addMonths(s: DateStr, n: number): DateStr {
  const d = parseYmd(s);
  const day = d.getDate();
  d.setDate(1);
  d.setMonth(d.getMonth() + n);
  const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
  d.setDate(Math.min(day, last));
  return ymd(d);
}

/** Sunday starting the week that contains `s`. */
export const weekStart = (s: DateStr): DateStr => addDays(s, -parseYmd(s).getDay());
export const monthStart = (s: DateStr): DateStr => s.slice(0, 8) + '01';
export const range = (from: DateStr, count: number): DateStr[] =>
  Array.from({ length: count }, (_, i) => addDays(from, i));

/** Whole days from `b` to `a` (positive when `a` is later). */
export const diffDays = (a: DateStr, b: DateStr) =>
  Math.round((parseYmd(a).getTime() - parseYmd(b).getTime()) / 86_400_000);

/** Unix seconds of local midnight starting `s`. */
export const dayStartTs = (s: DateStr) => Math.floor(parseYmd(s).getTime() / 1000);
export const tsToDate = (ts: number): DateStr => ymd(new Date(ts * 1000));

export function fmt(s: DateStr, opts: Intl.DateTimeFormatOptions): string {
  return new Intl.DateTimeFormat(intlLocale(), opts).format(parseYmd(s));
}

/** Formats a date range compactly, e.g. "October 4 – 10" / "10월 4일~10일". */
export function fmtRange(a: DateStr, b: DateStr, opts: Intl.DateTimeFormatOptions): string {
  return new Intl.DateTimeFormat(intlLocale(), opts).formatRange(parseYmd(a), parseYmd(b));
}

export function fmtTs(ts: number, opts: Intl.DateTimeFormatOptions = { hour: '2-digit', minute: '2-digit', hour12: false }) {
  return new Intl.DateTimeFormat(intlLocale(), opts).format(new Date(ts * 1000));
}

/** Weekday names starting on Sunday, e.g. ["Sun", …] or ["일", …]. */
export function weekdayNames(style: 'narrow' | 'short' = 'short'): string[] {
  const fmt = new Intl.DateTimeFormat(intlLocale(), { weekday: style });
  return Array.from({ length: 7 }, (_, i) => fmt.format(new Date(2026, 0, 4 + i))); // Jan 4 2026 is a Sunday
}

export const dateOf = (dt: DateTime): DateStr => dt.slice(0, 10);
export const timeOf = (dt: DateTime): string => dt.slice(11, 16);
export const toDateTime = (date: DateStr, time: string): DateTime => `${date}T${time.slice(0, 5)}:00`;

export function addMinutes(dt: DateTime, minutes: number): DateTime {
  const d = new Date(dt);
  d.setMinutes(d.getMinutes() + minutes);
  return `${ymd(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}:00`;
}

/** The same time of day, `n` days later (or earlier). */
export const shiftDays = (dt: DateTime, n: number): DateTime => toDateTime(addDays(dateOf(dt), n), timeOf(dt));

/** Minutes since local midnight of the date-time. */
export const minutesOf = (dt: DateTime) => Number(dt.slice(11, 13)) * 60 + Number(dt.slice(14, 16));

/** First and last calendar day an event occupies (mirrors `Event::date_span` in Rust). */
export function eventSpan(e: CalEvent): [DateStr, DateStr] {
  const first = dateOf(e.start);
  let last: DateStr;
  if (e.all_day) last = dateOf(e.end);
  else if (e.end > e.start) last = dateOf(addMinutes(e.end, -1));
  else last = first;
  return [first, last < first ? first : last];
}

export function covers(e: CalEvent, day: DateStr): boolean {
  const [a, b] = eventSpan(e);
  return a <= day && day <= b;
}

/** Events shown in the all-day lane: all-day or spanning several days. */
export function isAllDayLane(e: CalEvent): boolean {
  const [a, b] = eventSpan(e);
  return e.all_day || a !== b;
}

export function fmtDuration(secs: number): string {
  secs = Math.max(0, Math.round(secs));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return t('dur.hm', { h, m: pad(m) });
  if (m > 0) return t('dur.m', { m });
  return t('dur.s', { s: secs });
}

export function fmtHours(secs: number): string {
  const h = secs / 3600;
  return t('dur.h', { h: h >= 10 ? h.toFixed(0) : h.toFixed(1) });
}

type DDayLike = { date: DateStr; yearly: boolean; count_from_one: boolean };

/** The `year`'s anniversary of `date` (Feb 29 falls on Feb 28 in other years). */
function anniversary(date: DateStr, year: number): DateStr {
  const [, m, d] = date.split('-').map(Number);
  const last = new Date(year, m, 0).getDate();
  return `${year}-${pad(m)}-${pad(Math.min(d, last))}`;
}

/** The day a D-Day counts toward: its date, or for yearly ones the next anniversary from `from` on. */
export function ddayTarget(d: DDayLike, from: DateStr = today()): DateStr {
  if (!d.yearly || d.date >= from) return d.date;
  const year = Number(from.slice(0, 4));
  const thisYear = anniversary(d.date, year);
  return thisYear >= from ? thisYear : anniversary(d.date, year + 1);
}

/** Whether the D-Day falls on `day` (every anniversary, for yearly ones). */
export function ddayOn(d: DDayLike, day: DateStr): boolean {
  if (d.date === day) return true;
  return d.yearly && day > d.date && anniversary(d.date, Number(day.slice(0, 4))) === day;
}

/** Days until the D-Day (positive), or since it (negative). Counting from one adds the start day. */
export function ddayDays(d: DDayLike, from: DateStr = today()): number {
  const n = diffDays(ddayTarget(d, from), from);
  return n <= 0 && d.count_from_one && !d.yearly ? n - 1 : n;
}

/** "D-12", "D-Day" or "D+30". */
export function ddayLabel(d: DDayLike, from: DateStr = today()): string {
  const n = ddayDays(d, from);
  if (n === 0) return 'D-Day';
  return n > 0 ? `D-${n}` : `D+${-n}`;
}

/** Whether the D-Day is still ahead (yearly ones always are). */
export const ddayUpcoming = (d: DDayLike, from: DateStr = today()) => ddayTarget(d, from) >= from;

/** D-Days sorted by the day they count toward. */
export const byDdayTarget = <T extends DDayLike>(list: T[], from: DateStr = today()): T[] =>
  [...list].sort((a, b) => ddayTarget(a, from).localeCompare(ddayTarget(b, from)));

/** Assigns overlapping time spans (sorted by start) to side-by-side columns. */
export function overlapColumns(spans: [number, number][]): { col: number; cols: number }[] {
  const out = spans.map(() => ({ col: 0, cols: 1 }));
  let cluster: number[] = [];
  let colEnds: number[] = [];
  let clusterEnd = -Infinity;
  const finish = () => {
    for (const i of cluster) out[i].cols = Math.max(1, colEnds.length);
    cluster = [];
    colEnds = [];
  };
  spans.forEach(([s, e], i) => {
    if (s >= clusterEnd) {
      finish();
      clusterEnd = -Infinity;
    }
    let col = colEnds.findIndex((end) => end <= s);
    if (col === -1) {
      colEnds.push(e);
      col = colEnds.length - 1;
    } else {
      colEnds[col] = e;
    }
    out[i].col = col;
    cluster.push(i);
    clusterEnd = Math.max(clusterEnd, e);
  });
  finish();
  return out;
}
