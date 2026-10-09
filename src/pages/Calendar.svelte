<script lang="ts">
  import { CalendarPlus, ChevronLeft, ChevronRight, Leaf, MapPin, Repeat, Settings2, Tags, Target } from '@lucide/svelte';
  import MiniCalendar from '../components/MiniCalendar.svelte';
  import MonthView from './MonthView.svelte';
  import TimeGrid from './TimeGrid.svelte';
  import EventModal from './EventModal.svelte';
  import { api, type CalEvent, type DDay } from '../lib/api';
  import { DEFAULT_COLOR, hex, textOn } from '../lib/colors';
  import {
    addDays, addMonths, byDdayTarget, covers, ddayLabel, ddayOn, ddayUpcoming, diffDays, eventSpan, fmt, fmtRange, isAllDayLane, monthStart, pad, range, timeOf,
    today, toDateTime, weekStart,
  } from '../lib/dates';
  import { data, load, ui } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';
  import { eventMenu, pageTarget, pasteAsEvent } from '../lib/clipboard.svelte';
  import { eventHover } from '../lib/hovercard.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { eventColor, groupedTags, matchesTagFilter, setTagFilter, tagStore, tagsOf, toggleTagFilter } from '../lib/tags.svelte';
  import TagManager from '../components/TagManager.svelte';
  import TodoChip from '../components/TodoChip.svelte';
  import { loadCalendarTodos } from '../lib/calendarTodos';
  import type { CalTodo } from '../lib/lanes';

  type View = 'month' | 'week' | 'day';
  let view = $state<View>('month');
  let cursor = $state(ui.calendarFocus ?? today());
  ui.calendarFocus = null;
  let events = $state<CalEvent[]>([]);
  let ddays = $state<DDay[]>([]);
  let editing = $state<CalEvent | null>(null);
  let todos = $state<CalTodo[]>([]);

  const visible = $derived.by((): [string, string] => {
    if (view === 'month') {
      const start = weekStart(monthStart(cursor));
      return [start, addDays(start, 41)];
    }
    if (view === 'week') {
      const start = weekStart(cursor);
      return [start, addDays(start, 6)];
    }
    return [cursor, cursor];
  });

  $effect(() => {
    data.version;
    const [from, to] = visible;
    load(api.eventsBetween(from, to), []).then((e) => (events = e));
    load(api.ddays(), []).then((d) => (ddays = d));
    loadCalendarTodos().then((x) => (todos = x));
  });

  const title = $derived.by(() => {
    if (view === 'month') return fmt(cursor, { month: 'long', year: 'numeric' });
    if (view === 'day') return fmt(cursor, { weekday: 'long', month: 'long', day: 'numeric' });
    const [a, b] = visible;
    return fmtRange(a, b, { month: a.slice(0, 7) === b.slice(0, 7) ? 'long' : 'short', day: 'numeric' });
  });

  /** Events passing the tag filter (all of them when no tag is selected). */
  const shownEvents = $derived(events.filter(matchesTagFilter));
  const marks = $derived(
    new Set([
      ...shownEvents.flatMap((e) => { const [a, b] = eventSpan(e); return range(a, diffDays(b, a) + 1); }),
      ...todos.flatMap((x) => (x.todo.done || !x.todo.due ? [] : [x.todo.due])),
    ]),
  );
  const agendaTodos = $derived(todos.filter((x) => x.todo.due === cursor));
  const agenda = $derived(shownEvents.filter((e) => covers(e, cursor)));
  let managingTags = $state(false);
  const agendaDdays = $derived(ddays.filter((d) => ddayOn(d, cursor)));
  const upcoming = $derived(byDdayTarget(ddays.filter((d) => ddayUpcoming(d))).slice(0, 4));

  function step(dir: number) {
    cursor = view === 'month' ? addMonths(cursor, dir) : addDays(cursor, (view === 'week' ? 7 : 1) * dir);
  }

  const hm = (minutes: number) => `${pad(Math.floor(minutes / 60))}:${pad(minutes % 60)}`;

  function newEvent(day: string, minutes = 9 * 60, endMinutes = Math.min(minutes + 60, 24 * 60 - 1)) {
    editing = {
      id: 0,
      title: '',
      start: toDateTime(day, hm(minutes)),
      end: toDateTime(day, hm(endMinutes)),
      all_day: false,
      color: DEFAULT_COLOR,
      location: '',
      links: [],
      memo: '',
      repeat: null,
      exdates: [],
      occurrence: null,
      cancelled: false, tags: [], reminder: null,
    };
  }

  // ⌘V with the mouse elsewhere pastes onto the selected day.
  $effect(() => {
    const day = cursor;
    pageTarget.paste = () => pasteAsEvent(day);
    return () => (pageTarget.paste = null);
  });

  /** A drag over several days creates a daily series, one event per day. */
  function newRange(from: string, to: string, startMinutes = 9 * 60, endMinutes = 10 * 60) {
    newEvent(from, startMinutes, endMinutes);
    if (editing && to > from) editing.repeat = { freq: 'daily', interval: 1, weekdays: [], until: to, count: null };
  }

  function openDay(day: string) {
    cursor = day;
    view = 'day';
  }
</script>

<div class="calendar">
  <header>
    <h1 class="title">{title}</h1>
    <div class="nav">
      <button class="icon-btn" onclick={() => step(-1)} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
      <button class="btn small" onclick={() => (cursor = today())}>{t('common.today')}</button>
      <button class="icon-btn" onclick={() => step(1)} aria-label={t('common.next')}><ChevronRight size={18} /></button>
    </div>
    <span class="spacer"></span>
    <button class="btn primary" onclick={() => newEvent(cursor)}><CalendarPlus size={17} /> {t('cal.new')}</button>
  </header>

  <!-- Sits above the side calendar, so the big calendar and the side column start at the same height. -->
  <div class="segmented views">
    {#each [['month', t('cal.month')], ['week', t('cal.week')], ['day', t('cal.day')]] as [v, label] (v)}
      <button class:active={view === v} onclick={() => (view = v as View)}>{label}</button>
    {/each}
  </div>

  <div class="main">
    <div class="view">
      {#if view === 'month'}
        <MonthView {cursor} events={shownEvents} {ddays} {todos} onopen={(e) => (editing = e)} onnew={(d) => newEvent(d)} onday={openDay} onselect={(d) => (cursor = d)} onrange={(a, b) => newRange(a, b)} />
      {:else}
        <TimeGrid
          days={view === 'week' ? range(weekStart(cursor), 7) : [cursor]}
          events={shownEvents}
          {ddays}
          {todos}
          onopen={(e) => (editing = e)}
          onnew={(d, m) => newEvent(d, m)}
          onday={openDay}
          onrange={newRange}
        />
      {/if}
    </div>
    <p class="hint faint small">{t('cal.dragTip')}</p>
  </div>

  <aside class="side">
    <div class="card">
      <MiniCalendar selected={cursor} onpick={(d) => (cursor = d)} {marks} events={shownEvents} />
    </div>

    <div class="card agenda">
      <h3>{cursor === today() ? t('common.today') : fmt(cursor, { weekday: 'long', month: 'short', day: 'numeric' })}</h3>
      {#each agendaDdays as d (d.id)}
        <div class="agenda-dday" style:--c={hex(d.color)}><Target size={15} /> {d.title}</div>
      {/each}
      {#each agenda as e (`${e.id}-${e.occurrence}`)}
        <button
          class="agenda-item event-block"
          class:cancelled={e.cancelled}
          style:--c={hex(eventColor(e))}
          style:--on-c={textOn(eventColor(e))}
          onclick={() => (editing = e)}
          {...eventHover(e)}
          oncontextmenu={(ev) => openMenu(ev, eventMenu(e, () => (editing = e)))}
        >
          <span class="info">
            <span class="ag-title truncate">{#if e.repeat}<Repeat size={12} />{/if} {e.title}</span>
            <span class="sub small">{isAllDayLane(e) ? t('common.allDay') : `${timeOf(e.start)} – ${timeOf(e.end)}`}</span>
            {#if e.location}<span class="sub small row truncate"><MapPin size={12} /> {e.location}</span>{/if}
            {#if tagsOf(e).length}<span class="sub small truncate">{tagsOf(e).map((tag) => '#' + tag.name).join(' ')}</span>{/if}
          </span>
        </button>
      {:else}
        {#if agendaDdays.length === 0 && agendaTodos.length === 0}
          <div class="free">
            <span class="leaf"><Leaf size={22} /></span>
            <span class="muted">{t('cal.nothingPlanned')}</span>
            <button class="btn small" onclick={() => newEvent(cursor)}>{t('cal.addSomething')}</button>
          </div>
        {/if}
      {/each}
      {#if agendaTodos.length}
        <div class="faint small todo-head">{t('nav.todos')}</div>
        {#each agendaTodos as item (item.todo.id)}<TodoChip {item} />{/each}
      {/if}
    </div>

    <div class="card">
      <div class="card-head">
        <h3><Tags size={16} /> {t('tag.title')}</h3>
        <span class="spacer"></span>
        {#if tagStore.filter.length}<button class="btn small ghost" onclick={() => setTagFilter([])}>{t('tag.showAll')}</button>{/if}
        <button class="icon-btn" onclick={() => (managingTags = true)} title={t('tag.manage')} aria-label={t('tag.manage')}><Settings2 size={16} /></button>
      </div>
      {#each groupedTags() as group (group.category)}
        <div class="tag-group">
          <div class="faint small">{group.category || t('tag.noCategory')}</div>
          <div class="tag-list">
            {#each group.tags as tag (tag.id)}
              <button
                class="tag-chip"
                class:on={tagStore.filter.includes(tag.id)}
                style:--tc={hex(tag.color)}
                onclick={() => toggleTagFilter(tag.id)}
                title={t('tag.filterHint')}
              ><span class="tag-dot"></span>{tag.name}</button>
            {/each}
          </div>
        </div>
      {:else}
        <p class="muted small tag-empty">{t('tag.empty')}</p>
        <button class="btn small" onclick={() => (managingTags = true)}>{t('tag.create')}</button>
      {/each}
    </div>

    {#if upcoming.length}
      <div class="card">
        <h3>{t('cal.upcomingDdays')}</h3>
        <div class="ddays">
          {#each upcoming as d (d.id)}
            <div class="dd">
              <span class="dd-label" style:color={hex(d.color)} style:background="color-mix(in srgb, {hex(d.color)} 14%, transparent)">{ddayLabel(d)}</span>
              <span class="truncate">{d.title}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </aside>
</div>

{#if managingTags}
  <TagManager onclose={() => (managingTags = false)} />
{/if}

{#if editing}
  <EventModal event={editing} onclose={() => (editing = null)} />
{/if}

<style>
  .calendar {
    /* One spacing for both the calendar ↔ side column and between the side cards. */
    --gap: 16px;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 270px;
    /* Top row: title bar | view switcher. Bottom row: calendar | side column (same top edge). */
    grid-template-rows: auto minmax(0, 1fr);
    grid-template-areas:
      'head views'
      'main side';
    gap: var(--gap);
    height: 100%;
    padding: 28px 24px 20px 28px;
  }
  .main {
    grid-area: main;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    grid-area: head;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .views {
    grid-area: views;
    align-self: center;
  }
  .views button {
    flex: 1;
    justify-content: center;
  }
  .title {
    font-size: 22px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .view {
    flex: 1;
    min-height: 0;
  }
  .hint {
    margin-top: 8px;
    text-align: center;
  }
  .side {
    grid-area: side;
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    overflow-y: auto;
    min-height: 0;
  }
  .side .card {
    padding: 16px;
  }
  .agenda {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .agenda h3 {
    margin-bottom: 2px;
  }
  .agenda-item {
    display: flex;
    gap: 10px;
    padding: 10px;
    border: none;
    border-radius: var(--radius-sm);
    text-align: left;
    cursor: pointer;
  }
  .sub {
    opacity: 0.85;
  }
  .info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .ag-title {
    font-weight: 650;
  }
  .agenda-dday {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    border: 1.5px dashed color-mix(in srgb, var(--c) 60%, transparent);
    color: var(--c);
    font-weight: 650;
  }
  .agenda :global(.todo-chip) {
    height: 30px;
    margin: 0;
    font-size: 13px;
  }
  .todo-head {
    margin-top: 4px;
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }
  .card-head h3 {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tag-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
  }
  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border: 1.5px solid color-mix(in srgb, var(--tc) 40%, transparent);
    border-radius: 999px;
    background: transparent;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .tag-chip:hover {
    background: color-mix(in srgb, var(--tc) 10%, transparent);
  }
  .tag-chip.on {
    background: var(--tc);
    border-color: var(--tc);
    color: #fff;
  }
  .tag-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--tc);
  }
  .tag-chip.on .tag-dot {
    background: #fff;
  }
  .tag-empty {
    margin-bottom: 8px;
  }
  .free {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 14px 0 6px;
    font-size: 13px;
  }
  .leaf {
    color: var(--success);
  }
  .ddays {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
  }
  .dd {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .dd-label {
    flex: none;
    min-width: 58px;
    text-align: center;
    padding: 3px 8px;
    border-radius: 999px;
    font-weight: 750;
    font-size: 12.5px;
  }
  @container main (max-width: 940px) {
    /* No side column: the view switcher moves up next to the title bar. */
    .calendar {
      grid-template-columns: minmax(0, 1fr) auto;
      grid-template-areas:
        'head views'
        'main main';
    }
    .side {
      display: none;
    }
  }
  @container main (max-width: 680px) {
    .calendar {
      padding: 20px 14px 14px;
    }
    header {
      flex-wrap: wrap;
    }
    .hint {
      display: none;
    }
  }
</style>
