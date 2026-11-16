<script lang="ts">
  // Health page, sleep tab: the night before the chosen day (bed and wake-up time, how well you
  // slept, waking up, naps) and the week's nights.
  import { untrack } from 'svelte';
  import { BedDouble, Moon, Sunrise, Trash } from '@lucide/svelte';
  import BarChart from '../BarChart.svelte';
  import TimeField from '../TimeField.svelte';
  import { api, type SleepLog } from '../../lib/api';
  import { addDays, fmt, range, weekStart } from '../../lib/dates';
  import { t } from '../../lib/i18n.svelte';
  import { numeric } from '../../lib/numberInput';
  import { data, load, mutate, toast } from '../../lib/state.svelte';
  import { SCALE_COLORS, averageTime, averageWake, fmtDuration, sleepMinutes } from '../../lib/wellness';

  let { day = $bindable() }: { day: string } = $props();

  let nights = $state<SleepLog[]>([]);
  const week = $derived(range(weekStart(day), 7));
  $effect(() => {
    data.version;
    load(api.sleepBetween(week[0], week[6]), []).then((n) => (nights = n));
  });

  const saved = $derived(nights.find((n) => n.date === day) ?? null);
  // The form starts from the saved night, or from the last night recorded this week.
  const blank = (): SleepLog => {
    const last = [...nights].reverse().find((n) => n.date < day);
    return { id: 0, date: day, bed: last?.bed ?? '23:30', wake: last?.wake ?? '07:00', quality: 0, wakeups: 0, nap_minutes: 0, note: '' };
  };
  let form = $state<SleepLog>({ id: 0, date: '', bed: '23:30', wake: '07:00', quality: 0, wakeups: 0, nap_minutes: 0, note: '' });
  let dirty = $state(false);
  $effect(() => {
    // Reload the form when the day (or what is saved for it) changes, unless edits are pending.
    const next = saved ? { ...saved } : blank();
    untrack(() => {
      if (!dirty || form.date !== next.date) {
        form = next;
        dirty = false;
      }
    });
  });
  const edit = (patch: Partial<SleepLog>) => {
    form = { ...form, ...patch };
    dirty = true;
  };

  const minutes = $derived(sleepMinutes(form.bed, form.wake));
  async function save() {
    const ok = await mutate(api.saveSleep({ ...form, date: day }), t('wl.sleepSaved'));
    if (ok !== undefined) dirty = false;
  }
  async function remove() {
    if (!saved) return;
    try {
      const stored = await api.deleteSleep(saved.id);
      dirty = false;
      data.version++;
      toast(t('wl.sleepDeleted'), 'info', { label: t('common.undo'), run: () => stored && mutate(api.saveSleep({ ...stored, id: 0 })) });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  const chart = $derived(
    week.map((d) => {
      const n = nights.find((x) => x.date === d);
      return { label: fmt(d, { weekday: 'short' }), value: n ? sleepMinutes(n.bed, n.wake) / 60 : 0 };
    }),
  );
  const avg = $derived(
    nights.length ? nights.reduce((s, n) => s + sleepMinutes(n.bed, n.wake), 0) / nights.length : 0,
  );
  const rated = $derived(nights.filter((n) => n.quality > 0));
  const QUALITY = [1, 2, 3, 4, 5];
</script>

<div class="sleep">
  <div class="layout">
    <div class="card night">
      <div class="col-head">
        <h3><BedDouble size={18} /> {t('wl.lastNight')}</h3>
        <span class="muted small">{t('wl.nightOf', { from: fmt(addDays(day, -1), { month: 'short', day: 'numeric' }), to: fmt(day, { month: 'short', day: 'numeric' }) })}</span>
      </div>

      <div class="times">
        <div class="time">
          <span class="label"><Moon size={14} /> {t('wl.bedTime')}</span>
          <TimeField value={form.bed} label={t('wl.bedTime')} onchange={(v) => edit({ bed: v })} />
        </div>
        <div class="duration">
          <span class="big tabular">{fmtDuration(minutes)}</span>
          <span class="faint small">{t('wl.asleep')}</span>
        </div>
        <div class="time">
          <span class="label"><Sunrise size={14} /> {t('wl.wakeTime')}</span>
          <TimeField value={form.wake} label={t('wl.wakeTime')} onchange={(v) => edit({ wake: v })} />
        </div>
      </div>

      <div class="field">
        <span class="label">{t('wl.quality')}</span>
        <div class="quality" role="group" aria-label={t('wl.quality')}>
          {#each QUALITY as q (q)}
            <button class="q" class:on={form.quality === q} style:--c={SCALE_COLORS[q - 1]} aria-pressed={form.quality === q}
              onclick={() => edit({ quality: form.quality === q ? 0 : q })}>
              <span class="q-n">{q}</span>
              <span class="q-l">{t(`wl.quality.${q}` as 'wl.quality.1')}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="form-row">
        <div class="field grow">
          <label for="sl-wakeups">{t('wl.wakeups')}</label>
          <div class="unit-input">
            <input id="sl-wakeups" class="input tabular" type="text" use:numeric={{ max: 99 }} placeholder="0" value={form.wakeups || ''}
              oninput={(e) => edit({ wakeups: Number(e.currentTarget.value) || 0 })} />
            <span class="unit">{t('wl.timesUnit')}</span>
          </div>
        </div>
        <div class="field grow">
          <label for="sl-nap">{t('wl.nap')}</label>
          <div class="unit-input">
            <input id="sl-nap" class="input tabular" type="text" use:numeric={{ max: 1440 }} placeholder="0" value={form.nap_minutes || ''}
              oninput={(e) => edit({ nap_minutes: Number(e.currentTarget.value) || 0 })} />
            <span class="unit">{t('hl.minUnit')}</span>
          </div>
        </div>
      </div>

      <div class="field">
        <label for="sl-note">{t('wl.sleepNote')}</label>
        <textarea id="sl-note" class="textarea" rows="2" placeholder={t('wl.sleepNotePlaceholder')} value={form.note}
          oninput={(e) => edit({ note: e.currentTarget.value })}></textarea>
      </div>

      <div class="actions">
        {#if saved}
          <button class="btn ghost danger" onclick={remove}><Trash size={15} /> {t('common.delete')}</button>
        {/if}
        <span class="spacer"></span>
        {#if saved && !dirty}<span class="faint small">{t('wl.savedState')}</span>{/if}
        <button class="btn primary" onclick={save} disabled={!!saved && !dirty}>{saved ? t('common.save') : t('wl.saveSleep')}</button>
      </div>
    </div>

    <div class="card">
      <div class="col-head">
        <h3>{t('wl.thisWeek')}</h3>
        {#if avg}<span class="muted small">{t('wl.avgSleep', { time: fmtDuration(avg) })}</span>{/if}
      </div>
      <BarChart data={chart} color="var(--long-break)" height={180} format={(v) => (v ? fmtDuration(Math.round(v * 60)) : '')}
        onselect={(i) => (day = week[i])} />
      <div class="stats">
        <div class="stat"><span class="faint small">{t('wl.avgBed')}</span><b class="tabular">{averageTime(nights.map((n) => n.bed)) || '–'}</b></div>
        <div class="stat"><span class="faint small">{t('wl.avgWake')}</span><b class="tabular">{averageWake(nights.map((n) => n.wake)) || '–'}</b></div>
        <div class="stat">
          <span class="faint small">{t('wl.avgQuality')}</span>
          <b class="tabular">{rated.length ? (rated.reduce((s, n) => s + n.quality, 0) / rated.length).toFixed(1) : '–'}</b>
        </div>
        <div class="stat"><span class="faint small">{t('wl.nightsLogged')}</span><b class="tabular">{nights.length}/7</b></div>
      </div>
    </div>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .night {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px 10px;
  }
  .col-head h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .times {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: end;
    gap: 14px;
  }
  .time {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .duration {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-bottom: 2px;
  }
  .big {
    font-size: 24px;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--long-break);
  }
  .quality {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 6px;
  }
  .q {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 8px 4px;
    border: 1.5px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }
  .q:hover {
    border-color: var(--c);
  }
  .q.on {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, var(--surface));
    color: var(--c);
  }
  .q-n {
    font-size: 16px;
    font-weight: 800;
  }
  .q-l {
    font-size: 11px;
    font-weight: 650;
    text-align: center;
  }
  .form-row {
    display: flex;
    gap: 12px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .unit-input {
    position: relative;
  }
  .unit-input .input {
    padding-right: 52px;
  }
  .unit {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--faint);
    font-size: 13px;
    pointer-events: none;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-top: 14px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .stat b {
    font-size: 17px;
  }
  @container main (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
