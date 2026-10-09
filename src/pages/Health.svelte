<script lang="ts">
  import { onMount } from 'svelte';
  import {
    CalendarPlus, ChevronLeft, ChevronRight, CopyPlus, Flame, Pencil, Plus, Salad, Settings2, Trash, Utensils, CircleCheck, Circle,
  } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import DateField from '../components/DateField.svelte';
  import Select from '../components/Select.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import { api, type Meal, type MealSlot, type Workout } from '../lib/api';
  import { addDays, fmt, range, today, weekStart } from '../lib/dates';
  import {
    EXERCISES, MAX_KCAL, MAX_MINUTES, SLOTS, dayTotals, estimateBurn, exerciseOf, fmtKcal, fmtNum, healthSettings,
    loadHealthSettings, saveHealthSettings, slotForNow, slotOf,
  } from '../lib/health.svelte';
  import { t } from '../lib/i18n.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';

  let day = $state(today());
  let weekMeals = $state<Meal[]>([]);
  let weekWorkouts = $state<Workout[]>([]);
  let history = $state<Meal[]>([]);

  let editingMeal = $state<Meal | null>(null);
  let editingWorkout = $state<Workout | null>(null);
  let settingsOpen = $state(false);
  let settingsForm = $state({ goal: '', weight: '' });

  // Add-food form.
  let foodName = $state('');
  let foodKcal = $state('');
  let foodSlot = $state<MealSlot>(slotForNow());
  let foodEaten = $state(true);
  let foodInput: HTMLInputElement;

  // Add-exercise form. An empty calorie field means "use the estimate".
  let exKind = $state('walking');
  let exMinutes = $state('30');
  let exKcal = $state('');
  let exNote = $state('');

  const week = $derived(range(weekStart(day), 7));
  const isToday = $derived(day === today());

  onMount(() => {
    if (!healthSettings.loaded) loadHealthSettings();
  });

  $effect(() => {
    data.version;
    const [from, to] = [week[0], week[6]];
    load(api.mealsBetween(from, to), []).then((m) => (weekMeals = m));
    load(api.workoutsBetween(from, to), []).then((w) => (weekWorkouts = w));
  });

  // Foods eaten in the last two months, for one-click re-adding.
  $effect(() => {
    data.version;
    load(api.mealsBetween(addDays(today(), -60), addDays(today(), 7)), []).then((m) => (history = m));
  });

  // Food for a future day is something you are going to eat.
  $effect(() => {
    foodEaten = day <= today();
  });

  const meals = $derived(weekMeals.filter((m) => m.date === day));
  const workouts = $derived(weekWorkouts.filter((w) => w.date === day));
  const totals = $derived(dayTotals(meals, workouts, healthSettings.goal));
  /** What the day allows: the goal plus what exercise burned. */
  const allowance = $derived(healthSettings.goal + totals.burned);
  const scale = $derived(Math.max(allowance, totals.eaten + totals.planned, 1));
  const afterPlan = $derived(totals.left - totals.planned);

  const weekDays = $derived(
    week.map((d) => ({
      date: d,
      ...dayTotals(
        weekMeals.filter((m) => m.date === d),
        weekWorkouts.filter((w) => w.date === d),
        healthSettings.goal,
      ),
    })),
  );

  const recent = $derived.by(() => {
    const byName = new Map<string, { name: string; kcal: number; count: number }>();
    for (const m of history) {
      const key = m.name.trim().toLowerCase();
      const seen = byName.get(key);
      // History is oldest first, so the latest calorie value wins.
      byName.set(key, { name: m.name, kcal: m.kcal, count: (seen?.count ?? 0) + 1 });
    }
    return [...byName.values()].sort((a, b) => b.count - a.count).slice(0, 6);
  });

  const exEstimate = $derived(estimateBurn(exKind, Number(exMinutes) || 0));

  const validKcal = (n: number) => Number.isFinite(n) && n >= 0 && n <= MAX_KCAL;

  async function addFood() {
    const name = foodName.trim();
    if (!name) {
      toast(t('hl.nameRequired'), 'error');
      foodInput?.focus();
      return;
    }
    const kcal = Math.round(Number(foodKcal) || 0);
    if (!validKcal(kcal)) return toast(t('hl.invalidKcal'), 'error');
    const meal: Meal = { id: 0, date: day, slot: foodSlot, name, kcal, eaten: foodEaten };
    const ok = await mutate(api.saveMeal(meal), t('hl.mealAdded', { name }));
    if (ok !== undefined) {
      foodName = '';
      foodKcal = '';
      foodInput?.focus();
    }
  }

  function pickRecent(r: { name: string; kcal: number }) {
    foodName = r.name;
    foodKcal = String(r.kcal);
    foodInput?.focus();
  }

  const setEaten = (m: Meal, eaten: boolean) => mutate(api.saveMeal({ ...m, eaten }));

  async function saveMealEdit() {
    if (!editingMeal) return;
    const name = editingMeal.name.trim();
    if (!name) return toast(t('hl.nameRequired'), 'error');
    const kcal = Math.round(Number(editingMeal.kcal) || 0);
    if (!validKcal(kcal)) return toast(t('hl.invalidKcal'), 'error');
    const ok = await mutate(api.saveMeal({ ...editingMeal, name, kcal }), t('hl.saved'));
    if (ok !== undefined) editingMeal = null;
  }

  async function addWorkout() {
    const minutes = Math.round(Number(exMinutes));
    if (!(minutes > 0 && minutes <= MAX_MINUTES)) return toast(t('hl.invalidMinutes'), 'error');
    const kcal = exKcal.trim() === '' ? exEstimate : Math.round(Number(exKcal));
    if (!validKcal(kcal)) return toast(t('hl.invalidKcal'), 'error');
    const workout: Workout = { id: 0, date: day, kind: exKind, minutes, kcal, note: exNote.trim() };
    const ok = await mutate(api.saveWorkout(workout), t('hl.workoutAdded'));
    if (ok !== undefined) {
      exKcal = '';
      exNote = '';
    }
  }

  async function saveWorkoutEdit() {
    if (!editingWorkout) return;
    const minutes = Math.round(Number(editingWorkout.minutes));
    if (!(minutes > 0 && minutes <= MAX_MINUTES)) return toast(t('hl.invalidMinutes'), 'error');
    const kcal = Math.round(Number(editingWorkout.kcal));
    if (!validKcal(kcal)) return toast(t('hl.invalidKcal'), 'error');
    const ok = await mutate(api.saveWorkout({ ...editingWorkout, minutes, kcal }), t('hl.saved'));
    if (ok !== undefined) editingWorkout = null;
  }

  async function removeMeal(m: Meal) {
    try {
      const stored = await api.deleteMeal(m.id);
      data.version++;
      toast(t('clip.deleted', { name: m.name }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveMeal({ ...stored, id: 0 })),
      });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function removeWorkout(w: Workout) {
    try {
      const stored = await api.deleteWorkout(w.id);
      data.version++;
      toast(t('clip.deleted', { name: t(exerciseOf(w.kind).label) }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveWorkout({ ...stored, id: 0 })),
      });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function mealMenu(ev: MouseEvent, m: Meal) {
    openMenu(ev, [
      { label: t('menu.edit'), icon: Pencil, action: () => (editingMeal = { ...m }) },
      m.eaten
        ? { label: t('hl.markPlanned'), icon: Circle, action: () => setEaten(m, false) }
        : { label: t('hl.markEaten'), icon: CircleCheck, action: () => setEaten(m, true) },
      { label: t('menu.duplicate'), icon: CopyPlus, action: () => mutate(api.saveMeal({ ...m, id: 0 }), t('hl.mealAdded', { name: m.name })) },
      {
        label: t('hl.planTomorrow'),
        icon: CalendarPlus,
        action: () => mutate(api.saveMeal({ ...m, id: 0, date: addDays(m.date, 1), eaten: false }), t('hl.plannedTomorrow')),
      },
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: () => removeMeal(m) },
    ]);
  }

  function workoutMenu(ev: MouseEvent, w: Workout) {
    openMenu(ev, [
      { label: t('menu.edit'), icon: Pencil, action: () => (editingWorkout = { ...w }) },
      { label: t('menu.duplicate'), icon: CopyPlus, action: () => mutate(api.saveWorkout({ ...w, id: 0 }), t('hl.workoutAdded')) },
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: () => removeWorkout(w) },
    ]);
  }

  /** Enter or Space opens a row, like a button. */
  const onRowKey = (e: KeyboardEvent, open: () => void) => {
    if (e.target !== e.currentTarget || (e.key !== 'Enter' && e.key !== ' ')) return;
    e.preventDefault();
    open();
  };

  function openSettings() {
    settingsForm = { goal: String(healthSettings.goal), weight: String(healthSettings.weight) };
    settingsOpen = true;
  }

  async function saveSettings() {
    await saveHealthSettings(Number(settingsForm.goal), Number(settingsForm.weight));
    settingsOpen = false;
  }

  const slotOptions = $derived(SLOTS.map((s) => ({ value: s.id, label: t(s.label), color: s.color })));
  const pct = (n: number) => `${Math.max(0, Math.min(100, (n / scale) * 100))}%`;
</script>

{#snippet unitInput(value: string, set: (v: string) => void, unit: string, opts: { id?: string; placeholder?: string; label?: string; max?: number; onenter?: () => void })}
  <div class="unit-input">
    <input
      id={opts.id}
      class="input tabular"
      type="number"
      inputmode="numeric"
      min="0"
      max={opts.max}
      step="1"
      placeholder={opts.placeholder ?? '0'}
      aria-label={opts.label}
      {value}
      oninput={(e) => set(e.currentTarget.value)}
      onkeydown={(e) => e.key === 'Enter' && opts.onenter?.()}
    />
    <span class="unit">{unit}</span>
  </div>
{/snippet}

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.health')}</h1>
      <p class="sub">{t('hl.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <div class="day-nav">
      <button class="icon-btn" onclick={() => (day = addDays(day, -1))} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
      <div class="day-field"><DateField value={day} label={t('common.date')} onchange={(v) => v && (day = v)} /></div>
      <button class="icon-btn" onclick={() => (day = addDays(day, 1))} aria-label={t('common.next')}><ChevronRight size={18} /></button>
      {#if !isToday}<button class="btn small" onclick={() => (day = today())}>{t('common.today')}</button>{/if}
    </div>
    <button class="btn" onclick={openSettings}><Settings2 size={16} /> {t('hl.settings')}</button>
  </div>

  <div class="card summary">
    <div class="headline">
      <span class="muted strong">{totals.left >= 0 ? t('hl.left') : t('hl.over')}</span>
      <span class="big tabular" class:over={totals.left < 0}>{fmtNum(Math.abs(totals.left))}<small>kcal</small></span>
      <span class="faint small">
        {t('hl.formula', { goal: fmtNum(healthSettings.goal), eaten: fmtNum(totals.eaten), burned: fmtNum(totals.burned) })}
      </span>
    </div>
    <div class="meter-wrap">
      <div class="meter" role="img" aria-label={t('hl.meterLabel', { eaten: fmtKcal(totals.eaten), allowance: fmtKcal(allowance) })}>
        <span class="seg eaten" class:over={totals.eaten > allowance} style:width={pct(totals.eaten)}></span>
        <span class="seg planned" class:over={totals.eaten + totals.planned > allowance} style:width={pct(totals.planned)}></span>
        {#if scale > allowance}<span class="limit" style:left={pct(allowance)}></span>{/if}
      </div>
      <div class="legend">
        <span class="lg"><span class="sw eaten"></span><Utensils size={14} /> {t('hl.eaten')} <b class="tabular">{fmtKcal(totals.eaten)}</b></span>
        <span class="lg"><span class="sw planned"></span><Circle size={14} /> {t('hl.planned')} <b class="tabular">{fmtKcal(totals.planned)}</b></span>
        <span class="lg"><span class="sw burned"></span><Flame size={14} /> {t('hl.burned')} <b class="tabular">{fmtKcal(totals.burned)}</b>
          {#if totals.minutes}<span class="faint">· {t('hl.minutes', { n: totals.minutes })}</span>{/if}</span>
      </div>
      {#if totals.planned > 0}
        <p class="after small" class:over={afterPlan < 0}>
          {afterPlan >= 0 ? t('hl.afterPlanLeft', { kcal: fmtKcal(afterPlan) }) : t('hl.afterPlanOver', { kcal: fmtKcal(-afterPlan) })}
        </p>
      {/if}
    </div>
  </div>

  <div class="week">
    {#each weekDays as w (w.date)}
      {@const fill = healthSettings.goal + w.burned}
      <button class="wday" class:active={w.date === day} class:is-today={w.date === today()} onclick={() => (day = w.date)}>
        <span class="wd">{fmt(w.date, { weekday: 'short' })}</span>
        <span class="dn">{Number(w.date.slice(8))}</span>
        <span class="mini" class:over={w.eaten > fill}>
          <span class="mini-eaten" style:width="{Math.min(100, (w.eaten / fill) * 100)}%"></span>
          <span class="mini-planned" style:width="{Math.min(100 - Math.min(100, (w.eaten / fill) * 100), (w.planned / fill) * 100)}%"></span>
        </span>
        <span class="wk tabular">{w.eaten || w.planned ? fmtNum(w.eaten) : '–'}</span>
        <span class="burn tabular" class:none={!w.burned}><Flame size={11} /> {fmtNum(w.burned)}</span>
      </button>
    {/each}
  </div>

  <div class="layout">
    <div class="card col">
      <div class="col-head">
        <h3><Utensils size={18} /> {t('hl.meals')}</h3>
        <span class="muted small tabular">{t('hl.mealsTotal', { eaten: fmtKcal(totals.eaten), planned: fmtKcal(totals.planned) })}</span>
      </div>

      <div class="form">
        <div class="form-row">
          <div class="segmented">
            <button class:active={foodEaten} onclick={() => (foodEaten = true)}><CircleCheck size={15} /> {t('hl.ateIt')}</button>
            <button class:active={!foodEaten} onclick={() => (foodEaten = false)}><Circle size={15} /> {t('hl.willEat')}</button>
          </div>
          <div class="slot-pick"><Select bind:value={foodSlot} options={slotOptions} label={t('hl.slot')} /></div>
        </div>
        <div class="form-row">
          <input
            bind:this={foodInput}
            class="input grow"
            placeholder={foodEaten ? t('hl.foodPlaceholder') : t('hl.foodPlaceholderPlan')}
            aria-label={t('hl.foodName')}
            bind:value={foodName}
            onkeydown={(e) => e.key === 'Enter' && addFood()}
          />
          <div class="kcal-field">
            {@render unitInput(foodKcal, (v) => (foodKcal = v), 'kcal', { label: t('hl.kcal'), max: MAX_KCAL, onenter: addFood })}
          </div>
          <button class="btn primary" onclick={addFood}><Plus size={17} /> {t('common.add')}</button>
        </div>
        {#if recent.length}
          <div class="recent">
            <span class="faint small">{t('hl.recent')}</span>
            {#each recent as r (r.name)}
              <button class="recent-chip" onclick={() => pickRecent(r)}>
                <span class="truncate">{r.name}</span><span class="faint tabular">{fmtNum(r.kcal)}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      {#each SLOTS as s (s.id)}
        {@const list = meals.filter((m) => m.slot === s.id)}
        <div class="slot-head" style:--c={s.color}>
          <span class="slot-icon"><s.icon size={15} /></span>
          <span class="strong">{t(s.label)}</span>
          <span class="spacer"></span>
          {#if list.length}<span class="muted small tabular">{fmtKcal(list.reduce((sum, m) => sum + m.kcal, 0))}</span>{/if}
        </div>
        {#each list as m (m.id)}
          <div
            class="item"
            class:planned={!m.eaten}
            role="button"
            tabindex="0"
            onclick={() => (editingMeal = { ...m })}
            onkeydown={(e) => onRowKey(e, () => (editingMeal = { ...m }))}
            oncontextmenu={(e) => mealMenu(e, m)}
          >
            <input
              type="checkbox"
              class="check"
              checked={m.eaten}
              title={m.eaten ? t('hl.markPlanned') : t('hl.markEaten')}
              aria-label={m.eaten ? t('hl.markPlanned') : t('hl.markEaten')}
              onclick={(e) => e.stopPropagation()}
              onchange={() => setEaten(m, !m.eaten)}
            />
            <span class="item-name truncate">{m.name}</span>
            {#if !m.eaten}<span class="chip planned-chip">{t('hl.plannedBadge')}</span>{/if}
            <span class="item-kcal tabular">{fmtNum(m.kcal)}<small>kcal</small></span>
          </div>
        {:else}
          <p class="faint small none">{t('hl.noMeals')}</p>
        {/each}
      {/each}
    </div>

    <div class="card col">
      <div class="col-head">
        <h3><Flame size={18} /> {t('hl.exercise')}</h3>
        <span class="muted small tabular">
          {fmtKcal(totals.burned)}{#if totals.minutes}{` · ${t('hl.minutes', { n: totals.minutes })}`}{/if}
        </span>
      </div>

      <div class="form">
        <div class="kinds">
          {#each EXERCISES as x (x.id)}
            <button type="button" class="kind" class:on={exKind === x.id} style:--c={x.color} onclick={() => (exKind = x.id)} title={t(x.label)}>
              <span class="kind-icon"><x.icon size={17} /></span>
              <span class="truncate">{t(x.label)}</span>
            </button>
          {/each}
        </div>
        <div class="form-row">
          <div class="field-sm">
            <span class="label">{t('hl.duration')}</span>
            {@render unitInput(exMinutes, (v) => (exMinutes = v), t('hl.minUnit'), { label: t('hl.duration'), max: MAX_MINUTES, onenter: addWorkout })}
          </div>
          <div class="field-sm">
            <span class="label">{t('hl.burnKcal')}</span>
            {@render unitInput(exKcal, (v) => (exKcal = v), 'kcal', { label: t('hl.burnKcal'), placeholder: String(exEstimate), max: MAX_KCAL, onenter: addWorkout })}
          </div>
        </div>
        <div class="quick-min">
          {#each [15, 30, 45, 60, 90] as n (n)}
            <button class="chip-btn" class:on={Number(exMinutes) === n} onclick={() => (exMinutes = String(n))}>{t('hl.minutes', { n })}</button>
          {/each}
        </div>
        <div class="estimate" style:--c={exerciseOf(exKind).color}>
          <Flame size={18} />
          <div>
            <div class="strong tabular">{t('hl.estimate', { kcal: fmtKcal(exEstimate) })}</div>
            <div class="faint small">{t('hl.estimateBasis', { kg: healthSettings.weight })}{exKcal.trim() ? '' : ` · ${t('hl.ownKcalHint')}`}</div>
          </div>
        </div>
        <div class="form-row">
          <input class="input grow" placeholder={t('hl.notePlaceholder')} bind:value={exNote} onkeydown={(e) => e.key === 'Enter' && addWorkout()} />
          <button class="btn primary" onclick={addWorkout}><Plus size={17} /> {t('common.add')}</button>
        </div>
      </div>

      {#each workouts as w (w.id)}
        {@const x = exerciseOf(w.kind)}
        <div
          class="item workout"
          style:--c={x.color}
          role="button"
          tabindex="0"
          onclick={() => (editingWorkout = { ...w })}
          onkeydown={(e) => onRowKey(e, () => (editingWorkout = { ...w }))}
          oncontextmenu={(e) => workoutMenu(e, w)}
        >
          <span class="kind-icon"><x.icon size={16} /></span>
          <span class="item-text">
            <span class="item-name truncate">{t(x.label)}</span>
            {#if w.note}<span class="faint small truncate">{w.note}</span>{/if}
          </span>
          <span class="muted small tabular">{t('hl.minutes', { n: w.minutes })}</span>
          <span class="item-kcal tabular">{fmtNum(w.kcal)}<small>kcal</small></span>
        </div>
      {:else}
        <div class="empty small-empty">
          <span class="empty-icon"><Salad size={26} /></span>
          <p>{t('hl.noWorkouts')}</p>
        </div>
      {/each}
    </div>
  </div>
</div>

{#if editingMeal}
  <Modal title={t('hl.editMeal')} onclose={() => (editingMeal = null)} width={460}>
    <div class="field">
      <label for="hl-food">{t('hl.foodName')}</label>
      <input id="hl-food" class="input" bind:value={editingMeal.name} onkeydown={(e) => e.key === 'Enter' && saveMealEdit()} />
    </div>
    <div class="form-row">
      <div class="field grow">
        <label for="hl-kcal">{t('hl.kcal')}</label>
        {@render unitInput(String(editingMeal.kcal), (v) => editingMeal && (editingMeal.kcal = Number(v)), 'kcal', { id: 'hl-kcal', max: MAX_KCAL, onenter: saveMealEdit })}
      </div>
      <div class="field grow">
        <label for="hl-slot">{t('hl.slot')}</label>
        <Select id="hl-slot" bind:value={editingMeal.slot} options={slotOptions} />
      </div>
    </div>
    <div class="form-row">
      <div class="segmented">
        <button class:active={editingMeal.eaten} onclick={() => editingMeal && (editingMeal.eaten = true)}><CircleCheck size={15} /> {t('hl.ateIt')}</button>
        <button class:active={!editingMeal.eaten} onclick={() => editingMeal && (editingMeal.eaten = false)}><Circle size={15} /> {t('hl.willEat')}</button>
      </div>
      <div class="grow"><DateField value={editingMeal.date} label={t('common.date')} onchange={(v) => editingMeal && v && (editingMeal.date = v)} /></div>
    </div>
    {#snippet footer()}
      <ConfirmButton onconfirm={() => { const m = editingMeal; editingMeal = null; if (m) removeMeal(m); }} />
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingMeal = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveMealEdit}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

{#if editingWorkout}
  {@const estimate = estimateBurn(editingWorkout.kind, Number(editingWorkout.minutes) || 0)}
  <Modal title={t('hl.editWorkout')} onclose={() => (editingWorkout = null)} width={520}>
    <div class="kinds">
      {#each EXERCISES as x (x.id)}
        <button type="button" class="kind" class:on={editingWorkout.kind === x.id} style:--c={x.color} onclick={() => editingWorkout && (editingWorkout.kind = x.id)}>
          <span class="kind-icon"><x.icon size={17} /></span>
          <span class="truncate">{t(x.label)}</span>
        </button>
      {/each}
    </div>
    <div class="form-row">
      <div class="field grow">
        <label for="hl-min">{t('hl.duration')}</label>
        {@render unitInput(String(editingWorkout.minutes), (v) => editingWorkout && (editingWorkout.minutes = Number(v)), t('hl.minUnit'), { id: 'hl-min', max: MAX_MINUTES, onenter: saveWorkoutEdit })}
      </div>
      <div class="field grow">
        <label for="hl-burn">{t('hl.burnKcal')}</label>
        {@render unitInput(String(editingWorkout.kcal), (v) => editingWorkout && (editingWorkout.kcal = Number(v)), 'kcal', { id: 'hl-burn', max: MAX_KCAL, onenter: saveWorkoutEdit })}
      </div>
    </div>
    {#if estimate !== Number(editingWorkout.kcal)}
      <button class="btn small use-estimate" onclick={() => editingWorkout && (editingWorkout.kcal = estimate)}>
        <Flame size={14} /> {t('hl.useEstimate', { kcal: fmtKcal(estimate) })}
      </button>
    {/if}
    <div class="form-row">
      <input class="input grow" placeholder={t('hl.notePlaceholder')} bind:value={editingWorkout.note} />
      <div class="date-sm"><DateField value={editingWorkout.date} label={t('common.date')} onchange={(v) => editingWorkout && v && (editingWorkout.date = v)} /></div>
    </div>
    {#snippet footer()}
      <ConfirmButton onconfirm={() => { const w = editingWorkout; editingWorkout = null; if (w) removeWorkout(w); }} />
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingWorkout = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveWorkoutEdit}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

{#if settingsOpen}
  <Modal title={t('hl.settings')} onclose={() => (settingsOpen = false)} width={420}>
    <div class="field">
      <label for="hl-goal">{t('hl.goal')}</label>
      {@render unitInput(settingsForm.goal, (v) => (settingsForm.goal = v), 'kcal', { id: 'hl-goal', max: MAX_KCAL, onenter: saveSettings })}
    </div>
    <div class="field">
      <label for="hl-weight">{t('hl.weight')}</label>
      {@render unitInput(settingsForm.weight, (v) => (settingsForm.weight = v), 'kg', { id: 'hl-weight', max: 500, onenter: saveSettings })}
      <span class="faint small">{t('hl.weightHint')}</span>
    </div>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (settingsOpen = false)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveSettings}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .strong {
    font-weight: 650;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .day-nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .day-field {
    width: 210px;
  }

  /* ---- Today's summary ---- */
  .summary {
    display: grid;
    grid-template-columns: minmax(200px, 0.8fr) minmax(0, 2fr);
    gap: 28px;
    align-items: center;
    margin-bottom: 14px;
  }
  .headline {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .big {
    font-size: 40px;
    font-weight: 800;
    letter-spacing: -0.03em;
    line-height: 1.1;
    color: var(--success);
  }
  .big small {
    margin-left: 4px;
    font-size: 16px;
    font-weight: 650;
    color: var(--muted);
    letter-spacing: 0;
  }
  .big.over {
    color: var(--danger);
  }
  .meter-wrap {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .meter {
    position: relative;
    display: flex;
    height: 16px;
    border-radius: 16px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .seg {
    height: 100%;
    transition: width 0.3s ease;
  }
  .seg.eaten {
    background: var(--primary);
  }
  .seg.planned {
    background: repeating-linear-gradient(
      -45deg,
      color-mix(in srgb, var(--primary) 45%, transparent) 0 6px,
      color-mix(in srgb, var(--primary) 22%, transparent) 6px 12px
    );
  }
  .seg.eaten.over {
    background: var(--danger);
  }
  .seg.planned.over {
    background: repeating-linear-gradient(
      -45deg,
      color-mix(in srgb, var(--danger) 45%, transparent) 0 6px,
      color-mix(in srgb, var(--danger) 22%, transparent) 6px 12px
    );
  }
  .limit {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--text);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    font-size: 13px;
    color: var(--muted);
  }
  .lg {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .lg b {
    color: var(--text);
    font-weight: 700;
  }
  .sw {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
  .sw.eaten {
    background: var(--primary);
  }
  .sw.planned {
    background: color-mix(in srgb, var(--primary) 35%, transparent);
  }
  .sw.burned {
    background: var(--warning);
  }
  .after {
    margin: 0;
    color: var(--muted);
  }
  .after.over {
    color: var(--danger);
  }

  /* ---- Week strip ---- */
  .week {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 16px;
  }
  .wday {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    min-width: 0;
    padding: 10px 6px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
  }
  .wday:hover {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  }
  .wday.active {
    border-color: var(--primary);
    background: color-mix(in srgb, var(--primary) 7%, var(--surface));
  }
  .wd {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .dn {
    font-size: 17px;
    font-weight: 750;
  }
  .wday.is-today .dn {
    color: var(--primary);
  }
  .mini {
    display: flex;
    width: 100%;
    max-width: 64px;
    height: 6px;
    margin: 3px 0;
    border-radius: 6px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .mini-eaten {
    background: var(--primary);
  }
  .mini-planned {
    background: color-mix(in srgb, var(--primary) 35%, transparent);
  }
  .mini.over .mini-eaten {
    background: var(--danger);
  }
  .wk {
    font-size: 12px;
    font-weight: 650;
  }
  .burn {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 11px;
    font-weight: 600;
    color: var(--warning);
  }
  .burn.none {
    visibility: hidden;
  }

  /* ---- Meals and exercise ---- */
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .col-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 10px;
  }
  .col-head h3 {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    margin-bottom: 8px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .form-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .form-row > .btn {
    height: 40px;
  }
  .form .input {
    background: var(--surface);
  }
  .form :global(.select-btn),
  .form :global(.date-field) {
    background: var(--surface);
  }
  .slot-pick {
    flex: 1;
    min-width: 120px;
  }
  .kcal-field {
    width: 120px;
  }
  .unit-input {
    position: relative;
    display: flex;
    align-items: center;
  }
  .unit-input .input {
    width: 100%;
    padding-right: 46px;
  }
  .unit-input .input::-webkit-inner-spin-button,
  .unit-input .input::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .unit {
    position: absolute;
    right: 12px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--faint);
    pointer-events: none;
  }
  .recent {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .recent-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 180px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .recent-chip:hover {
    border-color: var(--primary);
  }
  .slot-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 4px 4px;
  }
  .slot-icon {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .none {
    margin: 0;
    padding: 4px 6px 4px 38px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 8px 6px;
    border-radius: 12px;
    cursor: pointer;
  }
  .item:hover {
    background: var(--surface-2);
  }
  .item:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: -2px;
  }
  .item .check {
    margin-left: 2px;
  }
  .item-name {
    flex: 1;
    font-weight: 600;
  }
  .item.planned .item-name,
  .item.planned .item-kcal {
    color: var(--muted);
  }
  .planned-chip {
    height: 20px;
    padding: 0 8px;
    font-size: 11px;
    color: var(--primary);
    background: color-mix(in srgb, var(--primary) 12%, transparent);
  }
  .item-kcal {
    font-weight: 750;
  }
  .item-kcal small {
    margin-left: 2px;
    font-size: 11px;
    font-weight: 600;
    color: var(--faint);
  }
  .item-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: 6px;
  }
  .kind {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    min-width: 0;
    padding: 8px 4px;
    border: 1.5px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
  }
  .kind .truncate {
    max-width: 100%;
  }
  .kind:hover {
    border-color: var(--c);
  }
  .kind.on {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 10%, var(--surface));
  }
  .kind-icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 10px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .field-sm {
    flex: 1;
    min-width: 120px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .quick-min {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip-btn {
    height: 28px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--muted);
    cursor: pointer;
  }
  .chip-btn:hover {
    color: var(--text);
  }
  .chip-btn.on {
    border-color: var(--primary);
    color: var(--primary);
    background: color-mix(in srgb, var(--primary) 8%, var(--surface));
  }
  .estimate {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    color: var(--c);
    background: color-mix(in srgb, var(--c) 10%, var(--surface));
  }
  .estimate .strong {
    color: var(--text);
  }
  .small-empty {
    padding: 22px 12px;
  }
  .use-estimate {
    align-self: flex-start;
  }
  .date-sm {
    width: 220px;
  }
  @container main (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .summary {
      grid-template-columns: minmax(0, 1fr);
      gap: 16px;
    }
  }
  @container main (max-width: 560px) {
    .week {
      gap: 4px;
    }
    .wday {
      padding: 8px 2px;
    }
    .wk,
    .burn {
      display: none;
    }
  }
</style>
