// Medicines, sleep and how the day felt (the health page's other tabs): what is due, how long a
// night was, and the preset scales and tags.
import type { Component } from 'svelte';
import { Angry, Frown, Laugh, Meh, Smile } from '@lucide/svelte';
import type { Condition, DateStr, MedLog, Medication } from './api';
import { pad } from './dates';
import { t, type Key } from './i18n.svelte';

type Icon = Component<{ size?: number }>;

// ---- medicines

/** Scheduled (not as-needed) medicines to take on `day`. */
export const medsDue = (meds: Medication[], day: DateStr) =>
  meds.filter((m) => m.active && !m.as_needed && (!m.start || m.start <= day) && (!m.end || m.end >= day));

/** Each dose of a medicine in a day: its times, or one dose at no set time. */
export const dosesOf = (m: Medication) => (m.times.length ? m.times : ['']);

export const isTaken = (logs: MedLog[], m: Medication, day: DateStr, slot: string) =>
  logs.some((l) => l.med_id === m.id && l.date === day && l.slot === slot);

/** Doses due on `day` and how many of them were ticked. */
export function doseCount(meds: Medication[], logs: MedLog[], day: DateStr) {
  let due = 0;
  let taken = 0;
  for (const m of medsDue(meds, day)) {
    for (const slot of dosesOf(m)) {
      due++;
      if (isTaken(logs, m, day, slot)) taken++;
    }
  }
  return { due, taken };
}

export const nowTime = () => {
  const d = new Date();
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};

// ---- sleep

const minutesOf = (hhmm: string) => Number(hhmm.slice(0, 2)) * 60 + Number(hhmm.slice(3, 5));

/** Minutes asleep from bed time to wake time, across midnight. */
export const sleepMinutes = (bed: string, wake: string) => (minutesOf(wake) - minutesOf(bed) + 1440) % 1440;

/** "7h 20m". */
export const fmtDuration = (minutes: number) => {
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  return h ? (m ? t('wl.hm', { h, m }) : t('wl.h', { h })) : t('wl.m', { m });
};

/** The average of clock times around midnight (23:30 and 00:30 average to 00:00). */
export function averageTime(times: string[]) {
  if (!times.length) return '';
  // Times before noon count as after midnight, so a bed time of 01:00 sits after 23:00.
  const shifted = times.map((x) => {
    const m = minutesOf(x);
    return m < 720 ? m + 1440 : m;
  });
  const avg = Math.round(shifted.reduce((s, m) => s + m, 0) / shifted.length) % 1440;
  return `${pad(Math.floor(avg / 60))}:${pad(avg % 60)}`;
}
/** Wake-up times don't cross midnight, so they average plainly. */
export function averageWake(times: string[]) {
  if (!times.length) return '';
  const avg = Math.round(times.reduce((s, x) => s + minutesOf(x), 0) / times.length);
  return `${pad(Math.floor(avg / 60))}:${pad(avg % 60)}`;
}

// ---- how the day felt

export type Scale = 'mood' | 'energy' | 'stress';
export const SCALES: Scale[] = ['mood', 'energy', 'stress'];

export const MOOD_ICONS: Icon[] = [Angry, Frown, Meh, Smile, Laugh];
/** Bad to good, for mood and energy. */
export const SCALE_COLORS = ['#ef5d6c', '#ff8a5c', '#f5b83d', '#34c38f', '#4aa8ff'];
/** Stress runs the other way: 5 is the worst. */
export const scaleColor = (scale: Scale, n: number) => (n ? SCALE_COLORS[scale === 'stress' ? 5 - n : n - 1] : 'var(--faint)');
export const scaleLabel = (scale: Scale, n: number) => t(`wl.${scale}.${n}` as Key);

/** Preset tags, grouped. Anything else the user types is kept as is. */
export const TAG_GROUPS: { id: string; tags: string[] }[] = [
  { id: 'feeling', tags: ['happy', 'calm', 'excited', 'grateful', 'depressed', 'anxious', 'irritable', 'sad', 'lonely', 'angry'] },
  { id: 'body', tags: ['tired', 'sleepy', 'headache', 'indigestion', 'pain', 'cold', 'dizzy', 'period'] },
  { id: 'appetite', tags: ['appetite_up', 'appetite_down', 'craving'] },
  { id: 'mind', tags: ['focused', 'unfocused', 'motivated', 'lethargic', 'stressed'] },
];
const PRESET = new Set(TAG_GROUPS.flatMap((g) => g.tags));
export const isPresetTag = (tag: string) => PRESET.has(tag);
export const tagLabel = (tag: string) => (PRESET.has(tag) ? t(`wl.tag.${tag}` as Key) : tag);

export const emptyCondition = (date: DateStr): Condition => ({ id: 0, date, mood: 0, energy: 0, stress: 0, tags: [], note: '' });
