<script lang="ts">
  import { CalendarDays, Repeat, Target } from '@lucide/svelte';
  import { eventMenu, pasteAsEvent, pasteOnHover, slotMenu } from '../lib/clipboard.svelte';
  import { eventHover, hideHoverCard, showHoverCard } from '../lib/hovercard.svelte';
  import { isSecondaryClick, openMenu } from '../lib/menu.svelte';
  import type { CalEvent, DDay } from '../lib/api';
  import { hex, textOn } from '../lib/colors';
  import { layoutLanes, type CalTodo } from '../lib/lanes';
  import TodoChip from '../components/TodoChip.svelte';
  import { covers, fmt, isAllDayLane, monthStart, range, timeOf, today, weekStart, weekdayNames } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';

  let {
    cursor,
    events,
    ddays,
    todos = [],
    onopen,
    onnew,
    onday,
    onselect,
    onrange,
  }: {
    cursor: string;
    events: CalEvent[];
    ddays: DDay[];
    /** Todos with a due date, shown on that day. */
    todos?: CalTodo[];
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
  const weeks = $derived(Array.from({ length: 6 }, (_, w) => days.slice(w * 7, w * 7 + 7)));
  const month = $derived(cursor.slice(0, 7));
  const todayStr = $derived(today());
  let gridHeight = $state(600);
  // How many item rows fit in a cell (day number takes ~36px, each row 24px).
  const capacity = $derived(Math.max(1, Math.floor((gridHeight / 6 - 36) / 24)));

  /** Lays out a week: multi-day events become one bar across the days they cover. */
  function layoutWeek(week: string[]) {
    const { placed, lanesPerDay } = layoutLanes(week, ddays, events, todos);
    // If any day overflows, keep the last row for "+N more" buttons.
    const overflow = lanesPerDay.some((n) => n > capacity);
    const limit = overflow ? capacity - 1 : capacity;
    const hidden = Array.from({ length: 7 }, (_, c) => placed.filter((p) => p.lane >= limit && p.col <= c && c < p.col + p.span).length);
    return { bars: placed.filter((p) => p.lane < limit), hidden, moreRow: limit };
  }
</script>

<!-- A right-click during a drag cancels it (capture: item menus stop propagation). -->
<svelte:window onpointerup={dragEnd} oncontextmenucapture={() => (dragFrom = dragTo = null)} />

<div class="month">
  <div class="dow">
    {#each weekdayNames() as d, i (i)}<div>{d}</div>{/each}
  </div>
  <div class="grid" bind:clientHeight={gridHeight}>
    {#each weeks as week (week[0])}
      {@const layout = layoutWeek(week)}
      <div class="week">
        {#each week as day (day)}
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
          </div>
        {/each}

        <div class="bars">
          {#each layout.bars as b, i (i)}
            {#if b.kind === 'dday'}
              <div class="chip dday" style:grid-column="{b.col + 1} / span 1" style:grid-row={b.lane + 1} style:--c={hex(b.d.color)} title={b.d.title}>
                <Target size={12} /> <span class="truncate">{b.d.title}</span>
              </div>
            {:else if b.kind === 'todo'}
              <TodoChip item={b.t} style="grid-column: {b.col + 1} / span 1; grid-row: {b.lane + 1}" />
            {:else}
              {@const e = b.e}
              <button
                class="chip event-block"
                class:cancelled={e.cancelled}
                class:from-prev={b.fromPrev}
                class:to-next={b.toNext}
                style:grid-column="{b.col + 1} / span {b.span}"
                style:grid-row={b.lane + 1}
                style:--c={hex(e.color)}
                style:--on-c={textOn(e.color)}
                onclick={(ev) => { ev.stopPropagation(); onopen(e); }}
                ondblclick={(ev) => ev.stopPropagation()}
                title={isAllDayLane(e) ? e.title : `${timeOf(e.start)} ${e.title}`}
                {...eventHover(e)}
                oncontextmenu={(ev) => openMenu(ev, eventMenu(e, () => onopen(e)))}
              >
                {#if e.repeat}<Repeat size={11} />{/if}
                {#if !isAllDayLane(e)}<span class="time">{timeOf(e.start)}</span>{/if}
                <span class="truncate">{e.title}</span>
              </button>
            {/if}
          {/each}
          {#each layout.hidden as n, c (c)}
            {#if n > 0}
              <button
                class="more"
                style:grid-column="{c + 1} / span 1"
                style:grid-row={layout.moreRow + 1}
                onclick={(e) => { e.stopPropagation(); hideHoverCard(); onday(week[c]); }}
                onmouseenter={(e) => showHoverCard(e.currentTarget, events.filter((ev) => covers(ev, week[c])), fmt(week[c], { month: 'long', day: 'numeric', weekday: 'short' }))}
                onmouseleave={hideHoverCard}
              >
                {t('cal.more', { n })}
              </button>
            {/if}
          {/each}
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
    grid-template-rows: repeat(6, 1fr);
    min-height: 0;
  }
  .week {
    position: relative;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    min-height: 0;
    overflow: hidden;
  }
  .week:not(:last-child) {
    border-bottom: 1px solid var(--border);
  }
  .cell {
    border-right: 1px solid var(--border);
    padding: 6px;
    min-width: 0;
    min-height: 0;
    cursor: pointer;
    transition: background 0.12s;
    outline: none;
  }
  .cell:nth-child(7) {
    border-right: none;
  }
  .cell:hover {
    background: color-mix(in srgb, var(--surface-2) 60%, transparent);
  }
  .cell.selected {
    background: color-mix(in srgb, var(--primary) 7%, transparent);
  }
  .week .cell.in-range {
    background: color-mix(in srgb, var(--primary) 16%, transparent);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--primary) 50%, transparent);
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
  /* Items sit on a 7-column grid over the day cells, so a bar can run across days. */
  .bars {
    position: absolute;
    inset: 36px 0 0 0;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    grid-auto-rows: 21px;
    row-gap: 3px;
    align-content: start;
    pointer-events: none;
  }
  .bars > * {
    pointer-events: auto;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 21px;
    margin: 0 5px;
    padding: 0 7px;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .chip.from-prev {
    margin-left: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }
  .chip.to-next {
    margin-right: 0;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .time {
    flex: none;
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
  }
  /* In narrow cells give the title all the room; the time is in the tooltip. */
  @container main (max-width: 900px) {
    .time {
      display: none;
    }
  }
  .dday {
    color: var(--c);
    border: 1.5px dashed color-mix(in srgb, var(--c) 60%, transparent);
    background: var(--surface);
    cursor: default;
    pointer-events: none;
  }
  .more {
    margin: 0 5px;
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
