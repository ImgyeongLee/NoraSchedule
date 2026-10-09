<script lang="ts">
  // Small dots for an event's tags, drawn inside a solid event block (see .event-block).
  import type { CalEvent } from '../lib/api';
  import { hex } from '../lib/colors';
  import { tagsOf } from '../lib/tags.svelte';

  let { event, max = 3 }: { event: CalEvent; max?: number } = $props();
  const tags = $derived(tagsOf(event).slice(0, max));
</script>

{#if tags.length}
  <span class="tag-dots">
    {#each tags as tag (tag.id)}<span class="dot" style:background={hex(tag.color)} title={tag.name}></span>{/each}
  </span>
{/if}

<style>
  .tag-dots {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex: none;
  }
  /* A ring in the block's text color keeps a dot visible on a block of the same color. */
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    box-shadow: 0 0 0 1.5px color-mix(in srgb, var(--on-c, #fff) 85%, transparent);
  }
</style>
