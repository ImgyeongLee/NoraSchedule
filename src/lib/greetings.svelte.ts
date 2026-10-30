// The Overview greeting ("Good morning", "Still up?"): the user can write their own lines for each
// part of the day. Several lines per part are allowed; one is picked per day so it doesn't flicker.
import { api } from './api';
import { t, type Key } from './i18n.svelte';

export type DayPart = 'night' | 'morning' | 'afternoon' | 'evening';

/** Parts of the day, in order, with the hours they start at. */
export const DAY_PARTS: { id: DayPart; from: number; to: number; label: Key; fallback: Key }[] = [
  { id: 'night', from: 0, to: 5, label: 'greet.night', fallback: 'home.greetNight' },
  { id: 'morning', from: 5, to: 12, label: 'greet.morning', fallback: 'home.greetMorning' },
  { id: 'afternoon', from: 12, to: 18, label: 'greet.afternoon', fallback: 'home.greetAfternoon' },
  { id: 'evening', from: 18, to: 24, label: 'greet.evening', fallback: 'home.greetEvening' },
];

const SETTING = 'home.greetings';

/** The user's lines per part of the day (empty: the built-in greeting). */
export const greetings = $state<Record<DayPart, string>>({ night: '', morning: '', afternoon: '', evening: '' });
let loaded = false;

export async function loadGreetings() {
  if (loaded) return;
  loaded = true;
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    for (const p of DAY_PARTS) if (typeof saved?.[p.id] === 'string') greetings[p.id] = saved[p.id];
  } catch {
    /* built-in greetings */
  }
}

export async function saveGreetings(next: Record<DayPart, string>) {
  for (const p of DAY_PARTS) greetings[p.id] = next[p.id].trim();
  await api.setSetting(SETTING, JSON.stringify(greetings)).catch(() => {});
}

export const partOf = (hour: number) => DAY_PARTS.find((p) => hour >= p.from && hour < p.to) ?? DAY_PARTS[0];

/** The greeting for `date`: one of the user's lines for that part of the day, or the built-in one. */
export function greetingFor(date: Date): string {
  const part = partOf(date.getHours());
  const lines = greetings[part.id].split('\n').map((l) => l.trim()).filter(Boolean);
  if (!lines.length) return t(part.fallback);
  // Same line all day (per part of the day), different days get different lines.
  const seed = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}-${part.id}`;
  let hash = 0;
  for (const c of seed) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  return lines[hash % lines.length];
}
