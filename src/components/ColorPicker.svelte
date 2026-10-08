<script lang="ts">
  import { Check, Plus } from '@lucide/svelte';
  import { PALETTE, fromHex, hex } from '../lib/colors';
  import { t } from '../lib/i18n.svelte';

  let { value = $bindable() }: { value: number } = $props();
  const custom = $derived(!PALETTE.some((p) => p.value === value));
</script>

<div class="swatches">
  {#each PALETTE as p (p.value)}
    <button
      type="button"
      class="swatch"
      class:selected={value === p.value}
      style:background={hex(p.value)}
      title={p.name}
      aria-label={p.name}
      onclick={() => (value = p.value)}
    >
      {#if value === p.value}<Check size={14} strokeWidth={3} />{/if}
    </button>
  {/each}
  <label class="swatch custom" class:selected={custom} style:background={custom ? hex(value) : undefined} title={t('common.customColor')}>
    {#if custom}<Check size={14} strokeWidth={3} />{:else}<Plus size={14} />{/if}
    <input type="color" value={hex(value)} oninput={(e) => (value = fromHex(e.currentTarget.value))} />
  </label>
</div>

<style>
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: none;
    display: grid;
    place-items: center;
    color: #fff;
    cursor: pointer;
    transition: transform 0.12s, box-shadow 0.12s;
    position: relative;
  }
  .swatch:hover {
    transform: scale(1.1);
  }
  .swatch.selected {
    box-shadow: 0 0 0 3px var(--surface), 0 0 0 5px currentColor;
    color: #fff;
  }
  .custom {
    background: var(--surface-2);
    color: var(--muted);
    border: 1.5px dashed var(--faint);
  }
  .custom.selected {
    border: none;
    color: #fff;
  }
  .custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
</style>
