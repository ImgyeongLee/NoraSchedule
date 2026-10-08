<script lang="ts">
  import { CalendarCheck, Leaf, Plus, Target } from '@lucide/svelte';
  import EventModal from '../pages/EventModal.svelte';
  import { api, type CalEvent, type DDay } from '../lib/api';
  import { DEFAULT_COLOR, hex, textOn } from '../lib/colors';
  import { addMinutes, covers, ddayOn, isAllDayLane, pad, timeOf, toDateTime, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, openCalendar } from '../lib/state.svelte';
  import { eventMenu, pasteAsEvent, pasteOnHover } from '../lib/clipboard.svelte';
  import { eventHover } from '../lib/hovercard.svelte';
  import { openMenu } from '../lib/menu.svelte';

  let events = $state<CalEvent[]>([]);
  let ddays = $state<DDay[]>([]);
  let editing = $state<CalEvent | null>(null);

  $effect(() => {
    data.version;
    const d = today();
    load(api.eventsBetween(d, d), []).then((e) => (events = e.filter((x) => covers(x, d))));
    load(api.ddays(), []).then((x) => (ddays = x.filter((y) => ddayOn(y, d))));
  });

  function addEvent() {
    const now = new Date();
    const start = toDateTime(today(), `${pad(Math.min(22, now.getHours() + 1))}:00`);
    editing = { id: 0, title: '', start, end: addMinutes(start, 60), all_day: false, color: DEFAULT_COLOR, location: '', links: [], memo: '', repeat: null, exdates: [], occurrence: null, cancelled: false, tags: [], reminder: null };
  }
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => openCalendar(today())}><span class="w-icon"><CalendarCheck size={15} /></span>{t('w.agenda')}</button>
    <span class="spacer"></span>
    <button class="icon-btn" onclick={addEvent} title={t('event.new')}><Plus size={17} /></button>
  </div>
  <div class="w-body" role="list" {...pasteOnHover(() => pasteAsEvent(today()))}>
    {#each ddays as d (d.id)}
      <div class="dday" style:--c={hex(d.color)}><Target size={14} /> {d.title}</div>
    {/each}
    {#each events as e (`${e.id}-${e.occurrence}`)}
      <button
        class="item event-block"
        class:cancelled={e.cancelled}
        style:--c={hex(e.color)}
        style:--on-c={textOn(e.color)}
        onclick={() => (editing = e)}
        {...eventHover(e)}
        oncontextmenu={(ev) => openMenu(ev, eventMenu(e, () => (editing = e)))}
      >
        <span class="time tabular">{isAllDayLane(e) ? t('common.allDay') : timeOf(e.start)}</span>
        <span class="title truncate">{e.title}</span>
      </button>
    {:else}
      {#if !ddays.length}<div class="w-empty"><span><Leaf size={22} /><br />{t('w.agenda.empty')}</span></div>{/if}
    {/each}
  </div>
</div>

{#if editing}
  <EventModal event={editing} onclose={() => (editing = null)} />
{/if}

<style>
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    margin-bottom: 6px;
    border: none;
    border-radius: 12px;
    text-align: left;
    cursor: pointer;
  }
  .time {
    font-size: 12px;
    font-weight: 650;
    opacity: 0.85;
    min-width: 42px;
  }
  .title {
    font-weight: 600;
  }
  .dday {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    margin-bottom: 6px;
    border-radius: 12px;
    border: 1.5px dashed color-mix(in srgb, var(--c) 60%, transparent);
    color: var(--c);
    font-weight: 650;
    font-size: 13px;
  }
</style>
