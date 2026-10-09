<script lang="ts">
  // Five stars in half steps. `value` is 0–10 (half stars). Without `onchange` it only displays.
  import { Star } from '@lucide/svelte';
  import { t } from '../lib/i18n.svelte';

  let { value, onchange, size = 16 }: { value: number; onchange?: (value: number) => void; size?: number } = $props();

  let hover = $state<number | null>(null);
  const shown = $derived(hover ?? value);

  /** Which half star the pointer is over (1–10). */
  function at(e: PointerEvent | MouseEvent): number {
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    return Math.min(10, Math.max(1, Math.ceil(((e.clientX - box.left) / box.width) * 10)));
  }

  function key(e: KeyboardEvent) {
    if (!onchange) return;
    const step = e.key === 'ArrowRight' || e.key === 'ArrowUp' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowDown' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    onchange(Math.min(10, Math.max(0, value + step)));
  }
</script>

{#if onchange}
  <span
    class="stars interactive"
    style:--size="{size}px"
    role="slider"
    tabindex="0"
    aria-label={t('read.rating')}
    aria-valuemin={0}
    aria-valuemax={5}
    aria-valuenow={value / 2}
    onpointermove={(e) => (hover = at(e))}
    onpointerleave={() => (hover = null)}
    onclick={(e) => {
      const v = at(e);
      // Clicking the current rating again clears it.
      onchange(v === value ? 0 : v);
    }}
    onkeydown={key}
  >
    {@render layers()}
  </span>
{:else}
  <span class="stars" style:--size="{size}px" title={value ? t('read.ratingOf', { n: value / 2 }) : t('read.noRating')}>
    {@render layers()}
  </span>
{/if}

{#snippet layers()}
  <span class="base">{#each Array(5) as _, i (i)}<Star {size} />{/each}</span>
  <span class="fill" style:width="{shown * 10}%">{#each Array(5) as _, i (i)}<Star {size} fill="currentColor" />{/each}</span>
{/snippet}

<style>
  .stars {
    position: relative;
    display: inline-flex;
    flex: none;
    line-height: 0;
    color: var(--warning);
  }
  .interactive {
    cursor: pointer;
    border-radius: 6px;
  }
  .base {
    display: inline-flex;
    color: color-mix(in srgb, var(--faint) 70%, transparent);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    display: inline-flex;
    overflow: hidden;
    white-space: nowrap;
  }
  .base :global(svg),
  .fill :global(svg) {
    flex: none;
  }
</style>
