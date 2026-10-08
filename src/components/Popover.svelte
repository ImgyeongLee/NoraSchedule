<script lang="ts">
  // A small floating panel anchored to a button. It lives in <body> (see lib/portal.ts),
  // so it is never clipped by dialogs, tiles or scroll areas, and flips upward near the
  // bottom of the window.
  import { onMount, type Snippet } from 'svelte';
  import { scale } from 'svelte/transition';
  import { portal } from '../lib/portal';

  let { anchor, onclose, children, width = 280 }: { anchor: HTMLElement; onclose: () => void; children: Snippet; width?: number } =
    $props();

  let panel = $state<HTMLDivElement>();
  let pos = $state({ left: 0, top: 0, up: false });

  function place() {
    if (!anchor || !panel) return;
    const r = anchor.getBoundingClientRect();
    const h = panel.offsetHeight;
    const below = window.innerHeight - r.bottom;
    const up = below < h + 12 && r.top > below;
    pos = {
      left: Math.max(8, Math.min(r.left, window.innerWidth - width - 8)),
      top: up ? Math.max(8, r.top - h - 6) : r.bottom + 6,
      up,
    };
  }

  onMount(() => {
    place();
    requestAnimationFrame(place);
  });

  function onWindowDown(e: MouseEvent) {
    const target = e.target as Node;
    if (panel && !panel.contains(target) && !anchor.contains(target)) onclose();
  }

  // Capture phase: Escape closes only this panel, not the dialog underneath.
  function onKeydownCapture(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onmousedown={onWindowDown} onkeydowncapture={onKeydownCapture} onresize={place} onscrollcapture={place} />

<div class="popover-anchor">
  <div
    use:portal
    bind:this={panel}
    class="popover"
    class:up={pos.up}
    style:left="{pos.left}px"
    style:top="{pos.top}px"
    style:width="{width}px"
    in:scale={{ start: 0.96, duration: 110 }}
  >
    {@render children()}
  </div>
</div>

<style>
  .popover-anchor {
    display: contents;
  }
  .popover {
    position: fixed;
    z-index: 300;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--surface);
    box-shadow: var(--shadow-lg);
    transform-origin: top left;
  }
  .popover.up {
    transform-origin: bottom left;
  }
</style>
