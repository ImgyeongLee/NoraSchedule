<script lang="ts">
  // TRPG page → Report: how much, when, which systems and whose scenarios, for one year or all time.
  import { BookCopy, Crown, ScrollText, Sparkles } from '@lucide/svelte';
  import StatCard from './StatCard.svelte';
  import BarChart from './BarChart.svelte';
  import BarList from './BarList.svelte';
  import Select from './Select.svelte';
  import type { TrpgEntry } from '../lib/api';
  import { colorForName } from '../lib/colors';
  import { today } from '../lib/dates';
  import { rolesOf } from '../lib/trpgRoles';
  import { intlLocale, t } from '../lib/i18n.svelte';

  let { entries }: { entries: TrpgEntry[] } = $props();

  const played = $derived(entries.filter((e) => e.kind === 'played'));
  const years = $derived([...new Set(played.map((e) => e.date?.slice(0, 4)).filter((y): y is string => !!y))].sort().reverse());
  /** '' means all time. */
  let year = $state(today().slice(0, 4));
  // Fall back to all time when the chosen year has no sessions.
  const scope = $derived(year && years.includes(year) ? year : '');
  const inScope = $derived(scope ? played.filter((e) => e.date?.startsWith(scope)) : played);

  // A session can be both GM and PL (and have HO/PC slots too).
  const gm = $derived(inScope.filter((e) => rolesOf(e.role).includes('gm')).length);
  const pl = $derived(inScope.filter((e) => rolesOf(e.role).includes('pl')).length);
  const noRole = $derived(inScope.filter((e) => !rolesOf(e.role).length).length);
  const wishlist = $derived(entries.filter((e) => e.kind === 'wishlist').length);
  const rulebooks = $derived(entries.filter((e) => e.kind === 'rulebook').length);
  const scenarioBooks = $derived(entries.filter((e) => e.kind === 'scenario_book').length);

  /** Sessions per month of the chosen year, or per year for all time. */
  const timeline = $derived.by(() => {
    if (scope) {
      return Array.from({ length: 12 }, (_, m) => {
        const prefix = `${scope}-${String(m + 1).padStart(2, '0')}`;
        return { label: String(m + 1), value: inScope.filter((e) => e.date?.startsWith(prefix)).length };
      });
    }
    return [...years].reverse().map((y) => ({ label: y, value: played.filter((e) => e.date?.startsWith(y)).length }));
  });

  /** The most common values of `field`, with how many sessions each has. */
  function top(field: 'system' | 'writer', n = 6) {
    const counts = new Map<string, number>();
    for (const e of inScope) if (e[field]) counts.set(e[field], (counts.get(e[field]) ?? 0) + 1);
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], 'ko'))
      .slice(0, n)
      .map(([label, value]) => ({ label, value, color: colorForName(label) }));
  }
  const bySystem = $derived(top('system'));
  const byWriter = $derived(top('writer'));
  const monthName = (m: number) => new Date(2000, m - 1, 1).toLocaleString(intlLocale(), { month: 'long' });
  const busiestMonth = $derived(scope ? timeline.reduce((a, b) => (b.value > a.value ? b : a), timeline[0]) : null);
</script>

<div class="report">
  <div class="report-head">
    <h3>{scope ? t('trpg.reportYear', { year: scope }) : t('trpg.reportAll')}</h3>
    <span class="spacer"></span>
    <div class="year-pick">
      <Select
        value={scope}
        options={[{ value: '', label: t('trpg.allTime') }, ...years.map((y) => ({ value: y, label: t('trpg.year', { year: y }) }))]}
        onchange={(v) => (year = v)}
        label={t('trpg.reportPeriod')}
      />
    </div>
  </div>

  <div class="stats">
    <StatCard
      label={t('trpg.statSessions')}
      value={String(inScope.length)}
      sub={busiestMonth?.value ? t('trpg.statBusiest', { m: monthName(Number(busiestMonth.label)), n: busiestMonth.value }) : ''}
      icon={ScrollText}
    />
    <StatCard
      label={t('trpg.statRoles')}
      value={`GM ${gm} · PL ${pl}`}
      sub={noRole ? t('trpg.statNoRole', { n: noRole }) : ''}
      icon={Crown}
      tint="var(--warning)"
    />
    <StatCard label={t('trpg.wishlist')} value={String(wishlist)} sub={t('trpg.statWishlist')} icon={Sparkles} tint="#f2668b" />
    <StatCard
      label={t('trpg.statLibrary')}
      value={String(rulebooks + scenarioBooks)}
      sub={t('trpg.statBooks', { r: rulebooks, s: scenarioBooks })}
      icon={BookCopy}
      tint="#22b5bf"
    />
  </div>

  {#if inScope.length}
    <div class="card spaced">
      <h3>{scope ? t('trpg.perMonth') : t('trpg.perYear')}</h3>
      <BarChart data={timeline} height={180} />
    </div>
    <div class="grid-2">
      <div class="card">
        <h3>{t('trpg.topSystems')}</h3>
        {#if bySystem.length}<BarList items={bySystem} format={(v) => t('trpg.times', { n: v })} />{:else}<p class="muted small">{t('trpg.noData')}</p>{/if}
      </div>
      <div class="card">
        <h3>{t('trpg.topWriters')}</h3>
        {#if byWriter.length}<BarList items={byWriter} format={(v) => t('trpg.times', { n: v })} />{:else}<p class="muted small">{t('trpg.noData')}</p>{/if}
      </div>
    </div>
  {:else}
    <div class="card empty">
      <span class="empty-icon"><ScrollText size={30} /></span>
      <p>{t('trpg.reportEmpty')}</p>
    </div>
  {/if}
</div>

<style>
  .report-head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .year-pick {
    width: 160px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(170px, 100%), 1fr));
    gap: 14px;
    margin-bottom: 16px;
  }
  .card h3 {
    margin-bottom: 10px;
  }
  .spaced {
    margin-bottom: 16px;
  }
</style>
