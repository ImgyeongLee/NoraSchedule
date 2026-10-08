<script lang="ts">
  // Time picker: a button showing the time, opening a list of times (every 15 minutes)
  // plus a box to type any exact time, e.g. "9:05", "0905" or "21:30".
  import { Clock, ChevronDown } from '@lucide/svelte';
  import { tick } from 'svelte';
  import Popover from './Popover.svelte';
  import { pad } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';

  let { value = $bindable(), label, onchange }: { value: string; label?: string; onchange?: (value: string) => void } = $props();

  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLDivElement>();
  let open = $state(false);
  let typed = $state('');

  const TIMES = Array.from({ length: 96 }, (_, i) => `${pad(Math.floor(i / 4))}:${pad((i % 4) * 15)}`);

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
  <Popover anchor={button} onclose={() => (open = false)} width={180}>
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
    <div class="list" bind:this={list} role="listbox">
      {#each TIMES as time, i (time)}
        <button type="button" class="opt tabular" class:on={time === value} data-i={i} role="option" aria-selected={time === value} onclick={() => pick(time)}>
          {time}
        </button>
      {/each}
    </div>
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
