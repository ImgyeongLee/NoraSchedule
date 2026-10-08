<script lang="ts">
  import { ChevronLeft, ChevronRight } from '@lucide/svelte';
  import type { CalEvent } from '../lib/api';
  import { addMonths, covers, fmt, monthStart, range, today, weekStart, weekdayNames } from '../lib/dates';
  import { hideHoverCard, showHoverCard } from '../lib/hovercard.svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    selected,
    onpick,
    marks = new Set<string>(),
    events = [],
    min,
  }: {
    selected: string;
    onpick: (d: string) => void;
    marks?: Set<string>;
    /** When given, hovering a day previews its events. */
    events?: CalEvent[];
    /** Days before this date cannot be picked. */
    min?: string;
  } = $props();

  let shownOverride = $state<string | null>(null);
  const shown = $derived(shownOverride ?? monthStart(selected));
  const days = $derived(range(weekStart(shown), 42));
  const todayStr = $derived(today());

  $effect(() => {
    // Follow the selection when it moves to another month.
    selected;
    shownOverride = null;
  });
</script>

<div class="mini">
  <div class="head">
    <span class="title">{fmt(shown, { month: 'long', year: 'numeric' })}</span>
    <button class="icon-btn" onclick={() => (shownOverride = addMonths(shown, -1))} aria-label={t('common.previous')}><ChevronLeft size={16} /></button>
    <button class="icon-btn" onclick={() => (shownOverride = addMonths(shown, 1))} aria-label={t('common.next')}><ChevronRight size={16} /></button>
  </div>
  <div class="grid">
    {#each weekdayNames('narrow') as d, i (i)}
      <span class="dow">{d}</span>
    {/each}
    {#each days as day (day)}
      <button
        class="day"
        class:other={day.slice(0, 7) !== shown.slice(0, 7)}
        class:today={day === todayStr}
        class:selected={day === selected}
        disabled={!!min && day < min}
        onclick={() => { hideHoverCard(); onpick(day); }}
        onmouseenter={(e) => showHoverCard(e.currentTarget, events.filter((ev) => covers(ev, day)), fmt(day, { month: 'long', day: 'numeric', weekday: 'short' }))}
        onmouseleave={hideHoverCard}
      >
        {Number(day.slice(8))}
        {#if marks.has(day)}<span class="mark"></span>{/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    margin-bottom: 6px;
  }
  .title {
    flex: 1;
    font-weight: 650;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 2px;
  }
  .dow {
    text-align: center;
    font-size: 11px;
    font-weight: 600;
    color: var(--faint);
    padding: 4px 0;
  }
  .day {
    position: relative;
    aspect-ratio: 1;
    min-width: 0;
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
  }
  .day:hover {
    background: var(--surface-2);
  }
  .day:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .day.other {
    color: var(--faint);
  }
  .day.today {
    color: var(--primary);
    font-weight: 750;
  }
  .day.selected {
    background: var(--primary);
    color: var(--primary-text);
  }
  .mark {
    position: absolute;
    bottom: 3px;
    left: 50%;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.6;
  }
</style>
