<script lang="ts">
  import { CalendarDays, Repeat, Target } from '@lucide/svelte';
  import { copyEvent, copyOnHover, eventMenu, pasteAsEvent, pasteOnHover, slotMenu } from '../lib/clipboard.svelte';
  import { isSecondaryClick, openMenu } from '../lib/menu.svelte';
  import type { CalEvent, DDay } from '../lib/api';
  import { hex } from '../lib/colors';
  import { covers, isAllDayLane, monthStart, range, timeOf, today, weekStart, weekdayNames } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';

  let {
    cursor,
    events,
    ddays,
    onopen,
    onnew,
    onday,
    onselect,
    onrange,
  }: {
    cursor: string;
    events: CalEvent[];
    ddays: DDay[];
    onopen: (e: CalEvent) => void;
    onnew: (day: string) => void;
    onday: (day: string) => void;
    onselect: (day: string) => void;
    /** Called after dragging across several days. */
    onrange: (from: string, to: string) => void;
  } = $props();

  // ---- drag across days to create an event
  let dragFrom = $state<string | null>(null);
  let dragTo = $state<string | null>(null);
  const rangeLo = $derived(dragFrom && dragTo ? (dragFrom < dragTo ? dragFrom : dragTo) : null);
  const rangeHi = $derived(dragFrom && dragTo ? (dragFrom < dragTo ? dragTo : dragFrom) : null);
  const inRange = (day: string) => !!rangeLo && !!rangeHi && rangeLo !== rangeHi && day >= rangeLo && day <= rangeHi;

  function dragStart(e: PointerEvent, day: string) {
    if (isSecondaryClick(e) || (e.target as HTMLElement).closest('button')) return;
    dragFrom = dragTo = day;
  }

  function dragEnd() {
    if (rangeLo && rangeHi && rangeLo !== rangeHi) onrange(rangeLo, rangeHi);
    dragFrom = dragTo = null;
  }

  const days = $derived(range(weekStart(monthStart(cursor)), 42));
  const month = $derived(cursor.slice(0, 7));
  const todayStr = $derived(today());
  let gridHeight = $state(600);
  // How many chips fit in a cell (day number takes ~34px, each chip ~24px).
  const capacity = $derived(Math.max(1, Math.floor((gridHeight / 6 - 36) / 24)));

  type Item = { kind: 'dday'; d: DDay } | { kind: 'event'; e: CalEvent };
  function itemsFor(day: string): Item[] {
    return [
      ...ddays.filter((d) => d.date === day).map((d) => ({ kind: 'dday' as const, d })),
      ...events.filter((e) => covers(e, day)).map((e) => ({ kind: 'event' as const, e })),
    ];
  }
</script>

<!-- A right-click during a drag cancels it (capture: item menus stop propagation). -->
<svelte:window onpointerup={dragEnd} oncontextmenucapture={() => (dragFrom = dragTo = null)} />

<div class="month">
  <div class="dow">
    {#each weekdayNames() as d, i (i)}<div>{d}</div>{/each}
  </div>
  <div class="grid" bind:clientHeight={gridHeight}>
    {#each days as day (day)}
      {@const items = itemsFor(day)}
      {@const shown = items.length > capacity ? capacity - 1 : items.length}
      <div
        class="cell"
        class:other={day.slice(0, 7) !== month}
        class:selected={day === cursor}
        class:in-range={inRange(day)}
        onpointerdown={(e) => dragStart(e, day)}
        onpointerenter={() => dragFrom && (dragTo = day)}
        onclick={() => onselect(day)}
        ondblclick={() => onnew(day)}
        role="gridcell"
        tabindex="-1"
        onkeydown={(e) => e.key === 'Enter' && onnew(day)}
        {...pasteOnHover(() => pasteAsEvent(day))}
        oncontextmenu={(e) =>
          openMenu(e, slotMenu(day, undefined, () => onnew(day), [
            { label: t('menu.openDay'), icon: CalendarDays, action: () => onday(day) },
          ]))}
      >
        <button class="num" class:today={day === todayStr} onclick={(e) => { e.stopPropagation(); onday(day); }} title={t('cal.openDay')}>
          {Number(day.slice(8))}
        </button>
        <div class="items">
          {#each items.slice(0, shown) as item, i (i)}
            {#if item.kind === 'dday'}
              <div class="chip dday" style:--c={hex(item.d.color)} title={item.d.title}><Target size={12} /> <span class="truncate">{item.d.title}</span></div>
            {:else if isAllDayLane(item.e)}
              {@const e = item.e}
              <button
                class="chip allday"
                style:--c={hex(e.color)}
                onclick={(ev) => { ev.stopPropagation(); onopen(e); }}
                title={e.title}
                {...copyOnHover(() => copyEvent(e))}
                oncontextmenu={(ev) => openMenu(ev, eventMenu(e, () => onopen(e)))}
              >
                {#if e.repeat}<Repeat size={11} />{/if}
                <span class="truncate">{e.title}</span>
              </button>
            {:else}
              {@const e = item.e}
              <button
                class="chip timed"
                style:--c={hex(e.color)}
                onclick={(ev) => { ev.stopPropagation(); onopen(e); }}
                title="{timeOf(e.start)} {e.title}"
                {...copyOnHover(() => copyEvent(e))}
                oncontextmenu={(ev) => openMenu(ev, eventMenu(e, () => onopen(e)))}
              >
                <span class="dot" style:background="var(--c)"></span>
                <span class="time">{timeOf(e.start)}</span>
                <span class="truncate">{e.title}</span>
                {#if e.repeat}<span class="rep"><Repeat size={11} /></span>{/if}
              </button>
            {/if}
          {/each}
          {#if items.length > shown}
            <button class="more" onclick={(e) => { e.stopPropagation(); onday(day); }}>{t('cal.more', { n: items.length - shown })}</button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .month {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  .dow {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    border-bottom: 1px solid var(--border);
  }
  .dow div {
    padding: 10px 12px;
    font-size: 12px;
    font-weight: 650;
    color: var(--faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-template-rows: repeat(6, 1fr);
    min-height: 0;
  }
  .cell {
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    padding: 6px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    cursor: pointer;
    transition: background 0.12s;
    outline: none;
    container-type: inline-size;
  }
  /* In narrow cells give the title all the room; the time is in the tooltip. */
  @container (max-width: 140px) {
    .time {
      display: none;
    }
  }
  .cell:nth-child(7n) {
    border-right: none;
  }
  .cell:nth-last-child(-n + 7) {
    border-bottom: none;
  }
  .cell:hover {
    background: color-mix(in srgb, var(--surface-2) 60%, transparent);
  }
  .cell.selected {
    background: color-mix(in srgb, var(--primary) 7%, transparent);
  }
  .grid .cell.in-range {
    background: color-mix(in srgb, var(--primary) 16%, transparent);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--primary) 50%, transparent);
  }
  .rep {
    display: grid;
    flex: none;
    color: var(--faint);
  }
  .cell.other .num {
    color: var(--faint);
  }
  .num {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: transparent;
    font-weight: 650;
    font-size: 13px;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
  }
  .num:hover {
    background: var(--surface-3);
  }
  .num.today {
    background: var(--primary);
    color: var(--primary-text);
  }
  .items {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 2px;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 21px;
    padding: 0 7px;
    border: none;
    border-radius: 7px;
    font-size: 12px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
    min-width: 0;
    width: 100%;
  }
  .allday {
    background: color-mix(in srgb, var(--c) 22%, var(--surface));
    color: var(--text);
    box-shadow: inset 3px 0 0 var(--c);
  }
  .allday:hover {
    background: color-mix(in srgb, var(--c) 32%, var(--surface));
  }
  .timed {
    background: transparent;
    color: var(--text);
  }
  .timed:hover {
    background: var(--surface-2);
  }
  .time {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    flex: none;
  }
  .dday {
    color: var(--c);
    border: 1.5px dashed color-mix(in srgb, var(--c) 60%, transparent);
    background: transparent;
    cursor: default;
  }
  .more {
    border: none;
    background: none;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    padding: 0 7px;
    cursor: pointer;
  }
  .more:hover {
    color: var(--primary);
  }
</style>
