<script lang="ts">
  // Time picker: a button showing the time, opening either a list of times (every 15 minutes)
  // or, when chosen in Settings, AM/PM with hour 1–12 and minute 0–59 columns.
  // Both have a box to type any exact time, e.g. "9:05", "0905" or "21:30".
  import { Clock, ChevronDown } from '@lucide/svelte';
  import { tick } from 'svelte';
  import Popover from './Popover.svelte';
  import { pad } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { calPrefs } from '../lib/calPrefs.svelte';

  let { value = $bindable(), label, onchange }: { value: string; label?: string; onchange?: (value: string) => void } = $props();

  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLDivElement>();
  let open = $state(false);
  let typed = $state('');

  const TIMES = Array.from({ length: 96 }, (_, i) => `${pad(Math.floor(i / 4))}:${pad((i % 4) * 15)}`);
  const HOURS = Array.from({ length: 12 }, (_, i) => i + 1);
  const MINUTES = Array.from({ length: 60 }, (_, i) => i);

  const precise = $derived(calPrefs.timePicker === 'precise');
  const h24 = $derived(Number(value.slice(0, 2)) || 0);
  const minute = $derived(Number(value.slice(3, 5)) || 0);
  const pm = $derived(h24 >= 12);
  const h12 = $derived(h24 % 12 || 12);
  let hourCol = $state<HTMLDivElement>();
  let minuteCol = $state<HTMLDivElement>();

  /** Sets the time without closing (precise mode). */
  function set(hour: number, min: number) {
    const next = `${pad(hour)}:${pad(min)}`;
    value = next;
    onchange?.(next);
  }
  const setHour12 = (h: number) => set((h % 12) + (pm ? 12 : 0), minute);
  const setPm = (on: boolean) => set((h24 % 12) + (on ? 12 : 0), minute);

  /** Reads "9", "930", "09:30", "9.30" … as HH:MM, or null if it is not a time. */
  function parse(text: string): string | null {
    const digits = text.replace(/[^0-9]/g, '');
    if (!digits) return null;
    const [h, m] = digits.length <= 2 ? [Number(digits), 0] : [Number(digits.slice(0, -2)), Number(digits.slice(-2))];
    return h < 24 && m < 60 ? `${pad(h)}:${pad(m)}` : null;
  }

  function pick(time: string) {
    value = time;
    open = false;
    onchange?.(time);
  }

  async function toggle() {
    open = !open;
    if (!open) return;
    typed = value;
    await tick();
    if (precise) {
      hourCol?.querySelector<HTMLElement>('.on')?.scrollIntoView({ block: 'center' });
      minuteCol?.querySelector<HTMLElement>('.on')?.scrollIntoView({ block: 'center' });
      return;
    }
    // Show the current time (or the nearest quarter) in the middle of the list.
    const [h, m] = value.split(':').map(Number);
    list?.querySelector<HTMLElement>(`[data-i="${h * 4 + Math.floor(m / 15)}"]`)?.scrollIntoView({ block: 'center' });
  }
</script>

<button bind:this={button} type="button" class="time-field" class:open aria-label={label} aria-haspopup="listbox" onclick={toggle}>
  <Clock size={15} />
  <span class="tabular">{value}</span>
  <ChevronDown size={13} />
</button>

{#if open && button}
  <Popover anchor={button} onclose={() => (open = false)} width={precise ? 220 : 180}>
    <input
      class="input typed"
      bind:value={typed}
      placeholder="HH:MM"
      aria-label={t('time.type')}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          const time = parse(typed);
          if (time) pick(time);
        }
      }}
    />
    {#if precise}
      <div class="segmented ampm">
        <button type="button" class:active={!pm} onclick={() => setPm(false)}>{t('time.am')}</button>
        <button type="button" class:active={pm} onclick={() => setPm(true)}>{t('time.pm')}</button>
      </div>
      <div class="cols">
        <div class="list col" bind:this={hourCol} role="listbox" aria-label={t('time.hour')}>
          {#each HOURS as h (h)}
            <button type="button" class="opt tabular" class:on={h === h12} role="option" aria-selected={h === h12} onclick={() => setHour12(h)}>
              {h}{t('time.hourUnit')}
            </button>
          {/each}
        </div>
        <div class="list col" bind:this={minuteCol} role="listbox" aria-label={t('time.minute')}>
          {#each MINUTES as m (m)}
            <button type="button" class="opt tabular" class:on={m === minute} role="option" aria-selected={m === minute} onclick={() => set(h24, m)}>
              {pad(m)}{t('time.minuteUnit')}
            </button>
          {/each}
        </div>
      </div>
      <button type="button" class="btn primary small done" onclick={() => (open = false)}>{t('common.done')}</button>
    {:else}
      <div class="list" bind:this={list} role="listbox">
        {#each TIMES as time, i (time)}
          <button type="button" class="opt tabular" class:on={time === value} data-i={i} role="option" aria-selected={time === value} onclick={() => pick(time)}>
            {time}
          </button>
        {/each}
      </div>
    {/if}
  </Popover>
{/if}

<style>
  .time-field {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 40px;
    padding: 0 10px 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
  }
  .time-field :global(svg) {
    color: var(--muted);
    flex: none;
  }
  .time-field:hover {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  }
  .time-field.open {
    border-color: var(--primary);
    background: var(--surface);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 18%, transparent);
  }
  .typed {
    height: 36px;
    margin-bottom: 8px;
    text-align: center;
    font-weight: 600;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 220px;
    overflow-y: auto;
  }
  .ampm {
    display: flex;
    margin-bottom: 8px;
  }
  .ampm button {
    flex: 1;
    justify-content: center;
  }
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .col {
    max-height: 200px;
  }
  .done {
    width: 100%;
    margin-top: 8px;
    justify-content: center;
  }
  .opt {
    height: 32px;
    flex: none;
    border: none;
    border-radius: 9px;
    background: none;
    font-weight: 550;
    cursor: pointer;
  }
  .opt:hover {
    background: var(--surface-2);
  }
  .opt.on {
    background: var(--primary);
    color: var(--primary-text);
  }
</style>
