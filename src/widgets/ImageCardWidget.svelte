<script lang="ts">
  // A purely decorative Overview card showing one of the user's pictures.
  import { ImagePlus, Pencil } from '@lucide/svelte';
  import ImageFrame from '../components/ImageFrame.svelte';
  import ImageFramer from '../components/ImageFramer.svelte';
  import { cleanFraming } from '../lib/framing';
  import { home, setTileImage } from '../lib/home.svelte';
  import { t } from '../lib/i18n.svelte';

  let { uid }: { uid?: string } = $props();
  const tile = $derived(home.tiles.find((x) => x.uid === uid));
  const framing = $derived(cleanFraming(tile?.framing));
  let w = $state(300);
  let h = $state(200);
  let editing = $state(false);
</script>

<div class="image-card" bind:clientWidth={w} bind:clientHeight={h}>
  {#if tile?.image}
    <ImageFrame image={tile.image} {framing} />
    <button class="edit" onclick={() => (editing = true)} title={t('w.image.edit')} aria-label={t('w.image.edit')}><Pencil size={14} /></button>
  {:else}
    <button class="empty" onclick={() => (editing = true)}>
      <ImagePlus size={24} />
      <span>{t('w.image.add')}</span>
    </button>
  {/if}
</div>

{#if editing && uid}
  <ImageFramer
    title={t('w.image')}
    image={tile?.image ?? null}
    {framing}
    width={w}
    height={h}
    longSide={1600}
    onsave={(image, f) => setTileImage(uid, image, f)}
    onremove={() => setTileImage(uid, null, framing)}
    onclose={() => (editing = false)}
  />
{/if}

<style>
  .image-card {
    position: relative;
    height: 100%;
    min-height: 120px;
    border-radius: inherit;
    overflow: hidden;
  }
  .edit {
    position: absolute;
    top: 10px;
    right: 10px;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 50%;
    background: color-mix(in srgb, var(--surface) 85%, transparent);
    color: var(--text);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .image-card:hover .edit,
  .edit:focus-visible {
    opacity: 1;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    height: 100%;
    border: 2px dashed var(--border);
    border-radius: inherit;
    background: transparent;
    color: var(--muted);
    font-weight: 600;
    cursor: pointer;
  }
  .empty:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
</style>
