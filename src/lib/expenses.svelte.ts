// Money book categories (spending and income, with preset icons/colors) and currency formatting.
import type { Component } from 'svelte';
import {
  Briefcase, Brush, Bus, CircleEllipsis, Coffee, Coins, GraduationCap, HeartHandshake, HeartPulse, House, Gamepad2, Gift, Landmark,
  Laptop, Plane, Presentation, Receipt, ShoppingBag, Store, Utensils,
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

/** Where income comes from; broad enough for jobs, side work and creative income alike. */
export const INCOME_CATEGORIES: { id: string; label: Key; icon: Component<{ size?: number }>; color: string }[] = [
  { id: 'salary', label: 'ex.in.salary', icon: Briefcase, color: '#4aa8ff' },
  { id: 'sidejob', label: 'ex.in.sidejob', icon: Laptop, color: '#7c74ff' },
  { id: 'commission', label: 'ex.in.commission', icon: Brush, color: '#f2668b' },
  { id: 'lesson', label: 'ex.in.lesson', icon: Presentation, color: '#22b5bf' },
  { id: 'sales', label: 'ex.in.sales', icon: Store, color: '#ff8a5c' },
  { id: 'creator', label: 'ex.in.creator', icon: HeartHandshake, color: '#b164e8' },
  { id: 'allowance', label: 'ex.in.allowance', icon: Coins, color: '#f5b83d' },
  { id: 'investment', label: 'ex.in.investment', icon: Landmark, color: '#34c38f' },
  { id: 'other_income', label: 'ex.in.other', icon: CircleEllipsis, color: '#94a3b8' },
];

export const categoryOf = (id: string) =>
  CATEGORIES.find((c) => c.id === id) ?? INCOME_CATEGORIES.find((c) => c.id === id) ?? CATEGORIES[CATEGORIES.length - 1];

export const CURRENCIES = ['KRW', 'USD', 'EUR', 'JPY', 'GBP', 'CNY', 'CAD', 'AUD'] as const;

/** Currencies normally written without decimals. */
const NO_DECIMALS = new Set(['KRW', 'JPY']);

export const expenseSettings = $state({
  currency: '' as string,
  budget: null as number | null,
  /** Show payments from the commission manager (after fees) as income. */
  linkCommissions: true,
  loaded: false,
});

export async function loadExpenseSettings() {
  const [currency, budget, link] = await Promise.all([
    api.getSetting('expense.currency').catch(() => null),
    api.getSetting('expense.budget').catch(() => null),
    api.getSetting('expense.linkCommissions').catch(() => null),
  ]);
  expenseSettings.linkCommissions = link !== 'false';
  expenseSettings.currency = currency ?? ({ ko: 'KRW', ja: 'JPY', en: 'USD' } as const)[i18n.locale];
  expenseSettings.budget = budget && Number(budget) > 0 ? Number(budget) : null;
  expenseSettings.loaded = true;
}

export async function saveExpenseSettings(currency: string, budget: number | null, linkCommissions = expenseSettings.linkCommissions) {
  expenseSettings.currency = currency;
  expenseSettings.budget = budget && budget > 0 ? budget : null;
  expenseSettings.linkCommissions = linkCommissions;
  await api.setSetting('expense.linkCommissions', String(linkCommissions)).catch(() => {});
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
