<script lang="ts">
  import type { Component } from 'svelte';

  let {
    label,
    value,
    sub = '',
    icon: Icon,
    tint = 'var(--primary)',
  }: { label: string; value: string; sub?: string; icon?: Component<{ size?: number }>; tint?: string } = $props();
</script>

<div class="stat">
  {#if Icon}
    <div class="icon" style:color={tint} style:background="color-mix(in srgb, {tint} 14%, transparent)">
      <Icon size={18} />
    </div>
  {/if}
  <div class="label">{label}</div>
  <div class="value truncate">{value}</div>
  <div class="sub truncate">{sub || ' '}</div>
</div>

<style>
  .stat {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 18px;
    box-shadow: var(--shadow-sm);
    min-width: 0;
  }
  .icon {
    width: 36px;
    height: 36px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    margin-bottom: 12px;
  }
  .label {
    font-size: 12.5px;
    color: var(--muted);
    font-weight: 600;
  }
  .value {
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.02em;
    margin-top: 2px;
  }
  .sub {
    font-size: 12px;
    color: var(--faint);
    min-height: 17px;
  }
</style>
