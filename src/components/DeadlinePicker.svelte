<script lang="ts">
  // Deadline editor: quick picks, a date and an optional time.
  // `compact` shows it behind a small button with a popover (used in the quick-add bar).
  import { CalendarClock, Clock, X } from '@lucide/svelte';
  import { fly } from 'svelte/transition';
  import { fmtDeadline, quickDates } from '../lib/deadline';
  import { t } from '../lib/i18n.svelte';

  let {
    due = $bindable(),
    time = $bindable(),
    compact = false,
  }: { due: string | null; time: string | null; compact?: boolean } = $props();

  let open = $state(false);
  let root: HTMLDivElement;

  function pick(date: string) {
    due = date;
  }

  function clear() {
    due = null;
    time = null;
    open = false;
  }

  function toggleTime(on: boolean) {
    time = on ? (time ?? '18:00') : null;
  }

  function onWindowDown(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onmousedown={onWindowDown} />

{#snippet editor()}
  <div class="quick">
    {#each quickDates() as q (q.label)}
      <button type="button" class="pick" class:active={due === q.date} onclick={() => pick(q.date)}>{q.label}</button>
    {/each}
  </div>
  <div class="inputs">
    <label class="date">
      <CalendarClock size={16} />
      <input
        type="date"
        class="input"
        value={due ?? ''}
        onchange={(e) => (due = e.currentTarget.value || null)}
        aria-label={t('dl.pickDate')}
      />
    </label>
    <label class="time-toggle">
      <input type="checkbox" class="switch" checked={time !== null} disabled={!due} onchange={(e) => toggleTime(e.currentTarget.checked)} />
      <span>{t('dl.addTime')}</span>
    </label>
    {#if time !== null && due}
      <label class="date">
        <Clock size={16} />
        <input type="time" class="input" value={time} onchange={(e) => (time = e.currentTarget.value || null)} />
      </label>
    {/if}
  </div>
{/snippet}

<div class="deadline" class:compact bind:this={root}>
  {#if compact}
    <button type="button" class="trigger" class:set={!!due} onclick={() => (open = !open)} title={t('dl.deadline')}>
      <CalendarClock size={16} />
      <span>{due ? fmtDeadline(due, time) : t('dl.set')}</span>
      {#if due}
        <span
          class="clear"
          role="button"
          tabindex="0"
          aria-label={t('dl.clear')}
          onclick={(e) => { e.stopPropagation(); clear(); }}
          onkeydown={(e) => e.key === 'Enter' && clear()}><X size={13} /></span
        >
      {/if}
    </button>
    {#if open}
      <div class="panel" transition:fly={{ y: -6, duration: 140 }}>
        {@render editor()}
        <div class="panel-foot">
          {#if due}<button type="button" class="btn small ghost" onclick={clear}>{t('dl.clear')}</button>{/if}
          <span class="spacer"></span>
          <button type="button" class="btn small primary" onclick={() => (open = false)}>{t('dl.done')}</button>
        </div>
      </div>
    {/if}
  {:else}
    {@render editor()}
    {#if due}
      <button type="button" class="btn small ghost remove" onclick={clear}><X size={14} /> {t('dl.clear')}</button>
    {/if}
  {/if}
</div>

<style>
  .deadline {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .deadline.compact {
    position: relative;
    display: block;
  }
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pick {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--muted);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
  }
  .pick:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
  .pick.active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-text);
  }
  .inputs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .date {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--faint);
  }
  .date :global(svg) {
    position: absolute;
    left: 11px;
    pointer-events: none;
  }
  .date .input {
    padding-left: 34px;
    width: auto;
  }
  .time-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
  }
  .remove {
    align-self: flex-start;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 10px;
    border: 1px dashed var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--muted);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }
  .trigger:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
  .trigger.set {
    border-style: solid;
    border-color: transparent;
    background: var(--primary-soft);
    color: var(--primary);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
  }
  .clear:hover {
    background: color-mix(in srgb, var(--primary) 20%, transparent);
  }
  .panel {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 20;
    width: 330px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-lg);
  }
  .panel-foot {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
