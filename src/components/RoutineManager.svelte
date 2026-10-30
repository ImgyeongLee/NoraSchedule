<script lang="ts">
  // Routines: todos that are added to the list by themselves every day (or on chosen weekdays).
  import { Plus, Repeat } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import ConfirmButton from './ConfirmButton.svelte';
  import Select from './Select.svelte';
  import { api, type Routine, type TodoGroup } from '../lib/api';
  import { hex } from '../lib/colors';
  import { weekdayNames } from '../lib/dates';
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

  // ---- new routine
  let title = $state('');
  let days = $state<number[]>([]);
  let group = $state<number | null>(null);

  const toggleDay = (list: number[], d: number) => (list.includes(d) ? list.filter((x) => x !== d) : [...list, d]);

  async function add() {
    const name = title.trim();
    if (!name) return;
    const ok = await mutate(
      api.saveRoutine({ id: 0, title: name, group_id: group, weekdays: days, active: true, created_at: 0 }),
      t('routine.added'),
    );
    if (ok !== undefined) {
      title = '';
      days = [];
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
    {@render dayPicker(days, (v) => (days = v))}
  </div>

  <div class="card list">
    {#each routines as r (r.id)}
      {@const g = r.group_id !== null ? groupMap.get(r.group_id) : undefined}
      <div class="routine" class:paused={!r.active}>
        <input type="checkbox" class="switch" checked={r.active} onchange={(e) => setActive(r, e.currentTarget.checked)}
          aria-label={t('routine.active')} title={r.active ? t('routine.pause') : t('routine.resume')} />
        <button class="r-main" onclick={() => (editing = { ...r, weekdays: [...r.weekdays] })}>
          <span class="r-title truncate">{r.title}</span>
          <span class="r-meta muted small">
            <Repeat size={12} /> {daysLabel(r.weekdays)}
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
      {@render dayPicker(editing.weekdays, (v) => editing && (editing.weekdays = v))}
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
