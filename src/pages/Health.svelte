<script lang="ts">
  // Health: diet and exercise, medicines, sleep and how the day felt, one tab each, for the
  // same chosen day. The strip on top sums up that day across all four.
  import { onMount } from 'svelte';
  import { BedDouble, ChevronLeft, ChevronRight, HeartPulse, Pill, Salad, Settings2 } from '@lucide/svelte';
  import DateField from '../components/DateField.svelte';
  import DietPanel from '../components/health/DietPanel.svelte';
  import MedsPanel from '../components/health/MedsPanel.svelte';
  import SleepPanel from '../components/health/SleepPanel.svelte';
  import ConditionPanel from '../components/health/ConditionPanel.svelte';
  import { api, type Condition, type Medication, type MedLog, type Meal, type SleepLog, type Workout } from '../lib/api';
  import { addDays, today } from '../lib/dates';
  import { dayTotals, fmtNum, healthSettings, loadHealthSettings } from '../lib/health.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, load } from '../lib/state.svelte';
  import { MOOD_ICONS, doseCount, fmtDuration, scaleColor, scaleLabel, sleepMinutes } from '../lib/wellness';

  type Tab = 'diet' | 'meds' | 'sleep' | 'condition';
  const TABS: { id: Tab; icon: typeof Salad }[] = [
    { id: 'diet', icon: Salad },
    { id: 'meds', icon: Pill },
    { id: 'sleep', icon: BedDouble },
    { id: 'condition', icon: HeartPulse },
  ];
  // The open tab is remembered in this browser.
  const TAB_KEY = 'nora.health.tab';
  function readTab(): Tab {
    try {
      const v = localStorage.getItem(TAB_KEY);
      return TABS.some((x) => x.id === v) ? (v as Tab) : 'diet';
    } catch {
      return 'diet';
    }
  }
  let tab = $state<Tab>(readTab());
  $effect(() => {
    const value = tab;
    try {
      localStorage.setItem(TAB_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });

  let day = $state(today());
  const isToday = $derived(day === today());
  let diet = $state<ReturnType<typeof DietPanel>>();

  onMount(() => {
    if (!healthSettings.loaded) loadHealthSettings();
  });

  // ---- the day at a glance
  let meals = $state<Meal[]>([]);
  let workouts = $state<Workout[]>([]);
  let meds = $state<Medication[]>([]);
  let medLogs = $state<MedLog[]>([]);
  let night = $state<SleepLog | null>(null);
  let feeling = $state<Condition | null>(null);
  $effect(() => {
    data.version;
    const d = day;
    load(api.mealsBetween(d, d), []).then((x) => (meals = x));
    load(api.workoutsBetween(d, d), []).then((x) => (workouts = x));
    load(api.medications(), []).then((x) => (meds = x));
    load(api.medLogsBetween(d, d), []).then((x) => (medLogs = x));
    load(api.sleepBetween(d, d), []).then((x) => (night = x[0] ?? null));
    load(api.conditionsBetween(d, d), []).then((x) => (feeling = x[0] ?? null));
  });
  const totals = $derived(dayTotals(meals, workouts, healthSettings.goal));
  const doses = $derived(doseCount(meds, medLogs, day));
  const MoodIcon = $derived(feeling?.mood ? MOOD_ICONS[feeling.mood - 1] : HeartPulse);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.health')}</h1>
      <p class="sub">{t('wl.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <div class="day-nav">
      <button class="icon-btn" onclick={() => (day = addDays(day, -1))} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
      <div class="day-field"><DateField value={day} label={t('common.date')} onchange={(v) => v && (day = v)} /></div>
      <button class="icon-btn" onclick={() => (day = addDays(day, 1))} aria-label={t('common.next')}><ChevronRight size={18} /></button>
      {#if !isToday}<button class="btn small" onclick={() => (day = today())}>{t('common.today')}</button>{/if}
    </div>
    {#if tab === 'diet'}
      <button class="btn" onclick={() => diet?.openSettings()}><Settings2 size={16} /> {t('hl.settings')}</button>
    {/if}
  </div>

  <div class="glance" role="tablist" aria-label={t('nav.health')}>
    <button class="tile" class:active={tab === 'diet'} role="tab" aria-selected={tab === 'diet'} onclick={() => (tab = 'diet')} style:--c="var(--primary)">
      <span class="tile-icon"><Salad size={18} /></span>
      <span class="tile-text">
        <span class="tile-label">{t('wl.tab.diet')}</span>
        <span class="tile-value tabular" class:over={totals.left < 0}>
          {totals.eaten || totals.burned ? `${fmtNum(Math.abs(totals.left))} kcal` : '–'}
        </span>
        <span class="tile-sub">{totals.eaten || totals.burned ? (totals.left >= 0 ? t('hl.left') : t('hl.over')) : t('wl.notLogged')}</span>
      </span>
    </button>
    <button class="tile" class:active={tab === 'meds'} role="tab" aria-selected={tab === 'meds'} onclick={() => (tab = 'meds')} style:--c="var(--success)">
      <span class="tile-icon"><Pill size={18} /></span>
      <span class="tile-text">
        <span class="tile-label">{t('wl.tab.meds')}</span>
        <span class="tile-value tabular">{doses.due ? `${doses.taken}/${doses.due}` : '–'}</span>
        <span class="tile-sub">{doses.due ? (doses.taken === doses.due ? t('wl.allTaken') : t('wl.dosesLeft', { n: doses.due - doses.taken })) : t('wl.noDoses')}</span>
      </span>
    </button>
    <button class="tile" class:active={tab === 'sleep'} role="tab" aria-selected={tab === 'sleep'} onclick={() => (tab = 'sleep')} style:--c="var(--long-break)">
      <span class="tile-icon"><BedDouble size={18} /></span>
      <span class="tile-text">
        <span class="tile-label">{t('wl.tab.sleep')}</span>
        <span class="tile-value tabular">{night ? fmtDuration(sleepMinutes(night.bed, night.wake)) : '–'}</span>
        <span class="tile-sub">{night ? `${night.bed} – ${night.wake}` : t('wl.notLogged')}</span>
      </span>
    </button>
    <button class="tile" class:active={tab === 'condition'} role="tab" aria-selected={tab === 'condition'} onclick={() => (tab = 'condition')}
      style:--c={feeling?.mood ? scaleColor('mood', feeling.mood) : 'var(--warning)'}>
      <span class="tile-icon"><MoodIcon size={18} /></span>
      <span class="tile-text">
        <span class="tile-label">{t('wl.tab.condition')}</span>
        <span class="tile-value">{feeling?.mood ? scaleLabel('mood', feeling.mood) : '–'}</span>
        <span class="tile-sub">{feeling?.tags.length ? t('wl.tagCount', { n: feeling.tags.length }) : t('wl.notLogged')}</span>
      </span>
    </button>
  </div>

  {#if tab === 'diet'}
    <DietPanel bind:this={diet} bind:day />
  {:else if tab === 'meds'}
    <MedsPanel bind:day />
  {:else if tab === 'sleep'}
    <SleepPanel bind:day />
  {:else}
    <ConditionPanel bind:day />
  {/if}
</div>

<style>
  .day-nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .day-field {
    width: 210px;
  }
  .glance {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }
  .tile {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 12px 14px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.12s,
      background 0.12s;
  }
  .tile:hover {
    border-color: color-mix(in srgb, var(--c) 50%, var(--border));
  }
  .tile.active {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 7%, var(--surface));
  }
  .tile-icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: 12px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .tile-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .tile-label {
    font-size: 12px;
    font-weight: 650;
    color: var(--muted);
  }
  .tile-value {
    font-size: 17px;
    font-weight: 800;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tile-value.over {
    color: var(--danger);
  }
  .tile-sub {
    font-size: 11.5px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  @container main (max-width: 900px) {
    .glance {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
