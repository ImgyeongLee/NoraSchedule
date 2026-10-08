<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity } from '@lucide/svelte';
  import { api, type ActivitySummary, type TrackerStatus } from '../lib/api';
  import { colorForName } from '../lib/colors';
  import { fmtDuration, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { ui } from '../lib/state.svelte';
  import { trackState } from '../lib/tracker';

  let summary = $state<ActivitySummary | null>(null);
  let status = $state<TrackerStatus | null>(null);

  async function refresh() {
    summary = await api.activitySummary([today()], 0).catch(() => null);
    status = await api.trackerStatus().catch(() => null);
  }

  onMount(() => {
    refresh();
    const timer = setInterval(refresh, 15_000);
    return () => clearInterval(timer);
  });

  const top = $derived(summary?.apps.slice(0, 3) ?? []);
  const max = $derived(top[0]?.secs ?? 1);
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'tracking')}><span class="w-icon"><Activity size={15} /></span>{t('w.working')}</button>
    <span class="spacer"></span>
    {#if status}
      {@const s = trackState(status.state)}
      <span class="dot" style:background={s.color} title={s.label}></span>
    {/if}
  </div>
  <div class="w-body">
    <div class="total tabular">{fmtDuration(summary?.total ?? 0)}</div>
    <div class="faint small">{t('w.working.today')}</div>
    <div class="apps">
      {#each top as a (a.app)}
        <div class="app">
          <span class="truncate name">{a.app}</span>
          <span class="bar"><span style:width="{(a.secs / max) * 100}%" style:background={colorForName(a.app)}></span></span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .total {
    font-size: 28px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }
  .apps {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 10px;
  }
  .app {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  .name {
    font-weight: 600;
  }
  .bar {
    height: 6px;
    border-radius: 6px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 6px;
  }
</style>
