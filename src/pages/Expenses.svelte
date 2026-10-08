<script lang="ts">
  import { onMount } from 'svelte';
  import {
    CalendarDays, ChevronLeft, ChevronRight, CopyPlus, Pencil, PiggyBank, Plus, Receipt, Settings2, Trash, TrendingDown, TrendingUp, Wallet,
  } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import StatCard from '../components/StatCard.svelte';
  import BarChart from '../components/BarChart.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import { api, type Expense } from '../lib/api';
  import { addMonths, fmt, monthStart, parseYmd, range, today } from '../lib/dates';
  import {
    CATEGORIES, CURRENCIES, categoryOf, currencySymbol, expenseSettings, fmtMoney, fmtMoneyShort, loadExpenseSettings,
    saveExpenseSettings,
  } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';

  let month = $state(monthStart(today()));
  let expenses = $state<Expense[]>([]);
  let lastMonth = $state<Expense[]>([]);
  let editing = $state<Expense | null>(null);
  let settingsOpen = $state(false);
  let settingsForm = $state({ currency: '', budget: '' });

  // Quick-add form.
  let amount = $state('');
  let category = $state('food');
  let date = $state(today());
  let note = $state('');
  let amountInput: HTMLInputElement;

  const daysInMonth = $derived(new Date(parseYmd(month).getFullYear(), parseYmd(month).getMonth() + 1, 0).getDate());
  const monthEnd = $derived(month.slice(0, 8) + String(daysInMonth).padStart(2, '0'));

  onMount(() => {
    if (!expenseSettings.loaded) loadExpenseSettings();
  });

  $effect(() => {
    data.version;
    const [from, to] = [month, monthEnd];
    load(api.expensesBetween(from, to), []).then((e) => (expenses = e));
    const prev = addMonths(from, -1);
    const prevEnd = new Date(parseYmd(prev).getFullYear(), parseYmd(prev).getMonth() + 1, 0).getDate();
    load(api.expensesBetween(prev, prev.slice(0, 8) + String(prevEnd).padStart(2, '0')), []).then((e) => (lastMonth = e));
  });

  const total = $derived(expenses.reduce((s, x) => s + x.amount, 0));
  const isCurrentMonth = $derived(month === monthStart(today()));
  const daysSoFar = $derived(isCurrentMonth ? parseYmd(today()).getDate() : daysInMonth);
  // Compare with the same days of last month (e.g. Oct 1–7 vs Sep 1–7), not the whole month.
  const lastMonthSamePeriod = $derived(
    lastMonth.filter((e) => Number(e.date.slice(8)) <= daysSoFar).reduce((s, x) => s + x.amount, 0),
  );
  const change = $derived(lastMonthSamePeriod > 0 ? (total - lastMonthSamePeriod) / lastMonthSamePeriod : null);
  const budget = $derived(expenseSettings.budget);
  const budgetUsed = $derived(budget ? total / budget : 0);

  const byCategory = $derived(
    CATEGORIES.map((c) => ({ ...c, value: expenses.filter((e) => e.category === c.id).reduce((s, e) => s + e.amount, 0) }))
      .filter((c) => c.value > 0)
      .sort((a, b) => b.value - a.value),
  );
  const perDay = $derived(
    range(month, daysInMonth).map((d) => ({
      label: String(Number(d.slice(8))),
      value: expenses.filter((e) => e.date === d).reduce((s, e) => s + e.amount, 0),
    })),
  );
  const groups = $derived.by(() => {
    const map = new Map<string, Expense[]>();
    for (const e of expenses) map.set(e.date, [...(map.get(e.date) ?? []), e]);
    return [...map.entries()];
  });

  function pct(n: number) {
    return `${Math.round(n * 100)}%`;
  }

  async function add() {
    const value = Number(amount);
    if (!(value > 0)) {
      toast(t('ex.invalidAmount'), 'error');
      amountInput?.focus();
      return;
    }
    const ok = await mutate(api.saveExpense({ id: 0, amount: value, category, date: date || today(), note: note.trim() }), t('ex.added'));
    if (ok !== undefined) {
      amount = '';
      note = '';
      amountInput?.focus();
    }
  }

  async function saveEdit() {
    if (!editing) return;
    if (!(editing.amount > 0)) return toast(t('ex.invalidAmount'), 'error');
    const ok = await mutate(api.saveExpense({ ...editing, note: editing.note.trim() }), t('ex.saved'));
    if (ok !== undefined) editing = null;
  }

  async function remove(e: Expense) {
    try {
      const stored = await api.deleteExpense(e.id);
      data.version++;
      toast(t('clip.deleted', { name: fmtMoney(e.amount) }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveExpense({ ...stored, id: 0 }), t('ex.saved')),
      });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function rowMenu(ev: MouseEvent, e: Expense) {
    openMenu(ev, [
      { label: t('menu.edit'), icon: Pencil, action: () => (editing = { ...e }) },
      { label: t('menu.duplicate'), icon: CopyPlus, action: () => mutate(api.saveExpense({ ...e, id: 0 }), t('ex.added')) },
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: () => remove(e) },
    ]);
  }

  function openSettings() {
    settingsForm = { currency: expenseSettings.currency, budget: expenseSettings.budget ? String(expenseSettings.budget) : '' };
    settingsOpen = true;
  }

  async function saveSettings() {
    await saveExpenseSettings(settingsForm.currency, settingsForm.budget ? Number(settingsForm.budget) : null);
    settingsOpen = false;
  }
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.expenses')}</h1>
      <p class="sub">{t('ex.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <div class="month-nav">
      <button class="icon-btn" onclick={() => (month = addMonths(month, -1))} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
      <span class="month-label">{fmt(month, { year: 'numeric', month: 'long' })}</span>
      <button class="icon-btn" onclick={() => (month = addMonths(month, 1))} aria-label={t('common.next')}><ChevronRight size={18} /></button>
      {#if !isCurrentMonth}<button class="btn small" onclick={() => (month = monthStart(today()))}>{t('common.today')}</button>{/if}
    </div>
    <button class="btn" onclick={openSettings}><Settings2 size={16} /> {t('ex.settings')}</button>
  </div>

  <div class="stats">
    <StatCard
      label={t('ex.thisMonth')}
      value={fmtMoney(total)}
      sub={change === null ? '' : change === 0 ? t('ex.sameAsLast') : t('ex.vsLast', { pct: `${change > 0 ? '+' : ''}${pct(change)}` })}
      icon={change !== null && change > 0 ? TrendingUp : TrendingDown}
    />
    <StatCard label={t('ex.dailyAvg')} value={fmtMoney(total / Math.max(1, daysSoFar))} icon={CalendarDays} tint="var(--long-break)" />
    <div class="budget-card">
      {#if budget}
        <StatCard
          label={budgetUsed > 1 ? t('ex.overBudget') : t('ex.budgetLeft')}
          value={fmtMoney(Math.abs(budget - total))}
          sub={t('ex.ofBudget', { pct: pct(budgetUsed), budget: fmtMoney(budget) })}
          icon={PiggyBank}
          tint={budgetUsed > 1 ? 'var(--danger)' : budgetUsed > 0.8 ? 'var(--warning)' : 'var(--success)'}
        />
        <div class="budget-bar" class:over={budgetUsed > 1}><span style:width="{Math.min(100, budgetUsed * 100)}%"></span></div>
      {:else}
        <button class="set-budget" onclick={openSettings}>
          <PiggyBank size={20} />
          <span class="strong">{t('ex.setBudget')}</span>
          <span class="faint small">{t('ex.noBudget')}</span>
        </button>
      {/if}
    </div>
    <StatCard label={t('ex.entries')} value={String(expenses.length)} icon={Receipt} tint="var(--warning)" />
  </div>

  <div class="layout">
    <div class="card quick">
      <h3>{t('ex.quickAdd')}</h3>
      <div class="amount-row">
        <span class="symbol">{currencySymbol()}</span>
        <input
          bind:this={amountInput}
          class="amount"
          type="number"
          inputmode="decimal"
          min="0"
          step="any"
          placeholder="0"
          bind:value={amount}
          onkeydown={(e) => e.key === 'Enter' && add()}
          aria-label={t('ex.amount')}
        />
      </div>
      <div class="cats">
        {#each CATEGORIES as c (c.id)}
          <button type="button" class="cat" class:on={category === c.id} style:--c={c.color} onclick={() => (category = c.id)} title={t(c.label)}>
            <span class="cat-icon"><c.icon size={18} /></span>
            <span class="truncate">{t(c.label)}</span>
          </button>
        {/each}
      </div>
      <div class="row">
        <input class="input date" type="date" bind:value={date} aria-label={t('common.date')} />
        <input class="input" placeholder={t('ex.notePlaceholder')} bind:value={note} onkeydown={(e) => e.key === 'Enter' && add()} />
      </div>
      <button class="btn primary add-btn" onclick={add}><Plus size={17} /> {t('ex.add')}</button>
    </div>

    <div class="charts">
      <div class="card">
        <h3>{t('ex.byCategory')}</h3>
        {#if byCategory.length}
          <div class="cat-list">
            {#each byCategory as c (c.id)}
              <div class="cat-row" style:--c={c.color}>
                <span class="cat-icon small-icon"><c.icon size={15} /></span>
                <span class="cat-name truncate">{t(c.label)}</span>
                <span class="cat-bar"><span style:width="{(c.value / byCategory[0].value) * 100}%"></span></span>
                <span class="cat-amount tabular">{fmtMoney(c.value)}</span>
                <span class="cat-pct faint small">{pct(c.value / total)}</span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="muted small">{t('ex.emptyBody')}</p>
        {/if}
      </div>
      <div class="card">
        <h3>{t('ex.byDay')}</h3>
        <BarChart data={perDay} color="var(--primary)" height={170} format={(v) => fmtMoneyShort(v)} />
      </div>
    </div>
  </div>

  <div class="card history">
    <h3>{t('ex.list')}</h3>
    {#each groups as [day, list] (day)}
      <div class="day-head">
        <span class="strong">{fmt(day, { weekday: 'short', month: 'short', day: 'numeric' })}</span>
        <span class="muted small tabular">{t('ex.dayTotal', { amount: fmtMoney(list.reduce((s, e) => s + e.amount, 0)) })}</span>
      </div>
      {#each list as e (e.id)}
        {@const c = categoryOf(e.category)}
        <button class="expense" style:--c={c.color} onclick={() => (editing = { ...e })} oncontextmenu={(ev) => rowMenu(ev, e)}>
          <span class="cat-icon"><c.icon size={18} /></span>
          <span class="exp-text">
            <span class="strong">{e.note || t(c.label)}</span>
            {#if e.note}<span class="faint small">{t(c.label)}</span>{/if}
          </span>
          <span class="exp-amount tabular">{fmtMoney(e.amount)}</span>
        </button>
      {/each}
    {:else}
      <div class="empty">
        <span class="empty-icon"><Wallet size={30} /></span>
        <h2>{t('ex.emptyTitle')}</h2>
        <p>{t('ex.emptyBody')}</p>
      </div>
    {/each}
  </div>
</div>

{#if editing}
  <Modal title={t('ex.edit')} onclose={() => (editing = null)} width={480}>
    <div class="field">
      <label for="ex-amount">{t('ex.amount')}</label>
      <div class="amount-row in-modal">
        <span class="symbol">{currencySymbol()}</span>
        <input id="ex-amount" class="amount" type="number" inputmode="decimal" min="0" step="any" bind:value={editing.amount}
          onkeydown={(e) => e.key === 'Enter' && saveEdit()} />
      </div>
    </div>
    <div class="field">
      <span class="label">{t('ex.category')}</span>
      <div class="cats">
        {#each CATEGORIES as c (c.id)}
          <button type="button" class="cat" class:on={editing.category === c.id} style:--c={c.color} onclick={() => editing && (editing.category = c.id)}>
            <span class="cat-icon"><c.icon size={18} /></span>
            <span class="truncate">{t(c.label)}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="row">
      <input class="input date" type="date" bind:value={editing.date} aria-label={t('common.date')} />
      <input class="input" placeholder={t('ex.notePlaceholder')} bind:value={editing.note} />
    </div>
    {#snippet footer()}
      <ConfirmButton onconfirm={() => { const e = editing; editing = null; if (e) remove(e); }} />
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editing = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveEdit}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

{#if settingsOpen}
  <Modal title={t('ex.settings')} onclose={() => (settingsOpen = false)} width={420}>
    <div class="field">
      <label for="ex-currency">{t('ex.currency')}</label>
      <select id="ex-currency" class="select" bind:value={settingsForm.currency}>
        {#each CURRENCIES as c (c)}<option value={c}>{c} · {currencySymbol(c)}</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="ex-budget">{t('ex.budget')}</label>
      <input id="ex-budget" class="input" type="number" min="0" step="any" placeholder={t('ex.budgetPlaceholder')} bind:value={settingsForm.budget} />
    </div>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (settingsOpen = false)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveSettings}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .month-nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .month-label {
    min-width: 120px;
    text-align: center;
    font-weight: 700;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr));
    gap: 14px;
    margin-bottom: 16px;
  }
  .budget-card {
    position: relative;
  }
  .budget-bar {
    position: absolute;
    left: 18px;
    right: 18px;
    bottom: 12px;
    height: 5px;
    border-radius: 5px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .budget-bar span {
    display: block;
    height: 100%;
    border-radius: 5px;
    background: var(--success);
  }
  .budget-bar.over span {
    background: var(--danger);
  }
  .set-budget {
    width: 100%;
    height: 100%;
    min-height: 130px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: 2px dashed var(--border);
    border-radius: var(--radius-lg);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .set-budget:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
  .strong {
    font-weight: 650;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.1fr);
    gap: 16px;
    margin-bottom: 16px;
    align-items: start;
  }
  .quick {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .amount-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .amount-row:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 18%, transparent);
  }
  .symbol {
    font-size: 24px;
    font-weight: 700;
    color: var(--muted);
  }
  .amount {
    flex: 1;
    min-width: 0;
    height: 52px;
    border: none;
    background: none;
    outline: none;
    font-size: 30px;
    font-weight: 750;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
  .amount::-webkit-inner-spin-button,
  .amount::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .amount-row.in-modal .amount {
    height: 42px;
    font-size: 22px;
  }
  .cats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 6px;
  }
  .cat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 10px 6px;
    border: 1.5px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
  }
  .cat:hover {
    border-color: var(--c);
  }
  .cat.on {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 10%, var(--surface));
  }
  .cat .truncate {
    max-width: 100%;
  }
  .cat-icon {
    width: 36px;
    height: 36px;
    flex: none;
    border-radius: 12px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .small-icon {
    width: 28px;
    height: 28px;
    border-radius: 9px;
  }
  .date {
    width: auto;
    flex: none;
  }
  .add-btn {
    height: 44px;
  }
  .charts {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .charts h3,
  .quick h3,
  .history h3 {
    margin-bottom: 4px;
  }
  .cat-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 10px;
  }
  .cat-row {
    display: grid;
    grid-template-columns: 28px minmax(70px, 1fr) minmax(0, 1.4fr) auto 38px;
    align-items: center;
    gap: 10px;
  }
  .cat-name {
    font-weight: 600;
  }
  .cat-bar {
    height: 8px;
    border-radius: 8px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .cat-bar span {
    display: block;
    height: 100%;
    border-radius: 8px;
    background: var(--c);
  }
  .cat-amount {
    font-weight: 650;
    font-size: 13px;
  }
  .cat-pct {
    text-align: right;
  }
  .day-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 14px 6px 6px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .expense {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 6px;
    border: none;
    border-radius: 12px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .expense:hover {
    background: var(--surface-2);
  }
  .exp-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .exp-amount {
    font-weight: 750;
  }
  @container main (max-width: 860px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
