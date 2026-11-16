<script lang="ts">
  // Routines: todos that are added to the list by themselves: every day, on chosen weekdays
  // (every week or every other week), or on chosen days of every month.
  import { Plus, Repeat } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import ConfirmButton from './ConfirmButton.svelte';
  import DateField from './DateField.svelte';
  import Select from './Select.svelte';
  import { api, type Routine, type RoutineFreq, type TodoGroup } from '../lib/api';
  import { hex } from '../lib/colors';
  import { fmt, parseYmd, today, weekdayNames } from '../lib/dates';
  import { FREQS, nextDates } from '../lib/routines';
  import { t } from '../lib/i18n.svelte';
  import { data, load, mutate } from '../lib/state.svelte';

  let { groups }: { groups: TodoGroup[] } = $props();

  let routines = $state<Routine[]>([]);
  $effect(() => {
    data.version;
    load(api.routines(), []).then((r) => (routines = r));
  });

  const groupMap = $derived(new Map(groups.map((g) => [g.id, g])));
  const groupOptions = $derived([
    { value: null as number | null, label: t('todo.ungrouped') },
    ...groups.map((g) => ({ value: g.id as number | null, label: g.name, color: hex(g.color) })),
  ]);
  const dayNames = $derived(weekdayNames('short'));
  const WEEKDAYS = [1, 2, 3, 4, 5];
  const WEEKEND = [0, 6];
  const same = (a: number[], b: number[]) => a.length === b.length && a.every((x) => b.includes(x));

  /** "Every day", "Weekdays", "Weekends" or "Mon · Wed · Fri". */
  function daysLabel(days: number[]) {
    if (!days.length || days.length === 7) return t('routine.everyDay');
    if (same(days, WEEKDAYS)) return t('routine.weekdays');
    if (same(days, WEEKEND)) return t('routine.weekend');
    return [...days].sort().map((d) => dayNames[d]).join(' · ');
  }

  type Schedule = Pick<Routine, 'freq' | 'weekdays' | 'month_days' | 'start'>;
  const MONTH_DAYS = Array.from({ length: 31 }, (_, i) => i + 1);
  const dayOfMonth = (d: number) => (d === 31 ? t('routine.lastDay') : t('routine.dayN', { n: d }));

  /** "Every day", "Every 2 weeks · Mon · Thu", "Monthly: 1st, last day". */
  function scheduleLabel(r: Schedule) {
    if (r.freq === 'monthly') return t('routine.monthlyLabel', { days: [...r.month_days].sort((a, b) => a - b).map(dayOfMonth).join(', ') });
    if (r.freq === 'biweekly') return t('routine.biweeklyLabel', { days: daysLabel(r.weekdays) });
    return daysLabel(r.weekdays);
  }

  /** Switching to "every other week" or "monthly" fills in what that needs. */
  function withFreq(r: Schedule, freq: RoutineFreq): Partial<Schedule> {
    if (freq === 'monthly' && !r.month_days.length) return { freq, month_days: [parseYmd(today()).getDate()] };
    // Every other week usually means one weekday (laundry on Wednesdays), so start with today's.
    if (freq === 'biweekly')
      return { freq, start: r.start ?? today(), weekdays: r.weekdays.length ? r.weekdays : [parseYmd(today()).getDay()] };
    return { freq };
  }
  // A monthly routine keeps at least one day.
  const toggleMonthDay = (list: number[], d: number) =>
    list.includes(d) ? (list.length > 1 ? list.filter((x) => x !== d) : list) : [...list, d].sort((a, b) => a - b);

  function nextLabel(r: Schedule) {
    const dates = nextDates({ id: 0, title: '', group_id: null, active: true, created_at: 0, ...r }, today(), 3);
    return dates.map((d) => fmt(d, { month: 'short', day: 'numeric', weekday: 'short' })).join(', ');
  }

  // ---- new routine
  let title = $state('');
  let sched = $state<Schedule>({ freq: 'weekly', weekdays: [], month_days: [], start: null });
  let group = $state<number | null>(null);

  const toggleDay = (list: number[], d: number) => (list.includes(d) ? list.filter((x) => x !== d) : [...list, d]);

  async function add() {
    const name = title.trim();
    if (!name) return;
    const ok = await mutate(
      api.saveRoutine({ id: 0, title: name, group_id: group, active: true, created_at: 0, ...sched }),
      t('routine.added'),
    );
    if (ok !== undefined) {
      title = '';
      sched = { freq: 'weekly', weekdays: [], month_days: [], start: null };
    }
  }

  const setActive = (r: Routine, active: boolean) => mutate(api.saveRoutine({ ...r, active }));

  // ---- edit
  let editing = $state<Routine | null>(null);
  async function save() {
    if (!editing || !editing.title.trim()) return;
    const ok = await mutate(api.saveRoutine({ ...editing, title: editing.title.trim() }), t('routine.saved'));
    if (ok !== undefined) editing = null;
  }
  async function remove() {
    if (!editing) return;
    const id = editing.id;
    editing = null;
    await mutate(api.deleteRoutine(id), t('routine.deleted'));
  }
</script>

{#snippet schedulePicker(r: Schedule, set: (patch: Partial<Schedule>) => void)}
  <div class="schedule">
    <div class="segmented freq" role="group" aria-label={t('routine.freq')}>
      {#each FREQS as f (f)}
        <button type="button" class:active={r.freq === f} onclick={() => set(withFreq(r, f))}>{t(`routine.freq.${f}`)}</button>
      {/each}
    </div>
    {#if r.freq === 'monthly'}
      <div class="month-days" role="group" aria-label={t('routine.monthDays')}>
        {#each MONTH_DAYS as d (d)}
          <button type="button" class="mday" class:on={r.month_days.includes(d)} aria-pressed={r.month_days.includes(d)}
            onclick={() => set({ month_days: toggleMonthDay(r.month_days, d) })}>{d}</button>
        {/each}
      </div>
      <p class="faint small hint">{t('routine.monthHint')}</p>
    {:else}
      {@render dayPicker(r.weekdays, (v) => set({ weekdays: v }))}
      {#if r.freq === 'biweekly'}
        <div class="start-row">
          <span class="muted small strong">{t('routine.startWeek')}</span>
          <div class="start-field"><DateField value={r.start ?? today()} label={t('routine.startWeek')} onchange={(v) => v && set({ start: v })} /></div>
          <span class="faint small">{t('routine.startHint')}</span>
        </div>
      {/if}
    {/if}
    <p class="faint small next"><Repeat size={12} /> {t('routine.next', { dates: nextLabel(r) })}</p>
  </div>
{/snippet}

{#snippet dayPicker(value: number[], set: (v: number[]) => void)}
  <div class="days" role="group" aria-label={t('routine.days')}>
    <button type="button" class="preset" class:on={!value.length || value.length === 7} onclick={() => set([])}>{t('routine.everyDay')}</button>
    <button type="button" class="preset" class:on={same(value, WEEKDAYS)} onclick={() => set([...WEEKDAYS])}>{t('routine.weekdays')}</button>
    <button type="button" class="preset" class:on={same(value, WEEKEND)} onclick={() => set([...WEEKEND])}>{t('routine.weekend')}</button>
    <span class="sep"></span>
    {#each dayNames as name, d (d)}
      <button type="button" class="day" class:on={value.includes(d)} class:sun={d === 0} aria-pressed={value.includes(d)} onclick={() => set(toggleDay(value, d))}>{name}</button>
    {/each}
  </div>
{/snippet}

<div class="routines">
  <div class="head">
    <h2 class="heading">{t('routine.title')}</h2>
    <p class="muted small">{t('routine.subtitle')}</p>
  </div>

  <div class="card add">
    <div class="add-row">
      <span class="plus"><Plus size={18} /></span>
      <input class="input" placeholder={t('routine.placeholder')} bind:value={title} onkeydown={(e) => e.key === 'Enter' && add()} />
      <div class="group-pick"><Select bind:value={group} options={groupOptions} label={t('todo.group')} /></div>
      <button class="btn primary" onclick={add} disabled={!title.trim()}>{t('common.add')}</button>
    </div>
    {@render schedulePicker(sched, (patch) => (sched = { ...sched, ...patch }))}
  </div>

  <div class="card list">
    {#each routines as r (r.id)}
      {@const g = r.group_id !== null ? groupMap.get(r.group_id) : undefined}
      <div class="routine" class:paused={!r.active}>
        <input type="checkbox" class="switch" checked={r.active} onchange={(e) => setActive(r, e.currentTarget.checked)}
          aria-label={t('routine.active')} title={r.active ? t('routine.pause') : t('routine.resume')} />
        <button class="r-main" onclick={() => (editing = { ...r, weekdays: [...r.weekdays], month_days: [...r.month_days] })}>
          <span class="r-title truncate">{r.title}</span>
          <span class="r-meta muted small">
            <Repeat size={12} /> {scheduleLabel(r)}
            {#if g}<span class="dot" style:background={hex(g.color)}></span> {g.name}{/if}
            {#if !r.active}· {t('routine.paused')}{/if}
          </span>
        </button>
      </div>
    {:else}
      <div class="empty">
        <span class="empty-icon"><Repeat size={28} /></span>
        <p>{t('routine.empty')}</p>
      </div>
    {/each}
  </div>
</div>

{#if editing}
  <Modal title={t('routine.edit')} onclose={() => (editing = null)} width={520}>
    <div class="field">
      <label for="routine-title">{t('common.name')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="routine-title" class="input" bind:value={editing.title} autofocus onkeydown={(e) => e.key === 'Enter' && save()} />
    </div>
    <div class="field">
      <span class="label">{t('routine.days')}</span>
      {@render schedulePicker(editing, (patch) => editing && Object.assign(editing, patch))}
    </div>
    <div class="field">
      <label for="routine-group">{t('todo.group')}</label>
      <Select id="routine-group" bind:value={editing.group_id} options={groupOptions} />
    </div>
    <label class="row active-row">
      <input type="checkbox" class="switch" bind:checked={editing.active} /> {t('routine.active')}
    </label>
    {#snippet footer()}
      <ConfirmButton onconfirm={remove} question={t('routine.deleteQ')} />
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editing = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={save}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .routines {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .heading {
    font-size: 22px;
  }
  .add {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
  }
  .add-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .plus {
    position: absolute;
    left: 14px;
    display: grid;
    color: var(--primary);
  }
  .add-row .input {
    flex: 1;
    padding-left: 40px;
  }
  .group-pick {
    width: 160px;
  }
  .add-row .btn {
    height: 40px;
  }
  .days {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .days button {
    height: 30px;
    padding: 0 12px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 650;
    cursor: pointer;
  }
  .days .day {
    width: 34px;
    padding: 0;
  }
  .days .sun {
    color: var(--danger);
  }
  .days button.on {
    border-color: var(--primary);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .schedule {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .freq {
    align-self: flex-start;
  }
  .month-days {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(34px, 1fr));
    gap: 5px;
    max-width: 360px;
  }
  .mday {
    height: 30px;
    padding: 0;
    border: 1.5px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 650;
    cursor: pointer;
  }
  .mday.on {
    border-color: var(--primary);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .hint,
  .next {
    margin: 0;
  }
  .next {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .start-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 10px;
  }
  .start-field {
    width: 170px;
  }
  .strong {
    font-weight: 650;
  }
  .sep {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--border);
  }
  .list {
    padding: 6px;
  }
  .routine {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 12px;
  }
  .routine:hover {
    background: var(--surface-2);
  }
  .routine.paused .r-title {
    color: var(--faint);
  }
  .r-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .r-title {
    font-weight: 650;
  }
  .r-meta {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .r-meta .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-left: 6px;
  }
  .active-row {
    gap: 10px;
    font-weight: 600;
    cursor: pointer;
  }
</style>
