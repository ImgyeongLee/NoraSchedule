<script lang="ts">
  // Finished commissions grouped by year, month or kind, with totals, so income is easy to
  // check later (e.g. for a tax return). The list goes by delivery date; "received" totals go
  // by the date each payment came in.
  import { CalendarCheck, ClipboardCopy, HandCoins, Receipt, Wallet } from '@lucide/svelte';
  import BarChart from './BarChart.svelte';
  import StatCard from './StatCard.svelte';
  import type { Commission } from '../lib/api';
  import { feeOf, netOf, netPayment, paidOf, unpaidOf } from '../lib/commissions.svelte';
  import { fmt, today, tsToDate } from '../lib/dates';
  import { fmtMoney, fmtMoneyShort } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { toast } from '../lib/state.svelte';

  let {
    commissions,
    clientName,
    onopen,
  }: { commissions: Commission[]; clientName: (c: Commission) => string; onopen: (c: Commission) => void } = $props();

  type Group = 'year' | 'month' | 'kind';
  // Per-viewer conveniences, remembered in this browser only.
  const PREFS_KEY = 'nora.commissions.ledger';
  function readGroup(): Group {
    try {
      const g = localStorage.getItem(PREFS_KEY);
      return g === 'year' || g === 'kind' ? g : 'month';
    } catch {
      return 'month';
    }
  }
  let group = $state<Group>(readGroup());
  $effect(() => {
    const value = group;
    try {
      localStorage.setItem(PREFS_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });

  const thisYear = today().slice(0, 4);
  /** When a commission was finished (older ones without a date: when it was added). */
  const doneOn = (c: Commission) => c.delivered ?? tsToDate(c.created_at);
  const done = $derived(commissions.filter((c) => c.status === 'done'));

  const years = $derived(
    [...new Set([thisYear, ...done.map((c) => doneOn(c).slice(0, 4)), ...commissions.flatMap((c) => c.payments.map((p) => p.date.slice(0, 4)))])]
      .sort()
      .reverse(),
  );
  let year = $state(thisYear);

  const shown = $derived(group === 'year' ? done : done.filter((c) => doneOn(c).startsWith(year)));

  const groups = $derived.by(() => {
    const keyOf = (c: Commission) => (group === 'year' ? doneOn(c).slice(0, 4) : group === 'month' ? doneOn(c).slice(0, 7) : c.kind.trim());
    const map = new Map<string, Commission[]>();
    for (const c of shown) map.set(keyOf(c), [...(map.get(keyOf(c)) ?? []), c]);
    const list = [...map.entries()].map(([key, items]) => ({
      key,
      label:
        group === 'year' ? t('trpg.year', { year: key }) : group === 'month' ? fmt(`${key}-01`, { year: 'numeric', month: 'long' }) : key || t('cm.kindNone'),
      items: items.sort((a, b) => doneOn(b).localeCompare(doneOn(a))),
      price: items.reduce((s, c) => s + c.price, 0),
      net: items.reduce((s, c) => s + netOf(c), 0),
      paid: items.reduce((s, c) => s + paidOf(c), 0),
    }));
    // Newest year/month first; kinds by how much they earned.
    return group === 'kind' ? list.sort((a, b) => b.price - a.price || a.label.localeCompare(b.label)) : list.sort((a, b) => b.key.localeCompare(a.key));
  });

  // Totals for the chosen year (all years when grouped by year).
  const inScope = (date: string) => group === 'year' || date.startsWith(year);
  // Money that actually came in: each payment after the platform fee.
  const payments = $derived(
    commissions.flatMap((c) => c.payments.map((p) => ({ ...p, net: netPayment(c, p.amount) }))).filter((p) => inScope(p.date)),
  );
  const received = $derived(payments.reduce((s, p) => s + p.net, 0));
  const shownPrice = $derived(shown.reduce((s, c) => s + c.price, 0));
  const shownFees = $derived(shown.reduce((s, c) => s + feeOf(c), 0));
  const shownUnpaid = $derived(shown.reduce((s, c) => s + unpaidOf(c), 0));
  const perMonth = $derived(
    Array.from({ length: 12 }, (_, m) => {
      const prefix = `${year}-${String(m + 1).padStart(2, '0')}`;
      return { label: String(m + 1), value: payments.filter((p) => p.date.startsWith(prefix)).reduce((s, p) => s + p.net, 0) };
    }),
  );

  /** Copies the list as tab-separated text, which pastes into Excel or Google Sheets as a table. */
  function copyTable() {
    const clean = (s: string) => s.replace(/[\t\n\r]+/g, ' ').trim();
    const rows = [
      [t('cm.colDone'), t('cm.title'), t('cm.client'), t('cm.kind'), t('cm.platform'), t('cm.price'), t('cm.fee'), t('cm.netLabel'), t('cm.colPaid'), t('cm.colPaidOn')],
      ...shown
        .slice()
        .sort((a, b) => doneOn(a).localeCompare(doneOn(b)))
        .map((c) => [
          doneOn(c),
          clean(c.title),
          clean(clientName(c)),
          clean(c.kind),
          clean(c.platform),
          String(c.price),
          String(Math.round(feeOf(c))),
          String(Math.round(netOf(c))),
          String(paidOf(c)),
          c.payments.map((p) => p.date).join(', '),
        ]),
      ['', t('cm.colTotal'), '', '', '', String(shownPrice), String(Math.round(shownFees)), String(Math.round(shownPrice - shownFees)), String(shown.reduce((s, c) => s + paidOf(c), 0)), ''],
    ];
    navigator.clipboard?.writeText(rows.map((r) => r.join('\t')).join('\n')).catch(() => {});
    toast(t('cm.tableCopied', { n: shown.length }), 'success');
  }
</script>

<div class="ledger">
  <div class="controls">
    <div class="segmented" role="group" aria-label={t('cm.groupBy')}>
      <button class:active={group === 'year'} onclick={() => (group = 'year')}>{t('cm.byYear')}</button>
      <button class:active={group === 'month'} onclick={() => (group = 'month')}>{t('cm.byMonth')}</button>
      <button class:active={group === 'kind'} onclick={() => (group = 'kind')}>{t('cm.byKind')}</button>
    </div>
    {#if group !== 'year'}
      <div class="segmented years" role="group" aria-label={t('cm.year')}>
        {#each years as y (y)}
          <button class:active={year === y} onclick={() => (year = y)}>{t('trpg.year', { year: y })}</button>
        {/each}
      </div>
    {/if}
    <span class="spacer"></span>
    <button class="btn" onclick={copyTable} disabled={!shown.length} title={t('cm.copyTableHint')}><ClipboardCopy size={16} /> {t('cm.copyTable')}</button>
  </div>

  <div class="stats">
    <StatCard label={t('cm.statDone')} value={String(shown.length)} sub={group === 'year' ? t('cm.allYears') : t('trpg.year', { year })} icon={CalendarCheck} tint="var(--success)" />
    <StatCard
      label={t('cm.statDoneTotal')}
      value={fmtMoney(shownPrice)}
      sub={shownFees > 0 ? t('cm.netOfTotal', { net: fmtMoney(shownPrice - shownFees), fee: fmtMoney(shownFees) }) : t('cm.basisDone')}
      icon={Receipt}
    />
    <StatCard label={t('cm.statReceived')} value={fmtMoney(received)} sub={t('cm.basisPaid', { n: payments.length })} icon={Wallet} tint="var(--long-break)" />
    <StatCard label={t('cm.statUnpaid')} value={fmtMoney(shownUnpaid)} sub={t('cm.unpaidDone')} icon={HandCoins} tint={shownUnpaid > 0 ? 'var(--warning)' : 'var(--success)'} />
  </div>

  {#if group !== 'year'}
    <div class="card">
      <h3>{t('cm.receivedPerMonth', { year })}</h3>
      <BarChart data={perMonth} color="var(--success)" height={170} format={(v) => fmtMoneyShort(v)} />
    </div>
  {/if}

  {#each groups as g (g.key)}
    <section class="card group">
      <div class="group-head">
        <h3>{g.label}</h3>
        <span class="muted small">{t('cm.statCount', { n: g.items.length })}</span>
        <span class="spacer"></span>
        <span class="small muted">{g.net < g.price ? t('cm.netGroup', { net: fmtMoney(g.net) }) : t('cm.paidOf', { paid: fmtMoney(g.paid), price: fmtMoney(g.price) })}</span>
        <span class="group-total tabular">{fmtMoney(g.price)}</span>
      </div>
      {#each g.items as c (c.id)}
        <button class="row" onclick={() => onopen(c)}>
          <span class="date muted small tabular">{fmt(doneOn(c), { month: 'short', day: 'numeric' })}</span>
          <span class="row-title">
            <span class="strong truncate">{c.title}</span>
            <span class="muted small truncate">{[clientName(c), group === 'kind' ? '' : c.kind, c.platform].filter(Boolean).join(' · ')}</span>
          </span>
          {#if unpaidOf(c) > 0}<span class="owes small">{t('cm.left', { amount: fmtMoney(unpaidOf(c)) })}</span>{/if}
          <span class="amounts">
            <span class="amount tabular">{fmtMoney(c.price)}</span>
            {#if feeOf(c) > 0}<span class="faint small tabular">{t('cm.netShort', { amount: fmtMoney(netOf(c)) })}</span>{/if}
          </span>
        </button>
      {/each}
    </section>
  {:else}
    <div class="card empty">
      <p>{t('cm.ledgerEmpty')}</p>
    </div>
  {/each}
</div>

<style>
  .ledger {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .years {
    max-width: 100%;
    overflow-x: auto;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr));
    gap: 14px;
  }
  .card h3 {
    margin: 0;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .group-head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 6px 12px;
    padding-bottom: 8px;
    margin-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }
  .group-total {
    font-size: 16px;
    font-weight: 750;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-width: 0;
    padding: 8px 6px;
    border: none;
    border-radius: 10px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .date {
    width: 56px;
    flex: none;
  }
  .row-title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .strong {
    font-weight: 650;
  }
  .amount {
    flex: none;
    font-weight: 700;
  }
  .owes {
    flex: none;
    color: var(--warning);
  }
  .amounts {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    flex: none;
  }
</style>
