<script lang="ts">
  import { onMount } from 'svelte';
  import { MapPin, Repeat, Target } from '@lucide/svelte';
  import { copyEvent, copyOnHover, eventMenu, pasteAsEvent, pointer, slotMenu } from '../lib/clipboard.svelte';
  import { isSecondaryClick, openMenu } from '../lib/menu.svelte';
  import type { CalEvent, DDay } from '../lib/api';
  import { hex } from '../lib/colors';
  import { covers, fmt, isAllDayLane, minutesOf, overlapColumns, pad, timeOf, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';

  let {
    days,
    events,
    ddays,
    onopen,
    onnew,
    onday,
    onrange,
  }: {
    days: string[];
    events: CalEvent[];
    ddays: DDay[];
    onopen: (e: CalEvent) => void;
    onnew: (day: string, minutes: number) => void;
    onday: (day: string) => void;
    /** Called after dragging over time slots (possibly across several days). */
    onrange: (fromDay: string, toDay: string, startMinutes: number, endMinutes: number) => void;
  } = $props();

  // ---- drag over slots to create an event
  let drag = $state<{ day0: string; min0: number; day1: string; min1: number } | null>(null);
  const dragBox = $derived.by(() => {
    if (!drag) return null;
    const [d0, d1] = drag.day0 <= drag.day1 ? [drag.day0, drag.day1] : [drag.day1, drag.day0];
    return { d0, d1, start: Math.min(drag.min0, drag.min1), end: Math.max(drag.min0, drag.min1) + 30 };
  });

  function dragStart(e: PointerEvent, day: string) {
    if (isSecondaryClick(e) || (e.target as HTMLElement).closest('button')) return;
    const m = slotAt(e);
    drag = { day0: day, min0: m, day1: day, min1: m };
  }

  function dragMove(e: PointerEvent, day: string) {
    if (drag) drag = { ...drag, day1: day, min1: slotAt(e) };
  }

  function dragEnd() {
    if (!drag || !dragBox) return;
    const moved = drag.day0 !== drag.day1 || drag.min0 !== drag.min1;
    if (moved) onrange(dragBox.d0, dragBox.d1, dragBox.start, Math.min(dragBox.end, 24 * 60 - 1));
    else onnew(drag.day0, Math.min(drag.min0, 23 * 60));
    drag = null;
  }

  const HOUR = 56;
  let scroller: HTMLDivElement;
  let now = $state(new Date());
  const todayStr = $derived(today());
  const nowMinutes = $derived(now.getHours() * 60 + now.getMinutes());

  onMount(() => {
    scroller.scrollTop = HOUR * 7.5;
    const t = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(t);
  });

  const lane = $derived(
    days.map((day) => ({
      ddays: ddays.filter((d) => d.date === day),
      events: events.filter((e) => isAllDayLane(e) && covers(e, day)),
    })),
  );
  const laneRows = $derived(Math.min(4, Math.max(1, ...lane.map((l) => l.ddays.length + l.events.length))));

  function timedFor(day: string) {
    const items = events
      .filter((e) => !isAllDayLane(e) && covers(e, day))
      .map((e) => {
        const s = e.start.slice(0, 10) < day ? 0 : minutesOf(e.start);
        const end = e.end.slice(0, 10) > day ? 24 * 60 : minutesOf(e.end);
        return { e, s, end: Math.max(end, s + 20) };
      })
      .sort((a, b) => a.s - b.s || b.end - a.end);
    const layout = overlapColumns(items.map((i) => [i.s, i.end]));
    return items.map((item, i) => ({ ...item, ...layout[i] }));
  }

  /** Half-hour slot under the mouse, in minutes past midnight. */
  function slotAt(e: MouseEvent): number {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    return Math.min(Math.floor(((e.clientY - rect.top) / HOUR) * 2) * 30, 23 * 60 + 30);
  }


</script>

<!-- A right-click during a drag cancels it (capture: item menus stop propagation). -->
<svelte:window onpointerup={dragEnd} oncontextmenucapture={() => (drag = null)} />

<div class="tg" style:--cols={days.length}>
  <div class="head">
    <div></div>
    {#each days as day (day)}
      <button class="dayhead" class:today={day === todayStr} onclick={() => onday(day)} disabled={days.length === 1}>
        <span class="dow">{fmt(day, { weekday: 'short' })}</span>
        <span class="num">{Number(day.slice(8))}</span>
      </button>
    {/each}
  </div>

  <div class="lane" style:min-height="{laneRows * 24 + 10}px">
    <div class="gutter-label">{t('cal.allDayLane')}</div>
    {#each lane as l, i (days[i])}
      <div class="lane-col">
        {#each l.ddays as d (d.id)}
          <div class="chip dday" style:--c={hex(d.color)}><Target size={12} /> <span class="truncate">{d.title}</span></div>
        {/each}
        {#each l.events as e (`${e.id}-${e.occurrence}`)}
          <button
            class="chip allday"
            style:--c={hex(e.color)}
            onclick={() => onopen(e)}
            {...copyOnHover(() => copyEvent(e))}
            oncontextmenu={(ev) => openMenu(ev, eventMenu(e, () => onopen(e)))}
          ><span class="truncate">{e.title}</span></button>
        {/each}
      </div>
    {/each}
  </div>

  <div class="scroll" bind:this={scroller}>
    <div class="body" style:height="{HOUR * 24}px">
      <div class="gutter">
        {#each Array.from({ length: 23 }, (_, i) => i + 1) as h (h)}
          <span style:top="{h * HOUR}px">{pad(h)}:00</span>
        {/each}
      </div>
      {#each days as day (day)}
        <div
          class="col"
          class:is-today={day === todayStr}
          style:--hour="{HOUR}px"
          onpointerdown={(e) => dragStart(e, day)}
          onpointermove={(e) => {
            dragMove(e, day);
            const m = slotAt(e);
            pointer.paste = () => pasteAsEvent(day, m);
          }}
          onmouseleave={() => (pointer.paste = null)}
          oncontextmenu={(e) => {
            const m = slotAt(e);
            openMenu(e, slotMenu(day, m, () => onnew(day, Math.min(m, 23 * 60))));
          }}
          role="presentation"
        >
          {#each timedFor(day) as t (`${t.e.id}-${t.e.occurrence}`)}
            <button
              class="event"
              style:--c={hex(t.e.color)}
              style:top="{(t.s / 60) * HOUR + 1}px"
              style:height="{((t.end - t.s) / 60) * HOUR - 3}px"
              style:left="calc({(t.col / t.cols) * 100}% + 3px)"
              style:width="calc({100 / t.cols}% - 6px)"
              onclick={(ev) => { ev.stopPropagation(); onopen(t.e); }}
              onpointermove={(ev) => ev.stopPropagation()}
              {...copyOnHover(() => copyEvent(t.e))}
              oncontextmenu={(ev) => openMenu(ev, eventMenu(t.e, () => onopen(t.e)))}
            >
              <span class="ev-title">{#if t.e.repeat}<Repeat size={11} />{/if} {t.e.title}</span>
              <span class="ev-time">{timeOf(t.e.start)} – {timeOf(t.e.end)}</span>
              {#if t.e.location}<span class="ev-time truncate loc"><MapPin size={11} /> {t.e.location}</span>{/if}
            </button>
          {/each}
          {#if dragBox && day >= dragBox.d0 && day <= dragBox.d1}
            <div
              class="drag-preview"
              style:top="{(dragBox.start / 60) * HOUR}px"
              style:height="{((dragBox.end - dragBox.start) / 60) * HOUR}px"
            ></div>
          {/if}
          {#if day === todayStr}
            <div class="now" style:top="{(nowMinutes / 60) * HOUR}px"></div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .tg {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  .head,
  .lane,
  .body {
    display: grid;
    grid-template-columns: 60px repeat(var(--cols), 1fr);
  }
  .head {
    padding: 10px 10px 6px 0;
  }
  .lane {
    padding-right: 10px;
    border-bottom: 1px solid var(--border);
  }
  .dayhead {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    border: none;
    background: none;
    cursor: pointer;
    padding: 4px;
    border-radius: 12px;
  }
  .dayhead:disabled {
    cursor: default;
  }
  .dayhead:not(:disabled):hover {
    background: var(--surface-2);
  }
  .dayhead .dow {
    font-size: 12px;
    font-weight: 650;
    color: var(--faint);
    text-transform: uppercase;
  }
  .dayhead .num {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 18px;
    font-weight: 650;
  }
  .dayhead.today .num {
    background: var(--primary);
    color: var(--primary-text);
  }
  .dayhead.today .dow {
    color: var(--primary);
  }
  .gutter-label {
    font-size: 11px;
    color: var(--faint);
    padding: 8px 8px 0 0;
    text-align: right;
  }
  .lane-col {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px 3px;
    min-width: 0;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 21px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
    min-width: 0;
    text-align: left;
  }
  .allday {
    background: color-mix(in srgb, var(--c) 22%, var(--surface));
    box-shadow: inset 3px 0 0 var(--c);
    color: var(--text);
  }
  .dday {
    color: var(--c);
    border: 1.5px dashed color-mix(in srgb, var(--c) 60%, transparent);
    background: transparent;
    cursor: default;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
  .body {
    position: relative;
    padding-right: 10px;
  }
  .gutter {
    position: relative;
  }
  .gutter span {
    position: absolute;
    right: 10px;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .col {
    position: relative;
    border-left: 1px solid var(--border);
    background-image: repeating-linear-gradient(to bottom, var(--border) 0 1px, transparent 1px var(--hour));
    cursor: copy;
  }
  .col.is-today {
    background-color: color-mix(in srgb, var(--primary) 4%, transparent);
  }
  .event {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 5px 8px;
    border: none;
    border-radius: 10px;
    background: color-mix(in srgb, var(--c) 24%, var(--surface));
    box-shadow: inset 3px 0 0 var(--c);
    color: var(--text);
    text-align: left;
    overflow: hidden;
    cursor: pointer;
    transition: transform 0.1s, box-shadow 0.15s;
    min-height: 20px;
  }
  .event:hover {
    background: color-mix(in srgb, var(--c) 34%, var(--surface));
    z-index: 2;
  }
  .ev-title {
    font-size: 12.5px;
    font-weight: 650;
    line-height: 1.25;
  }
  .ev-time {
    font-size: 11.5px;
    color: var(--muted);
    max-width: 100%;
  }
  .drag-preview {
    position: absolute;
    left: 3px;
    right: 3px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--primary) 22%, transparent);
    border: 1.5px dashed var(--primary);
    pointer-events: none;
    z-index: 4;
  }
  .loc {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .now {
    position: absolute;
    left: -5px;
    right: 0;
    height: 2px;
    background: var(--danger);
    pointer-events: none;
    z-index: 3;
  }
  .now::before {
    content: '';
    position: absolute;
    left: 0;
    top: -4px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--danger);
  }
</style>
