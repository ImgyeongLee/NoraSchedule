<script lang="ts">
  // Responsive column chart drawn with plain SVG.
  let {
    data,
    color = 'var(--primary)',
    height = 200,
    format = (v: number) => String(v),
  }: { data: { label: string; value: number }[]; color?: string; height?: number; format?: (v: number) => string } =
    $props();

  let width = $state(600);
  const top = 22;
  const bottom = 24;
  const max = $derived(Math.max(0, ...data.map((d) => d.value)));
  const slot = $derived(width / Math.max(1, data.length));
  const barW = $derived(Math.min(36, slot * 0.6));
  const chartH = $derived(height - top - bottom);
  const labelEvery = $derived(data.length > 14 ? Math.ceil(data.length / 10) : 1);
  let hover = $state<number | null>(null);
</script>

<div class="chart" bind:clientWidth={width}>
  <svg {width} {height} role="img">
    {#each [0.25, 0.5, 0.75, 1] as f (f)}
      <line x1="0" x2={width} y1={top + chartH * (1 - f)} y2={top + chartH * (1 - f)} class="grid" />
    {/each}
    {#each data as d, i (i)}
      {@const h = max > 0 ? (d.value / max) * chartH : 0}
      {@const cx = slot * (i + 0.5)}
      <g
        onmouseenter={() => (hover = i)}
        onmouseleave={() => (hover = null)}
        role="presentation"
      >
        <rect x={cx - slot / 2} y={top} width={slot} height={chartH} fill="transparent" />
        {#if h > 0}
          <rect
            x={cx - barW / 2}
            y={top + chartH - h}
            width={barW}
            height={h}
            rx={Math.min(8, barW / 2)}
            fill={color}
            opacity={hover === null || hover === i ? 1 : 0.45}
          />
        {/if}
        {#if d.value > 0 && (data.length <= 14 || hover === i)}
          <text x={cx} y={top + chartH - h - 6} class="value">{format(d.value)}</text>
        {/if}
        {#if i % labelEvery === 0 || i === data.length - 1}
          <text x={cx} y={height - 6} class="label">{d.label}</text>
        {/if}
      </g>
    {/each}
  </svg>
</div>

<style>
  .chart {
    width: 100%;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--border);
    stroke-dasharray: 3 4;
  }
  rect {
    transition: opacity 0.15s;
  }
  text {
    text-anchor: middle;
    font-size: 11px;
    fill: var(--muted);
  }
  .value {
    font-weight: 600;
    fill: var(--text);
  }
</style>
