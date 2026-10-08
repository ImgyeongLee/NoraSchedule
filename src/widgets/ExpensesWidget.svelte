<script lang="ts">
  import { onMount } from 'svelte';
  import { Wallet } from '@lucide/svelte';
  import { api, type Expense } from '../lib/api';
  import { monthStart, parseYmd, today } from '../lib/dates';
  import { categoryOf, expenseSettings, fmtMoney, loadExpenseSettings } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';

  let expenses = $state<Expense[]>([]);

  onMount(() => {
    if (!expenseSettings.loaded) loadExpenseSettings();
  });

  $effect(() => {
    data.version;
    const from = monthStart(today());
    const d = parseYmd(from);
    const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
    load(api.expensesBetween(from, from.slice(0, 8) + String(last).padStart(2, '0')), []).then((e) => (expenses = e));
  });

  const total = $derived(expenses.reduce((s, e) => s + e.amount, 0));
  const used = $derived(expenseSettings.budget ? total / expenseSettings.budget : 0);
  const top = $derived.by(() => {
    const sums = new Map<string, number>();
    for (const e of expenses) sums.set(e.category, (sums.get(e.category) ?? 0) + e.amount);
    return [...sums.entries()].sort((a, b) => b[1] - a[1]).slice(0, 3);
  });
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'expenses')}><span class="w-icon"><Wallet size={15} /></span>{t('w.expenses')}</button>
  </div>
  <div class="w-body">
    <div class="total tabular">{expenseSettings.loaded ? fmtMoney(total) : ''}</div>
    <div class="faint small">{t('w.expenses.month')}</div>
    {#if expenseSettings.budget}
      <div class="bar" class:over={used > 1}><span style:width="{Math.min(100, used * 100)}%"></span></div>
      <div class="faint small">{t('ex.ofBudget', { pct: `${Math.round(used * 100)}%`, budget: fmtMoney(expenseSettings.budget) })}</div>
    {/if}
    <div class="cats">
      {#each top as [id, value] (id)}
        {@const c = categoryOf(id)}
        <span class="cat" style:--c={c.color}><c.icon size={12} /> {fmtMoney(value)}</span>
      {/each}
    </div>
  </div>
</div>

<style>
  .total {
    font-size: 26px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }
  .bar {
    height: 6px;
    margin: 10px 0 4px;
    border-radius: 6px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 6px;
    background: var(--success);
  }
  .bar.over span {
    background: var(--danger);
  }
  .cats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .cat {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 999px;
    font-size: 11.5px;
    font-weight: 650;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 12%, transparent);
  }
</style>
