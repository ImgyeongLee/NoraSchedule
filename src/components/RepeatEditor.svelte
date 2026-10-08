<script lang="ts">
  // Repeat rule editor: frequency, interval, weekdays (weekly) and when the series ends.
  import { Repeat as RepeatIcon } from '@lucide/svelte';
  import type { Freq, Repeat } from '../lib/api';
  import { parseYmd, weekdayNames } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';

  let { value = $bindable(), startDate }: { value: Repeat | null; startDate: string } = $props();

  const FREQS: Freq[] = ['daily', 'weekly', 'monthly', 'yearly'];
  type Ends = 'never' | 'date' | 'count';
  const ends = $derived<Ends>(!value ? 'never' : value.until ? 'date' : value.count ? 'count' : 'never');

  function setFreq(freq: string) {
    if (freq === 'none') {
      value = null;
      return;
    }
    const weekday = startDate ? parseYmd(startDate).getDay() : 0;
    value = {
      freq: freq as Freq,
      interval: value?.interval ?? 1,
      weekdays: freq === 'weekly' ? (value?.weekdays.length ? value.weekdays : [weekday]) : [],
      until: value?.until ?? null,
      count: value?.count ?? null,
    };
  }

  function setEnds(kind: Ends) {
    if (!value) return;
    value.until = kind === 'date' ? (value.until ?? startDate) : null;
    value.count = kind === 'count' ? (value.count ?? 10) : null;
  }

  function toggleWeekday(d: number) {
    if (!value) return;
    const has = value.weekdays.includes(d);
    // Keep at least one day selected.
    if (has && value.weekdays.length === 1) return;
    value.weekdays = has ? value.weekdays.filter((x) => x !== d) : [...value.weekdays, d].sort();
  }
</script>

<div class="repeat">
  <div class="row head">
    <RepeatIcon size={16} />
    <select class="select" value={value?.freq ?? 'none'} onchange={(e) => setFreq(e.currentTarget.value)} aria-label={t('rep.label')}>
      <option value="none">{t('rep.none')}</option>
      {#each FREQS as f (f)}<option value={f}>{t(`rep.${f}`)}</option>{/each}
    </select>
  </div>

  {#if value}
    <div class="row">
      <span class="lbl">{t('rep.every')}</span>
      <input class="input num" type="number" min="1" max="99" bind:value={value.interval} />
      <span class="muted">{t(`rep.unit.${value.freq}`)}</span>
    </div>

    {#if value.freq === 'weekly'}
      <div class="row">
        <span class="lbl">{t('rep.on')}</span>
        <div class="days">
          {#each weekdayNames('narrow') as name, d (d)}
            <button type="button" class="day" class:on={value.weekdays.includes(d)} onclick={() => toggleWeekday(d)}>{name}</button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="row wrap">
      <span class="lbl">{t('rep.ends')}</span>
      <div class="segmented">
        <button type="button" class:active={ends === 'never'} onclick={() => setEnds('never')}>{t('rep.never')}</button>
        <button type="button" class:active={ends === 'date'} onclick={() => setEnds('date')}>{t('rep.onDate')}</button>
        <button type="button" class:active={ends === 'count'} onclick={() => setEnds('count')}>{t('rep.after')}</button>
      </div>
      {#if ends === 'date'}
        <input class="input date" type="date" min={startDate} value={value.until} onchange={(e) => value && (value.until = e.currentTarget.value || startDate)} />
      {:else if ends === 'count'}
        <input class="input num" type="number" min="1" max="999" bind:value={value.count} />
        <span class="muted">{t('rep.times')}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .repeat {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    gap: 10px;
  }
  .row.wrap {
    flex-wrap: wrap;
  }
  .head {
    color: var(--muted);
  }
  .head .select {
    flex: 1;
    background: var(--surface);
  }
  .lbl {
    min-width: 44px;
    font-size: 12.5px;
    font-weight: 650;
    color: var(--muted);
  }
  .num {
    width: 72px;
    background: var(--surface);
  }
  .date {
    width: auto;
    background: var(--surface);
  }
  .days {
    display: flex;
    gap: 4px;
  }
  .day {
    width: 32px;
    height: 32px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .day.on {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-text);
  }
</style>
