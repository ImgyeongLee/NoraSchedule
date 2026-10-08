<script lang="ts">
  // Stickers for the current page, drawn over its content. Outside decorate mode the whole
  // layer ignores the mouse, so the page underneath works exactly as without stickers.
  import { fade, fly } from 'svelte/transition';
  import { BringToFront, Check, ImagePlus, LoaderCircle, MoveDiagonal2, RotateCw, Trash, X } from '@lucide/svelte';
  import { imageUrl } from '../lib/images';
  import { t } from '../lib/i18n.svelte';
  import type { Page } from '../lib/state.svelte';
  import { toast } from '../lib/state.svelte';
  import {
    bringToFront, deleteFromLibrary, placeSticker, removeSticker, setDecorating, stickers, updateSticker, uploadSticker,
    type PlacedSticker,
  } from '../lib/stickers.svelte';

  let { page }: { page: Page } = $props();

  let layer: HTMLDivElement;
  let input: HTMLInputElement;
  let uploading = $state(false);
  let confirmDelete = $state<string | null>(null);
  const onPage = $derived(stickers.placed.filter((s) => s.page === page));

  type Gesture = { kind: 'move' | 'resize' | 'rotate'; id: string; dx: number; dy: number };
  let gesture: Gesture | null = null;

  function center(s: PlacedSticker) {
    const r = layer.getBoundingClientRect();
    return { cx: r.left + s.x * r.width, cy: r.top + s.y * r.height, r };
  }

  function start(e: PointerEvent, s: PlacedSticker, kind: Gesture['kind']) {
    if (!stickers.editing || e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    stickers.selected = s.id;
    const { cx, cy } = center(s);
    gesture = { kind, id: s.id, dx: e.clientX - cx, dy: e.clientY - cy };
  }

  function move(e: PointerEvent, s: PlacedSticker) {
    if (!gesture || gesture.id !== s.id) return;
    const { cx, cy, r } = center(s);
    if (gesture.kind === 'move') {
      updateSticker(s.id, { x: (e.clientX - gesture.dx - r.left) / r.width, y: (e.clientY - gesture.dy - r.top) / r.height });
    } else if (gesture.kind === 'resize') {
      // The corner handle sits on the diagonal; its distance from the center sets the size.
      updateSticker(s.id, { w: Math.hypot(e.clientX - cx, e.clientY - cy) * Math.SQRT2 });
    } else {
      const deg = (Math.atan2(e.clientY - cy, e.clientX - cx) * 180) / Math.PI + 90;
      updateSticker(s.id, { rot: e.shiftKey ? Math.round(deg / 15) * 15 : Math.round(deg) });
    }
  }

  async function upload(file: File | undefined) {
    if (!file) return;
    uploading = true;
    try {
      placeSticker(await uploadSticker(file), page);
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'error');
    } finally {
      uploading = false;
      input.value = '';
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (!stickers.editing) return;
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    if (e.key === 'Escape') setDecorating(false);
    else if ((e.key === 'Delete' || e.key === 'Backspace') && stickers.selected && !typing) removeSticker(stickers.selected);
  }
</script>

<svelte:window
  {onkeydown}
  onpointerdown={(e) => {
    // Clicking anywhere but a sticker or the tray deselects.
    if (stickers.editing && !(e.target as Element).closest?.('.sticker, .tray')) stickers.selected = null;
  }}
/>
<input bind:this={input} type="file" accept="image/*" hidden onchange={(e) => upload(e.currentTarget.files?.[0])} />

<div class="sticker-layer" class:editing={stickers.editing} bind:this={layer} aria-hidden={!stickers.editing}>
  {#each onPage as s (s.id)}
    {@const selected = stickers.editing && stickers.selected === s.id}
    <div
      class="sticker"
      class:selected
      style:left="{s.x * 100}%"
      style:top="{s.y * 100}%"
      style:width="{s.w}px"
      style:transform="translate(-50%, -50%) rotate({s.rot}deg)"
      onpointerdown={(e) => start(e, s, 'move')}
      onpointermove={(e) => move(e, s)}
      onpointerup={() => (gesture = null)}
      onpointercancel={() => (gesture = null)}
      role="presentation"
    >
      <!-- Own URL (not shared with the tray thumbnails) and re-created when decorating ends,
           so animated GIFs keep playing after the sticker box closes. -->
      {#key stickers.editing}
        <img src={imageUrl(s.image, 'placed')} alt="" draggable="false" />
      {/key}
      {#if selected}
        <button class="handle del" onpointerdown={(e) => e.stopPropagation()} onclick={() => removeSticker(s.id)} title={t('sticker.remove')}><X size={13} /></button>
        <button class="handle front" onpointerdown={(e) => e.stopPropagation()} onclick={() => bringToFront(s.id)} title={t('sticker.front')}><BringToFront size={13} /></button>
        <span
          class="handle rot"
          onpointerdown={(e) => start(e, s, 'rotate')}
          onpointermove={(e) => move(e, s)}
          onpointerup={() => (gesture = null)}
          title={t('sticker.rotate')}
          role="presentation"
        ><RotateCw size={13} /></span>
        <span
          class="handle size"
          onpointerdown={(e) => start(e, s, 'resize')}
          onpointermove={(e) => move(e, s)}
          onpointerup={() => (gesture = null)}
          title={t('sticker.resize')}
          role="presentation"
        ><MoveDiagonal2 size={13} /></span>
      {/if}
    </div>
  {/each}
</div>

{#if stickers.editing}
  <!-- Clicking an empty spot deselects; the page itself stays usable while decorating. -->
  <div class="tray card" transition:fly={{ y: 20, duration: 180 }}>
    <div class="tray-head">
      <span class="strong">{t('sticker.title')}</span>
      <span class="muted small">{t('sticker.hint')}</span>
      <span class="spacer"></span>
      <button class="btn primary small" onclick={() => setDecorating(false)}><Check size={15} /> {t('home.done')}</button>
    </div>
    <div class="library">
      <button class="add" onclick={() => input.click()} disabled={uploading} title={t('sticker.upload')}>
        {#if uploading}<span class="spin"><LoaderCircle size={20} /></span>{:else}<ImagePlus size={20} />{/if}
      </button>
      {#each stickers.library as image (image)}
        <div class="thumb" transition:fade={{ duration: 120 }}>
          <button class="thumb-btn" onclick={() => placeSticker(image, page)} title={t('sticker.place')}>
            <img src={imageUrl(image)} alt="" draggable="false" />
          </button>
          {#if confirmDelete === image}
            <button class="thumb-del sure" onclick={() => { deleteFromLibrary(image); confirmDelete = null; }} title={t('sticker.deleteSure')}><Trash size={11} /></button>
          {:else}
            <button class="thumb-del" onclick={() => (confirmDelete = image)} title={t('sticker.delete')}><X size={11} /></button>
          {/if}
        </div>
      {:else}
        <span class="muted small empty">{t('sticker.empty')}</span>
      {/each}
    </div>
  </div>
{/if}

<style>
  .sticker-layer {
    position: absolute;
    inset: 0;
    z-index: 4;
    overflow: hidden;
    pointer-events: none;
  }
  .sticker {
    position: absolute;
    user-select: none;
  }
  .sticker img {
    display: block;
    width: 100%;
    height: auto;
    pointer-events: none;
    filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.12));
  }
  .editing .sticker {
    pointer-events: auto;
    cursor: move;
    touch-action: none;
  }
  .editing .sticker:hover {
    outline: 1.5px dashed color-mix(in srgb, var(--primary) 60%, transparent);
    outline-offset: 4px;
  }
  .sticker.selected {
    outline: 2px solid var(--primary);
    outline-offset: 4px;
  }
  .handle {
    position: absolute;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
    cursor: pointer;
  }
  .del {
    top: -16px;
    right: -16px;
    background: var(--danger);
    color: #fff;
  }
  .front {
    top: -16px;
    left: -16px;
  }
  .rot {
    top: -40px;
    left: 50%;
    margin-left: -12px;
    cursor: grab;
  }
  .size {
    right: -16px;
    bottom: -16px;
    cursor: nwse-resize;
  }
  .tray {
    position: absolute;
    left: 50%;
    bottom: 18px;
    z-index: 6;
    width: min(680px, calc(100% - 32px));
    transform: translateX(-50%);
    padding: 12px 14px;
    box-shadow: var(--shadow-lg);
  }
  .tray-head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
  }
  .library {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow-x: auto;
    padding: 6px 2px 4px;
  }
  .add,
  .thumb-btn {
    flex: none;
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 14px;
    cursor: pointer;
  }
  .add {
    border: 2px dashed var(--border);
    background: transparent;
    color: var(--muted);
  }
  .add:hover:not(:disabled) {
    border-color: var(--primary);
    color: var(--primary);
  }
  .thumb {
    position: relative;
    flex: none;
  }
  .thumb-btn {
    padding: 4px;
    border: 1px solid var(--border);
    background: var(--surface-2);
  }
  .thumb-btn:hover {
    border-color: var(--primary);
  }
  .thumb-btn img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .thumb-del {
    position: absolute;
    top: -6px;
    right: -6px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--surface-3);
    color: var(--text);
    cursor: pointer;
  }
  .thumb-del.sure {
    background: var(--danger);
    color: #fff;
  }
  .empty {
    padding-left: 4px;
  }
  .spin {
    display: grid;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
