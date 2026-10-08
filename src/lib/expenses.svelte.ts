// Expense categories (preset icons/colors) and currency formatting.
import type { Component } from 'svelte';
import {
  Bus, CircleEllipsis, Coffee, GraduationCap, HeartPulse, House, Gamepad2, Gift, Plane, Receipt, ShoppingBag, Utensils,
} from '@lucide/svelte';
import { api } from './api';
import { intlLocale, i18n, type Key } from './i18n.svelte';

export const CATEGORIES: { id: string; label: Key; icon: Component<{ size?: number }>; color: string }[] = [
  { id: 'food', label: 'ex.cat.food', icon: Utensils, color: '#ff8a5c' },
  { id: 'cafe', label: 'ex.cat.cafe', icon: Coffee, color: '#b0773f' },
  { id: 'transport', label: 'ex.cat.transport', icon: Bus, color: '#4aa8ff' },
  { id: 'shopping', label: 'ex.cat.shopping', icon: ShoppingBag, color: '#f2668b' },
  { id: 'housing', label: 'ex.cat.housing', icon: House, color: '#7c74ff' },
  { id: 'bills', label: 'ex.cat.bills', icon: Receipt, color: '#7d8597' },
  { id: 'health', label: 'ex.cat.health', icon: HeartPulse, color: '#ef5d6c' },
  { id: 'fun', label: 'ex.cat.fun', icon: Gamepad2, color: '#b164e8' },
  { id: 'education', label: 'ex.cat.education', icon: GraduationCap, color: '#22b5bf' },
  { id: 'travel', label: 'ex.cat.travel', icon: Plane, color: '#34c38f' },
  { id: 'gifts', label: 'ex.cat.gifts', icon: Gift, color: '#f5b83d' },
  { id: 'other', label: 'ex.cat.other', icon: CircleEllipsis, color: '#94a3b8' },
];

export const categoryOf = (id: string) => CATEGORIES.find((c) => c.id === id) ?? CATEGORIES[CATEGORIES.length - 1];

export const CURRENCIES = ['KRW', 'USD', 'EUR', 'JPY', 'GBP', 'CNY', 'CAD', 'AUD'] as const;

/** Currencies normally written without decimals. */
const NO_DECIMALS = new Set(['KRW', 'JPY']);

export const expenseSettings = $state({ currency: '' as string, budget: null as number | null, loaded: false });

export async function loadExpenseSettings() {
  const [currency, budget] = await Promise.all([
    api.getSetting('expense.currency').catch(() => null),
    api.getSetting('expense.budget').catch(() => null),
  ]);
  expenseSettings.currency = currency ?? (i18n.locale === 'ko' ? 'KRW' : 'USD');
  expenseSettings.budget = budget && Number(budget) > 0 ? Number(budget) : null;
  expenseSettings.loaded = true;
}

export async function saveExpenseSettings(currency: string, budget: number | null) {
  expenseSettings.currency = currency;
  expenseSettings.budget = budget && budget > 0 ? budget : null;
  await api.setSetting('expense.currency', currency).catch(() => {});
  await api.setSetting('expense.budget', expenseSettings.budget ? String(expenseSettings.budget) : '').catch(() => {});
}

export function fmtMoney(amount: number, currency = expenseSettings.currency || 'USD'): string {
  const digits = NO_DECIMALS.has(currency) ? 0 : 2;
  return new Intl.NumberFormat(intlLocale(), {
    style: 'currency',
    currency,
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(amount);
}

/** Short form for chart labels, e.g. "₩12k". */
export function fmtMoneyShort(amount: number, currency = expenseSettings.currency || 'USD'): string {
  return new Intl.NumberFormat(intlLocale(), { style: 'currency', currency, notation: 'compact', maximumFractionDigits: 1 }).format(amount);
}

export const currencySymbol = (currency = expenseSettings.currency || 'USD') =>
  new Intl.NumberFormat(intlLocale(), { style: 'currency', currency }).formatToParts(0).find((p) => p.type === 'currency')?.value ?? currency;
