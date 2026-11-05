<script lang="ts">
  // Plans made well ahead, as a table: every event from today to N months out (repeating ones
  // only if asked), with the date, how far away it is, time, place and tags.
  import { MapPin, Repeat } from '@lucide/svelte';
  import { api, type CalEvent } from '../lib/api';
  import { calPrefs, setCalPrefs } from '../lib/calPrefs.svelte';
  import { hex } from '../lib/colors';
  import { addMonths, diffDays, eventSpan, fmt, isAllDayLane, timeOf, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load } from '../lib/state.svelte';
  import { eventColor, tagsOf } from '../lib/tags.svelte';

  let { onopen, compact = false }: { onopen: (e: CalEvent) => void; compact?: boolean } = $props();

  const RANGES = [1, 3, 6, 12];
  let events = $state<CalEvent[]>([]);
  const todayStr = today();

  $effect(() => {
    data.version;
    const to = addMonths(todayStr, calPrefs.upcomingMonths);
    load(api.eventsBetween(todayStr, to), []).then((list) => (events = list));
  });

  const rows = $derived(
    events
      .filter((e) => (calPrefs.upcomingRepeat || !e.repeat) && eventSpan(e)[1] >= todayStr)
      .sort((a, b) => a.start.localeCompare(b.start)),
  );

  /** "Today", "D-3"; ongoing multi-day events count as today. */
  function away(e: CalEvent) {
    const n = Math.max(0, diffDays(eventSpan(e)[0], todayStr));
    return n === 0 ? t('dd.today') : `D-${n}`;
  }
  function when(e: CalEvent) {
    const [a, b] = eventSpan(e);
    const day = (d: string) => fmt(d, { month: 'short', day: 'numeric', weekday: 'short' });
    return a === b ? day(a) : `${day(a)} – ${day(b)}`;
  }
</script>

<div class="upcoming" class:compact>
  <div class="controls">
    <div class="segmented ranges" role="group" aria-label={t('up.range')}>
      {#each RANGES as m (m)}
        <button class:active={calPrefs.upcomingMonths === m} onclick={() => setCalPrefs({ upcomingMonths: m })}>{t('up.months', { n: m })}</button>
      {/each}
    </div>
    <span class="spacer"></span>
    <label class="repeat-opt small">
      <input type="checkbox" class="switch" checked={calPrefs.upcomingRepeat} onchange={(e) => setCalPrefs({ upcomingRepeat: e.currentTarget.checked })} />
      {t('up.includeRepeat')}
    </label>
  </div>

  {#if rows.length}
    <div class="scroll">
      <table>
        <thead>
          <tr>
            <th class="c-date">{t('common.date')}</th>
            <th class="c-away">D-day</th>
            <th class="c-time">{t('up.time')}</th>
            <th class="c-title">{t('up.event')}</th>
            <th class="c-place">{t('common.location')}</th>
            <th class="c-tags">{t('tag.title')}</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as e (`${e.id}-${e.occurrence ?? ''}`)}
            {@const n = diffDays(eventSpan(e)[0], todayStr)}
            <tr class:soon={n <= 7} class:cancelled={e.cancelled} onclick={() => onopen(e)} title={e.title}>
              <td class="c-date tabular">{when(e)}</td>
              <td class="c-away tabular"><span class="away">{away(e)}</span></td>
              <td class="c-time tabular">{isAllDayLane(e) ? t('common.allDay') : timeOf(e.start)}</td>
              <td class="c-title">
                <span class="title-cell">
                  <span class="dot" style:background={hex(eventColor(e))}></span>
                  <span class="truncate">{e.title}</span>
                  {#if e.repeat}<Repeat size={12} />{/if}
                </span>
              </td>
              <td class="c-place">{#if e.location}<span class="truncate muted"><MapPin size={12} /> {e.location}</span>{/if}</td>
              <td class="c-tags">
                <span class="tags">{#each tagsOf(e) as tag (tag.id)}<span class="tag" style:--c={hex(tag.color)}>{tag.name}</span>{/each}</span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="muted small none">{t('up.empty', { n: calPrefs.upcomingMonths })}</p>
  {/if}
</div>

<style>
  .upcoming {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    height: 100%;
  }
  .controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .ranges button {
    height: 26px;
    padding: 0 10px;
    font-size: 12.5px;
  }
  .repeat-opt {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    cursor: pointer;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 8px 10px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 12px;
    font-weight: 650;
    text-align: left;
    white-space: nowrap;
  }
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
  td {
    padding: 8px 10px;
    border-top: 1px solid var(--border);
    vertical-align: middle;
    white-space: nowrap;
  }
  .c-title {
    width: 100%;
    max-width: 0;
  }
  .title-cell {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-weight: 600;
  }
  .title-cell :global(svg) {
    flex: none;
    color: var(--faint);
  }
  .dot {
    width: 9px;
    height: 9px;
    flex: none;
    border-radius: 50%;
  }
  .away {
    display: inline-block;
    min-width: 46px;
    padding: 2px 8px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 700;
    text-align: center;
  }
  tr.soon .away {
    color: var(--primary);
    background: var(--primary-soft);
  }
  tr.cancelled .title-cell {
    text-decoration: line-through;
    color: var(--faint);
  }
  .c-place {
    max-width: 180px;
  }
  .c-place span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 180px;
  }
  .tags {
    display: flex;
    gap: 4px;
  }
  .tag {
    padding: 1px 7px;
    border-radius: var(--radius-pill);
    font-size: 11px;
    font-weight: 650;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 13%, transparent);
  }
  .none {
    margin: 0;
    padding: 18px 4px;
    text-align: center;
  }
  /* Narrow (a small tile or the overlay panel): only the essentials. */
  @container (max-width: 560px) {
    .c-place,
    .c-tags {
      display: none;
    }
  }
  @container (max-width: 380px) {
    .c-time {
      display: none;
    }
    .repeat-opt {
      display: none;
    }
  }
</style>
