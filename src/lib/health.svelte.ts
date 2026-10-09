// Diet and exercise: meal slots, exercise types with their calorie burn, and the daily goal.
import type { Component } from 'svelte';
import {
  Activity, Bike, Cookie, Dumbbell, Flower2, Footprints, Moon, Mountain, Music, PersonStanding, StretchHorizontal, Sun, Sunrise,
  Volleyball, WavesLadder, Zap,
} from '@lucide/svelte';
import { api, type MealSlot } from './api';
import { intlLocale, type Key } from './i18n.svelte';

type Icon = Component<{ size?: number }>;

export const SLOTS: { id: MealSlot; label: Key; icon: Icon; color: string }[] = [
  { id: 'breakfast', label: 'hl.slot.breakfast', icon: Sunrise, color: '#f5b83d' },
  { id: 'lunch', label: 'hl.slot.lunch', icon: Sun, color: '#ff8a5c' },
  { id: 'dinner', label: 'hl.slot.dinner', icon: Moon, color: '#7c74ff' },
  { id: 'snack', label: 'hl.slot.snack', icon: Cookie, color: '#b0773f' },
];

export const slotOf = (id: string) => SLOTS.find((s) => s.id === id) ?? SLOTS[SLOTS.length - 1];

/** The meal you are most likely logging right now. */
export function slotForNow(): MealSlot {
  const h = new Date().getHours();
  return h < 11 ? 'breakfast' : h < 16 ? 'lunch' : h < 21 ? 'dinner' : 'snack';
}

/**
 * Exercise presets. `met` is the energy cost compared with sitting still (from the
 * Compendium of Physical Activities), used to estimate the calories burned.
 */
export const EXERCISES: { id: string; label: Key; icon: Icon; color: string; met: number }[] = [
  { id: 'walking', label: 'hl.ex.walking', icon: Footprints, color: '#34c38f', met: 3.5 },
  { id: 'running', label: 'hl.ex.running', icon: Zap, color: '#ef5d6c', met: 9.8 },
  { id: 'cycling', label: 'hl.ex.cycling', icon: Bike, color: '#4aa8ff', met: 7.5 },
  { id: 'swimming', label: 'hl.ex.swimming', icon: WavesLadder, color: '#22b5bf', met: 7.0 },
  { id: 'strength', label: 'hl.ex.strength', icon: Dumbbell, color: '#7c74ff', met: 5.0 },
  { id: 'hiit', label: 'hl.ex.hiit', icon: Activity, color: '#f2668b', met: 8.0 },
  { id: 'yoga', label: 'hl.ex.yoga', icon: Flower2, color: '#b164e8', met: 2.5 },
  { id: 'pilates', label: 'hl.ex.pilates', icon: PersonStanding, color: '#e58fd0', met: 3.0 },
  { id: 'hiking', label: 'hl.ex.hiking', icon: Mountain, color: '#6f9b4f', met: 6.0 },
  { id: 'dance', label: 'hl.ex.dance', icon: Music, color: '#ff8a5c', met: 5.0 },
  { id: 'sports', label: 'hl.ex.sports', icon: Volleyball, color: '#f5b83d', met: 6.5 },
  { id: 'stretching', label: 'hl.ex.stretching', icon: StretchHorizontal, color: '#94a3b8', met: 2.3 },
];

export const exerciseOf = (id: string) => EXERCISES.find((e) => e.id === id) ?? EXERCISES[0];

/** Estimated calories burned: MET × body weight (kg) × hours. */
export const estimateBurn = (kind: string, minutes: number, weight = healthSettings.weight) =>
  minutes > 0 ? Math.round((exerciseOf(kind).met * weight * minutes) / 60) : 0;

export const DEFAULT_GOAL = 2000;
export const DEFAULT_WEIGHT = 60;
export const MAX_KCAL = 20000;
export const MAX_MINUTES = 24 * 60;

export const healthSettings = $state({ goal: DEFAULT_GOAL, weight: DEFAULT_WEIGHT, loaded: false });

export async function loadHealthSettings() {
  const [goal, weight] = await Promise.all([
    api.getSetting('health.goal').catch(() => null),
    api.getSetting('health.weight').catch(() => null),
  ]);
  healthSettings.goal = Number(goal) > 0 ? Number(goal) : DEFAULT_GOAL;
  healthSettings.weight = Number(weight) > 0 ? Number(weight) : DEFAULT_WEIGHT;
  healthSettings.loaded = true;
}

export async function saveHealthSettings(goal: number, weight: number) {
  healthSettings.goal = goal > 0 ? Math.round(goal) : DEFAULT_GOAL;
  healthSettings.weight = weight > 0 ? Math.round(weight * 10) / 10 : DEFAULT_WEIGHT;
  await api.setSetting('health.goal', String(healthSettings.goal)).catch(() => {});
  await api.setSetting('health.weight', String(healthSettings.weight)).catch(() => {});
}

/** "1,240" (the unit is added by the caller's label). */
export const fmtNum = (n: number) => new Intl.NumberFormat(intlLocale()).format(Math.round(n));

/** "1,240 kcal". */
export const fmtKcal = (n: number) => `${fmtNum(n)} kcal`;

export interface DayTotals {
  eaten: number;
  planned: number;
  burned: number;
  minutes: number;
  /** Goal − eaten + burned: what is left to eat today (negative when over). */
  left: number;
}

export function dayTotals(
  meals: { kcal: number; eaten: boolean }[],
  workouts: { kcal: number; minutes: number }[],
  goal = healthSettings.goal,
): DayTotals {
  const eaten = meals.filter((m) => m.eaten).reduce((s, m) => s + m.kcal, 0);
  const planned = meals.filter((m) => !m.eaten).reduce((s, m) => s + m.kcal, 0);
  const burned = workouts.reduce((s, w) => s + w.kcal, 0);
  const minutes = workouts.reduce((s, w) => s + w.minutes, 0);
  return { eaten, planned, burned, minutes, left: goal - eaten + burned };
}
