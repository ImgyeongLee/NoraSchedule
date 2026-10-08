<script lang="ts" module>
  export interface SelectOption<V> {
    value: V;
    label: string;
    /** Color dot before the label (groups, folders). */
    color?: string;
    /** Indent level for nested items (folders). */
    depth?: number;
  }
</script>

<script lang="ts" generics="T">
  // Dropdown that replaces the browser's own <select>, which looks out of place in WebKit.
  // A button shows the current choice; the list opens in a popover (see Popover.svelte).
  // Keyboard: ↑/↓ or Enter/Space open it, ↑/↓ move, Enter picks, Escape closes.
  import type { Component } from 'svelte';
  import { tick } from 'svelte';
  import { Check, ChevronDown } from '@lucide/svelte';
  import Popover from './Popover.svelte';

  let {
    value = $bindable(),
    options,
    onchange,
    id,
    label,
    icon: Icon,
    placeholder = '',
  }: {
    value: T;
    options: SelectOption<T>[];
    onchange?: (value: T) => void;
    id?: string;
    /** Accessible name when there is no visible <label for={id}>. */
    label?: string;
    icon?: Component<{ size?: number }>;
    placeholder?: string;
  } = $props();

  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLDivElement>();
  let open = $state(false);
  let active = $state(0);
  const current = $derived(options.find((o) => o.value === value));

  async function show() {
    open = true;
    active = Math.max(0, options.findIndex((o) => o.value === value));
    await tick();
    list?.querySelector<HTMLElement>(`[data-i="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  function pick(o: SelectOption<T>) {
    open = false;
    button?.focus();
    if (o.value === value) return;
    value = o.value;
    onchange?.(o.value);
  }

  async function onkeydown(e: KeyboardEvent) {
    if (!open) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
        e.preventDefault();
        show();
      }
      return;
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      active = (active + (e.key === 'ArrowDown' ? 1 : -1) + options.length) % options.length;
      await tick();
      list?.querySelector<HTMLElement>(`[data-i="${active}"]`)?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (options[active]) pick(options[active]);
    } else if (e.key === 'Tab') {
      open = false;
    }
  }
</script>

<button
  bind:this={button}
  {id}
  type="button"
  class="select-btn"
  class:open
  aria-label={label}
  aria-haspopup="listbox"
  aria-expanded={open}
  onclick={() => (open ? (open = false) : show())}
  {onkeydown}
>
  {#if Icon}<span class="lead"><Icon size={16} /></span>{/if}
  {#if current?.color}<span class="dot" style:background={current.color}></span>{/if}
  <span class="text truncate" class:placeholder={!current}>{current?.label ?? placeholder}</span>
  <ChevronDown size={15} />
</button>

{#if open && button}
  <Popover anchor={button} onclose={() => (open = false)} width={Math.max(button.offsetWidth, 190)}>
    <div class="options" bind:this={list} role="listbox" aria-label={label}>
      {#each options as o, i (i)}
        <button
          type="button"
          class="option"
          class:active={i === active}
          class:selected={o.value === value}
          data-i={i}
          role="option"
          aria-selected={o.value === value}
          style:padding-left="{10 + (o.depth ?? 0) * 16}px"
          onmouseenter={() => (active = i)}
          onclick={() => pick(o)}
        >
          {#if o.color}<span class="dot" style:background={o.color}></span>{/if}
          <span class="truncate">{o.label}</span>
          {#if o.value === value}<span class="tick"><Check size={15} strokeWidth={2.5} /></span>{/if}
        </button>
      {/each}
    </div>
  </Popover>
{/if}

<style>
  .select-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    height: 40px;
    padding: 0 10px 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-weight: 550;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
  }
  .select-btn > :global(svg:last-child) {
    flex: none;
    color: var(--muted);
    transition: transform 0.15s;
  }
  .select-btn.open > :global(svg:last-child) {
    transform: rotate(180deg);
  }
  .select-btn:hover {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  }
  .select-btn:focus-visible,
  .select-btn.open {
    outline: none;
    border-color: var(--primary);
    background: var(--surface);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 18%, transparent);
  }
  .lead {
    display: grid;
    flex: none;
    color: var(--muted);
  }
  .text {
    flex: 1;
  }
  .placeholder {
    color: var(--faint);
  }
  .dot {
    width: 9px;
    height: 9px;
    flex: none;
    border-radius: 50%;
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 280px;
    overflow-y: auto;
    margin: -4px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 36px;
    flex: none;
    padding-right: 10px;
    border: none;
    border-radius: 10px;
    background: none;
    color: var(--text);
    font-size: 13.5px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
  }
  .option.active {
    background: var(--surface-2);
  }
  .option.selected {
    color: var(--primary);
    font-weight: 650;
  }
  .tick {
    display: grid;
    margin-left: auto;
    color: var(--primary);
  }
</style>
