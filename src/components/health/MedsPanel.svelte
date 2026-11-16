<script lang="ts">
  // Health page, medicines tab: tick each dose as you take it, log as-needed ones, and keep the
  // list of medicines (with their times and how long to take them).
  import { Check, Clock, Pill, Plus, Trash, X } from '@lucide/svelte';
  import Modal from '../Modal.svelte';
  import ColorPicker from '../ColorPicker.svelte';
  import ConfirmButton from '../ConfirmButton.svelte';
  import DateField from '../DateField.svelte';
  import TimeField from '../TimeField.svelte';
  import { api, type Medication, type MedLog } from '../../lib/api';
  import { PALETTE, hex } from '../../lib/colors';
  import { fmt, range, today, weekStart } from '../../lib/dates';
  import { t } from '../../lib/i18n.svelte';
  import { data, load, mutate } from '../../lib/state.svelte';
  import { doseCount, dosesOf, isTaken, medsDue, nowTime } from '../../lib/wellness';

  let { day = $bindable() }: { day: string } = $props();

  let meds = $state<Medication[]>([]);
  let logs = $state<MedLog[]>([]);
  const week = $derived(range(weekStart(day), 7));

  $effect(() => {
    data.version;
    load(api.medications(), []).then((m) => (meds = m));
  });
  $effect(() => {
    data.version;
    load(api.medLogsBetween(week[0], week[6]), []).then((l) => (logs = l));
  });

  const due = $derived(medsDue(meds, day));
  const asNeeded = $derived(meds.filter((m) => m.active && m.as_needed));
  const counts = $derived(doseCount(meds, logs, day));
  const weekCounts = $derived(week.map((d) => ({ date: d, ...doseCount(meds, logs, d) })));
  const dayLogs = (m: Medication) => logs.filter((l) => l.med_id === m.id && l.date === day);

  /** Ticks or unticks one dose. Shown right away; the database catches up. */
  async function toggle(m: Medication, slot: string) {
    const taken = !isTaken(logs, m, day, slot);
    logs = taken
      ? [...logs, { id: -Date.now(), med_id: m.id, date: day, slot, taken_at: Math.floor(Date.now() / 1000) }]
      : logs.filter((l) => !(l.med_id === m.id && l.date === day && l.slot === slot));
    await mutate(api.setMedTaken(m.id, day, slot, taken));
  }
  const logAsNeeded = (m: Medication) =>
    mutate(api.setMedTaken(m.id, day, day === today() ? nowTime() : '12:00', true), t('wl.loggedDose', { name: m.name }));
  const unlog = (l: MedLog) => mutate(api.setMedTaken(l.med_id, l.date, l.slot, false));

  // ---- the list of medicines
  let editing = $state<Medication | null>(null);
  let newTime = $state('08:00');
  const blank = (): Medication => ({
    id: 0,
    name: '',
    dose: '',
    times: ['08:00'],
    as_needed: false,
    color: PALETTE[meds.length % PALETTE.length].value,
    note: '',
    start: null,
    end: null,
    active: true,
    created_at: 0,
  });
  function addTime() {
    if (!editing || editing.times.includes(newTime)) return;
    editing.times = [...editing.times, newTime].sort();
  }
  async function save() {
    if (!editing || !editing.name.trim()) return;
    const ok = await mutate(api.saveMedication({ ...editing, name: editing.name.trim() }), t('wl.medSaved'));
    if (ok !== undefined) editing = null;
  }
  async function remove() {
    if (!editing) return;
    const id = editing.id;
    editing = null;
    await mutate(api.deleteMedication(id), t('wl.medDeleted'));
  }

  function scheduleText(m: Medication) {
    const when = m.as_needed ? t('wl.asNeeded') : m.times.length ? m.times.join(' · ') : t('wl.onceADay');
    const period =
      m.start || m.end
        ? ` · ${m.start ? fmt(m.start, { month: 'short', day: 'numeric' }) : ''}–${m.end ? fmt(m.end, { month: 'short', day: 'numeric' }) : ''}`
        : '';
    return `${when}${period}`;
  }
</script>

<div class="meds">
  <div class="layout">
    <div class="card today">
      <div class="col-head">
        <h3><Pill size={18} /> {t('wl.medsToday')}</h3>
        {#if counts.due}<span class="muted small tabular">{t('wl.dosesTaken', { taken: counts.taken, due: counts.due })}</span>{/if}
      </div>
      {#if counts.due}
        <div class="bar"><span style:width="{(counts.taken / counts.due) * 100}%"></span></div>
      {/if}

      {#each due as m (m.id)}
        <div class="med" style:--c={hex(m.color)}>
          <span class="pill-icon"><Pill size={16} /></span>
          <span class="med-text">
            <span class="strong truncate">{m.name}</span>
            {#if m.dose || m.note}<span class="faint small truncate">{[m.dose, m.note].filter(Boolean).join(' · ')}</span>{/if}
          </span>
          <div class="doses">
            {#each dosesOf(m) as slot (slot)}
              {@const on = isTaken(logs, m, day, slot)}
              <button class="dose" class:on onclick={() => toggle(m, slot)} aria-pressed={on}
                title={on ? t('wl.markNotTaken') : t('wl.markTaken')}>
                {#if on}<Check size={14} strokeWidth={3} />{:else}<Clock size={13} />{/if}
                {slot || t('wl.take')}
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <div class="empty small-empty">
          <span class="empty-icon"><Pill size={26} /></span>
          <p>{meds.length ? t('wl.noMedsToday') : t('wl.noMeds')}</p>
        </div>
      {/each}

      {#if asNeeded.length}
        <div class="label sub-label">{t('wl.asNeeded')}</div>
        {#each asNeeded as m (m.id)}
          {@const taken = dayLogs(m)}
          <div class="med" style:--c={hex(m.color)}>
            <span class="pill-icon"><Pill size={16} /></span>
            <span class="med-text">
              <span class="strong truncate">{m.name}</span>
              {#if m.dose}<span class="faint small truncate">{m.dose}</span>{/if}
            </span>
            <div class="doses">
              {#each taken as l (l.id)}
                <span class="dose on logged">
                  <Check size={14} strokeWidth={3} /> {l.slot}
                  <button class="unlog" onclick={() => unlog(l)} aria-label={t('common.delete')}><X size={12} /></button>
                </span>
              {/each}
              <button class="dose" onclick={() => logAsNeeded(m)}><Plus size={14} /> {t('wl.logDose')}</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <div class="side">
      <div class="card">
        <h3>{t('wl.thisWeek')}</h3>
        <div class="week">
          {#each weekCounts as w (w.date)}
            {@const pct = w.due ? w.taken / w.due : 0}
            <button class="wday" class:active={w.date === day} class:is-today={w.date === today()} onclick={() => (day = w.date)}>
              <span class="wd">{fmt(w.date, { weekday: 'short' })}</span>
              <span class="ring" class:full={w.due > 0 && w.taken === w.due} style:--p="{pct * 100}%">
                <span class="dn">{Number(w.date.slice(8))}</span>
              </span>
              <span class="faint tabular small">{w.due ? `${w.taken}/${w.due}` : '–'}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="card">
        <div class="col-head">
          <h3>{t('wl.myMeds')}</h3>
          <button class="btn small" onclick={() => (editing = blank())}><Plus size={15} /> {t('wl.addMed')}</button>
        </div>
        {#each meds as m (m.id)}
          <button class="med-row" class:stopped={!m.active} style:--c={hex(m.color)} onclick={() => (editing = { ...m, times: [...m.times] })}>
            <span class="dot"></span>
            <span class="med-text">
              <span class="strong truncate">{m.name}{m.dose ? ` · ${m.dose}` : ''}</span>
              <span class="faint small truncate">{scheduleText(m)}{m.active ? '' : ` · ${t('wl.stopped')}`}</span>
            </span>
          </button>
        {:else}
          <p class="faint small none">{t('wl.noMedsHint')}</p>
        {/each}
      </div>
    </div>
  </div>
</div>

{#if editing}
  <Modal title={editing.id ? t('wl.editMed') : t('wl.addMed')} onclose={() => (editing = null)} width={500}>
    <div class="form-row">
      <div class="field grow">
        <label for="med-name">{t('wl.medName')}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="med-name" class="input" placeholder={t('wl.medNamePlaceholder')} bind:value={editing.name} autofocus />
      </div>
      <div class="field dose-field">
        <label for="med-dose">{t('wl.dose')}</label>
        <input id="med-dose" class="input" placeholder={t('wl.dosePlaceholder')} bind:value={editing.dose} />
      </div>
    </div>
    <div class="field">
      <span class="label">{t('wl.when')}</span>
      <div class="segmented">
        <button class:active={!editing.as_needed} onclick={() => editing && (editing.as_needed = false)}>{t('wl.scheduled')}</button>
        <button class:active={editing.as_needed} onclick={() => editing && (editing.as_needed = true)}>{t('wl.asNeeded')}</button>
      </div>
      {#if !editing.as_needed}
        <div class="times">
          {#each editing.times as time (time)}
            <span class="time-chip">
              <Clock size={13} /> {time}
              <button onclick={() => editing && (editing.times = editing.times.filter((x) => x !== time))} aria-label={t('common.delete')}><X size={12} /></button>
            </span>
          {/each}
          <div class="time-add">
            <div class="time-field"><TimeField bind:value={newTime} label={t('wl.time')} /></div>
            <button class="btn small" onclick={addTime}><Plus size={14} /> {t('wl.addTime')}</button>
          </div>
        </div>
        <p class="faint small hint">{editing.times.length ? t('wl.timesHint') : t('wl.noTimesHint')}</p>
      {/if}
    </div>
    <div class="form-row">
      <div class="field grow">
        <span class="label">{t('wl.startDate')}</span>
        <DateField value={editing.start} clearable placeholder={t('wl.noStart')} label={t('wl.startDate')} onchange={(v) => editing && (editing.start = v || null)} />
      </div>
      <div class="field grow">
        <span class="label">{t('wl.endDate')}</span>
        <DateField value={editing.end} clearable placeholder={t('wl.noEnd')} label={t('wl.endDate')} min={editing.start ?? undefined}
          onchange={(v) => editing && (editing.end = v || null)} />
      </div>
    </div>
    <div class="field">
      <label for="med-note">{t('wl.medNote')}</label>
      <input id="med-note" class="input" placeholder={t('wl.medNotePlaceholder')} bind:value={editing.note} />
    </div>
    <div class="field">
      <span class="label">{t('common.color')}</span>
      <ColorPicker bind:value={editing.color} />
    </div>
    <label class="row active-row">
      <input type="checkbox" class="switch" bind:checked={editing.active} /> {t('wl.medActive')}
    </label>
    {#snippet footer()}
      {#if editing?.id}<ConfirmButton onconfirm={remove} question={t('wl.deleteMedQ')} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editing = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={save} disabled={!editing?.name.trim()}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .today {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 6px;
  }
  .col-head h3,
  .card h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .strong {
    font-weight: 650;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .bar {
    height: 8px;
    margin-bottom: 8px;
    border-radius: 8px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 8px;
    background: var(--success);
    transition: width 0.3s ease;
  }
  .med {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 8px;
    border-radius: 12px;
  }
  .med:hover {
    background: var(--surface-2);
  }
  .pill-icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 10px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .med-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .doses {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
  .dose {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 11px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .dose:hover {
    border-color: var(--c);
    color: var(--c);
  }
  .dose.on {
    border-color: var(--c);
    background: var(--c);
    color: #fff;
  }
  .dose.logged {
    cursor: default;
    padding-right: 6px;
  }
  .unlog {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: color-mix(in srgb, #fff 25%, transparent);
    color: inherit;
    cursor: pointer;
  }
  .sub-label {
    padding: 12px 8px 2px;
  }
  .small-empty {
    padding: 22px 12px;
  }
  .week {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 4px;
    margin-top: 12px;
  }
  .wday {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 8px 2px;
    border: none;
    border-radius: 12px;
    background: none;
    cursor: pointer;
  }
  .wday:hover {
    background: var(--surface-2);
  }
  .wday.active {
    background: var(--primary-soft);
  }
  .wd {
    font-size: 11.5px;
    font-weight: 650;
    color: var(--muted);
  }
  .ring {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: conic-gradient(var(--success) var(--p), var(--surface-3) 0);
  }
  .dn {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--surface);
    font-size: 12px;
    font-weight: 700;
  }
  .wday.is-today .dn {
    color: var(--primary);
  }
  .med-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px;
    border: none;
    border-radius: 10px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .med-row:hover {
    background: var(--surface-2);
  }
  .med-row.stopped {
    opacity: 0.55;
  }
  .dot {
    width: 10px;
    height: 10px;
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }
  .none {
    margin: 4px 0 0;
  }
  .form-row {
    display: flex;
    gap: 12px;
  }
  .dose-field {
    width: 150px;
  }
  .times {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
  }
  .time-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 6px 0 10px;
    border-radius: var(--radius-pill);
    background: var(--primary-soft);
    color: var(--primary);
    font-size: 13px;
    font-weight: 650;
  }
  .time-chip button {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  .time-add {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .time-field {
    width: 130px;
  }
  .hint {
    margin: 6px 0 0;
  }
  .active-row {
    gap: 10px;
    font-weight: 600;
    cursor: pointer;
  }
  @container main (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
