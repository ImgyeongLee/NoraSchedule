<script lang="ts">
  import { ChartColumn } from '@lucide/svelte';
  import BarChart from '../components/BarChart.svelte';
  import { api, type ActivitySummary } from '../lib/api';
  import { addDays, fmt, fmtDuration, fmtHours, range, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';

  let summary = $state<ActivitySummary | null>(null);
  let height = $state(120);
  const days = $derived(range(addDays(today(), -6), 7));

  $effect(() => {
    data.version;
    load(api.activitySummary(days, 0), null).then((s) => (summary = s));
  });

  const chart = $derived(days.map((d, i) => ({ label: fmt(d, { weekday: 'short' }), value: summary?.per_day[i] ?? 0 })));
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'analytics')}><span class="w-icon"><ChartColumn size={15} /></span>{t('w.weekChart')}</button>
    <span class="spacer"></span>
    <span class="muted small tabular">{fmtDuration(summary?.total ?? 0)}</span>
  </div>
  <div class="chart" bind:clientHeight={height}>
    <BarChart data={chart} format={fmtHours} height={Math.max(80, height)} />
  </div>
</div>

<style>
  .chart {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
