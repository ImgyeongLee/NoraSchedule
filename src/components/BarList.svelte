<script lang="ts">
  // Ranked horizontal bars.
  let {
    items,
    format = (v: number) => String(v),
  }: { items: { label: string; value: number; color: string; sub?: string }[]; format?: (v: number) => string } =
    $props();
  const max = $derived(Math.max(0, ...items.map((i) => i.value)));
</script>

<div class="list">
  {#each items as item, i (i)}
    <div class="row">
      <div class="name truncate" title={item.label}>
        <span class="dot" style:background={item.color}></span>
        <span class="truncate">{item.label}</span>
        {#if item.sub}<span class="faint small truncate">{item.sub}</span>{/if}
      </div>
      <div class="track">
        <div class="fill" style:width="{max > 0 ? (item.value / max) * 100 : 0}%" style:background={item.color}></div>
      </div>
      <div class="value tabular">{format(item.value)}</div>
    </div>
  {/each}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 2fr) 76px;
    align-items: center;
    gap: 12px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 550;
  }
  .track {
    height: 10px;
    border-radius: 10px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 10px;
    transition: width 0.4s ease;
  }
  .value {
    text-align: right;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 600;
  }
</style>
