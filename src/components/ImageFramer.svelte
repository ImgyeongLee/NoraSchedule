<script lang="ts">
  // Pick an image and frame it for a fixed box: drag to move it, wheel/slider to zoom.
  // The preview has the same shape as the real box, so what you see is what you get.
  import type { Snippet } from 'svelte';
  import { ImagePlus, LoaderCircle, Move, RefreshCw, Trash, ZoomIn } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import ImageFrame from './ImageFrame.svelte';
  import { api, type ImagePurpose } from '../lib/api';
  import { cleanFraming, DEFAULT_FRAMING, MAX_ZOOM, recommendFor, type Framing } from '../lib/framing';
  import { importImageFile } from '../lib/images';
  import { t } from '../lib/i18n.svelte';

  let {
    title,
    image: initialImage,
    framing: initialFraming,
    width,
    height,
    longSide,
    purpose = 'cover',
    extra,
    onsave,
    onremove,
    onclose,
  }: {
    title: string;
    image: string | null;
    framing: Framing;
    /** Size of the real box in pixels; sets the preview's shape. */
    width: number;
    height: number;
    /** Long side of the recommended upload, in pixels. */
    longSide: number;
    purpose?: ImagePurpose;
    /** Extra controls under the preview (e.g. the header height). */
    extra?: Snippet;
    onsave: (image: string, framing: Framing) => void;
    onremove?: () => void;
    onclose: () => void;
  } = $props();

  const startImage = () => initialImage;
  const startFraming = () => cleanFraming(initialFraming);
  let image = $state<string | null>(startImage());
  let framing = $state<Framing>(startFraming());
  let busy = $state(false);
  let error = $state('');
  let input: HTMLInputElement;
  let saved = false;

  const ratio = $derived(Math.max(0.2, Math.min(8, width / Math.max(1, height))));
  const tip = $derived(recommendFor(width, height, longSide));

  async function use(file: File | undefined) {
    if (!file) return;
    error = '';
    busy = true;
    try {
      image = await importImageFile(file, purpose);
      framing = { ...DEFAULT_FRAMING };
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      input.value = '';
    }
  }

  function save() {
    if (!image) return;
    saved = true;
    onsave(image, cleanFraming(framing));
    onclose();
  }

  function close() {
    // Drop an upload that was never saved.
    if (!saved) setTimeout(() => api.removeUnusedImages().catch(() => {}), 500);
    onclose();
  }
</script>

<input bind:this={input} type="file" accept="image/*" hidden onchange={(e) => use(e.currentTarget.files?.[0])} />

<Modal {title} onclose={close} width={640}>
  <p class="muted small tip">{t('frame.recommend', { ratio: tip.ratio, size: tip.size })}</p>

  <div class="stage" style:aspect-ratio={ratio} style:width="min(100%, {Math.round(340 * ratio)}px)">
    {#if busy}
      <div class="placeholder"><span class="spin"><LoaderCircle size={24} /></span>{t('dd.optimizing')}</div>
    {:else if image}
      <ImageFrame {image} bind:framing editable />
    {:else}
      <button
        type="button"
        class="placeholder drop"
        onclick={() => input.click()}
        ondragover={(e) => e.preventDefault()}
        ondrop={(e) => { e.preventDefault(); use(e.dataTransfer?.files[0]); }}
      >
        <ImagePlus size={26} />
        <span class="strong">{t('frame.pick')}</span>
        <span class="muted small">{t('frame.pickHint')}</span>
      </button>
    {/if}
  </div>

  {#if image && !busy}
    <p class="faint small hint"><Move size={13} /> {t('frame.dragHint')}</p>
    <div class="row zoom">
      <ZoomIn size={16} />
      <input
        type="range"
        min="1"
        max={MAX_ZOOM}
        step="0.01"
        value={framing.zoom}
        oninput={(e) => (framing = { ...framing, zoom: Number(e.currentTarget.value) })}
        aria-label={t('frame.zoom')}
      />
      <span class="tabular muted">{Math.round(framing.zoom * 100)}%</span>
      <button class="btn small ghost" onclick={() => (framing = { ...DEFAULT_FRAMING })}>{t('frame.reset')}</button>
    </div>
  {/if}

  {@render extra?.()}

  {#if error}<p class="error">{error}</p>{/if}

  {#snippet footer()}
    {#if image}
      <button class="btn" onclick={() => input.click()} disabled={busy}><RefreshCw size={15} /> {t('frame.change')}</button>
    {/if}
    {#if onremove && initialImage}
      <button class="btn danger" onclick={() => { saved = true; onremove?.(); onclose(); }}><Trash size={15} /> {t('frame.remove')}</button>
    {/if}
    <span class="spacer"></span>
    <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={save} disabled={!image || busy}>{t('common.save')}</button>
  {/snippet}
</Modal>

<style>
  .tip {
    margin-top: -4px;
  }
  .stage {
    width: 100%;
    margin: 0 auto;
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    height: 100%;
    color: var(--muted);
  }
  .drop {
    border: 2px dashed var(--border);
    border-radius: var(--radius);
    background: transparent;
    cursor: pointer;
  }
  .drop:hover {
    border-color: var(--primary);
    color: var(--primary);
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
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .zoom {
    gap: 10px;
    color: var(--muted);
  }
  .zoom input {
    flex: 1;
    accent-color: var(--primary);
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
</style>
