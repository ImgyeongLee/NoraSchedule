// Commissions (외주): stages, money and deadline helpers shared by the page, its Overview tile
// and the calendar. Amounts use the currency from the expense settings (see expenses.svelte.ts).
import type { Component } from 'svelte';
import { CircleCheckBig, Hourglass, Inbox, MessageSquareMore, Paintbrush } from '@lucide/svelte';
import type { Commission, CommissionStatus } from './api';
import { diffDays, today } from './dates';
import type { Key } from './i18n.svelte';

export const STAGES: { id: CommissionStatus; label: Key; icon: Component<{ size?: number }>; color: string }[] = [
  { id: 'inquiry', label: 'cm.stage.inquiry', icon: Inbox, color: '#94a3b8' },
  { id: 'queued', label: 'cm.stage.queued', icon: Hourglass, color: '#f5b83d' },
  { id: 'working', label: 'cm.stage.working', icon: Paintbrush, color: '#7c74ff' },
  { id: 'review', label: 'cm.stage.review', icon: MessageSquareMore, color: '#f2668b' },
  { id: 'done', label: 'cm.stage.done', icon: CircleCheckBig, color: '#34c38f' },
];

export const stageOf = (id: string) => STAGES.find((s) => s.id === id) ?? STAGES[0];

/** The stage after `id` (the last one stays). */
export const nextStage = (id: CommissionStatus): CommissionStatus =>
  STAGES[Math.min(STAGES.length - 1, STAGES.findIndex((s) => s.id === id) + 1)].id;

/** Common kinds of art commission, offered as suggestions (anything can be typed). */
export const KIND_PRESETS = ['두상', '흉상', '반신', '전신', 'SD', '일러스트', '배경', '캐릭터 디자인', '로고', '굿즈'];

export const paidOf = (c: Commission) => c.payments.reduce((s, p) => s + p.amount, 0);

/** Common commission sites, offered as suggestions for the platform (anything can be typed). */
export const PLATFORM_PRESETS = ['크레페', '크몽', '픽시브 리퀘스트', '스킵', '포스타입', '트위터(X) 직거래', 'VGen', 'Ko-fi'];

/** The platform's cut of the price (never more than the price). */
export function feeOf(c: Pick<Commission, 'price' | 'fee' | 'fee_kind'>): number {
  const price = Math.max(0, Number(c.price) || 0);
  const fee = Math.max(0, Number(c.fee) || 0);
  return Math.min(price, c.fee_kind === 'amount' ? fee : (price * Math.min(100, fee)) / 100);
}
/** What actually reaches the artist for the whole price. */
export const netOf = (c: Pick<Commission, 'price' | 'fee' | 'fee_kind'>) => Math.max(0, (Number(c.price) || 0) - feeOf(c));
/** Share of each payment that is kept after the fee (1 without a price or fee). */
export const keptShare = (c: Commission) => (c.price > 0 ? netOf(c) / c.price : 1);
/** A payment after the platform fee (the fee is spread over the payments in proportion). */
export const netPayment = (c: Commission, amount: number) => amount * keptShare(c);
/** Everything received so far, after fees. */
export const netPaidOf = (c: Commission) => netPayment(c, paidOf(c));
/** What is still owed (never negative). */
export const unpaidOf = (c: Commission) => Math.max(0, c.price - paidOf(c));

export type PayState = 'free' | 'unpaid' | 'partial' | 'paid';
export function payState(c: Commission): PayState {
  if (c.price <= 0) return paidOf(c) > 0 ? 'paid' : 'free';
  const paid = paidOf(c);
  return paid <= 0 ? 'unpaid' : paid < c.price ? 'partial' : 'paid';
}

/** Days until the deadline (negative when overdue); null without one or once delivered. */
export function daysLeft(c: Commission, from = today()): number | null {
  if (!c.deadline || c.status === 'done') return null;
  return diffDays(c.deadline, from);
}

/** Open work first by deadline (none last), then newest. */
export const byDeadline = (a: Commission, b: Commission) =>
  (a.deadline ?? '9999').localeCompare(b.deadline ?? '9999') || b.created_at - a.created_at;

/** Delivered work, most recently delivered first. */
export const byDelivered = (a: Commission, b: Commission) =>
  (b.delivered ?? '').localeCompare(a.delivered ?? '') || b.created_at - a.created_at;

/** Set by the calendar or the Overview tile to open a commission on the Commissions page. */
export const commissionFocus = $state({ id: null as number | null });
