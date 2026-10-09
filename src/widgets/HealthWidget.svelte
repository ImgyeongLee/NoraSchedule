<script lang="ts">
  import { onMount } from 'svelte';
  import { Flame, Salad, Utensils } from '@lucide/svelte';
  import { api, type Meal, type Workout } from '../lib/api';
  import { today } from '../lib/dates';
  import { dayTotals, fmtKcal, fmtNum, healthSettings, loadHealthSettings } from '../lib/health.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';

  let meals = $state<Meal[]>([]);
  let workouts = $state<Workout[]>([]);

  onMount(() => {
    if (!healthSettings.loaded) loadHealthSettings();
  });

  $effect(() => {
    data.version;
    const d = today();
    load(api.mealsBetween(d, d), []).then((m) => (meals = m));
    load(api.workoutsBetween(d, d), []).then((w) => (workouts = w));
  });

  const totals = $derived(dayTotals(meals, workouts, healthSettings.goal));
  const allowance = $derived(healthSettings.goal + totals.burned);
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'health')}><span class="w-icon"><Salad size={15} /></span>{t('w.health')}</button>
  </div>
  <div class="w-body">
    <div class="total tabular" class:over={totals.left < 0}>{fmtKcal(Math.abs(totals.left))}</div>
    <div class="faint small">{totals.left >= 0 ? t('w.health.left') : t('w.health.over')}</div>
    <div class="bar" class:over={totals.left < 0}>
      <span style:width="{Math.min(100, (totals.eaten / Math.max(1, allowance)) * 100)}%"></span>
    </div>
    <div class="chips">
      <span class="chip-c" style:--c="var(--primary)"><Utensils size={12} /> {fmtNum(totals.eaten)}</span>
      <span class="chip-c" style:--c="var(--warning)"><Flame size={12} /> {fmtNum(totals.burned)}</span>
    </div>
  </div>
</div>

<style>
  .total {
    font-size: 26px;
    font-weight: 750;
    letter-spacing: -0.02em;
    color: var(--success);
  }
  .total.over {
    color: var(--danger);
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
    background: var(--primary);
  }
  .bar.over span {
    background: var(--danger);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .chip-c {
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
