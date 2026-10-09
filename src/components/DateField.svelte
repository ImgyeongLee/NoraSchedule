<script lang="ts">
  // Date picker: a button showing the date in words, opening a calendar with shortcuts.
  // Replaces the browser's own date input, which is small and fiddly in WebKit.
  import { CalendarDays, ChevronDown, X } from '@lucide/svelte';
  import MiniCalendar from './MiniCalendar.svelte';
  import Popover from './Popover.svelte';
  import { addDays, fmt, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';

  let {
    value = $bindable(),
    min,
    clearable = false,
    placeholder,
    id,
    label,
    onchange,
  }: {
    /** `YYYY-MM-DD`, or empty/null for no date. */
    value: string | null;
    min?: string;
    clearable?: boolean;
    placeholder?: string;
    id?: string;
    /** Accessible name when there is no visible <label for={id}>. */
    label?: string;
    onchange?: (value: string | null) => void;
  } = $props();

  let button = $state<HTMLButtonElement>();
  let open = $state(false);

  function pick(day: string | null) {
    if (day && min && day < min) day = min;
    value = day;
    open = false;
    onchange?.(day);
  }

  const shortcuts = $derived([
    { label: t('dl.today'), date: today() },
    { label: t('dl.tomorrow'), date: addDays(today(), 1) },
    { label: t('date.nextWeek'), date: addDays(today(), 7) },
  ]);
</script>

<button
  bind:this={button}
  {id}
  type="button"
  class="date-field"
  class:open
  class:no-value={!value}
  aria-label={label}
  aria-haspopup="dialog"
  onclick={() => (open = !open)}
>
  <CalendarDays size={16} />
  <span class="text truncate">{value ? fmt(value, { weekday: 'short', year: 'numeric', month: 'short', day: 'numeric' }) : (placeholder ?? t('dl.pickDate'))}</span>
  <ChevronDown size={14} />
</button>

{#if open && button}
  <Popover anchor={button} onclose={() => (open = false)} width={288}>
    <MiniCalendar selected={value || today()} onpick={(d) => pick(d)} {min} />
    <div class="shortcuts">
      {#each shortcuts as s (s.label)}
        <button type="button" class="chip-btn" class:on={value === s.date} disabled={!!min && s.date < min} onclick={() => pick(s.date)}>
          {s.label}
        </button>
      {/each}
      {#if clearable && value}
        <button type="button" class="chip-btn clear" onclick={() => pick(null)}><X size={12} /> {t('common.clear')}</button>
      {/if}
    </div>
  </Popover>
{/if}

<style>
  .date-field {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-weight: 550;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
  }
  .date-field :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .date-field:hover {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  }
  .date-field.open {
    border-color: var(--primary);
    background: var(--surface);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 18%, transparent);
  }
  .date-field.no-value .text {
    color: var(--faint);
  }
  .text {
    flex: 1;
  }
  .shortcuts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .chip-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .chip-btn:hover:not(:disabled) {
    border-color: var(--primary);
    color: var(--primary);
  }
  .chip-btn.on {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-text);
  }
  .chip-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .chip-btn.clear {
    margin-left: auto;
  }
</style>
