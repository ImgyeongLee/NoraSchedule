<script lang="ts">
  // Cover photo picker: click or drop an image. The file is optimized by Rust on import.
  import { ImagePlus, LoaderCircle, RefreshCw, Trash } from '@lucide/svelte';
  import { imageUrl, importImageFile } from '../lib/images';
  import { t } from '../lib/i18n.svelte';

  let { value = $bindable() }: { value: string | null } = $props();

  let input: HTMLInputElement;
  let busy = $state(false);
  let error = $state('');
  let dragOver = $state(false);

  async function use(file: File | undefined) {
    if (!file) return;
    error = '';
    busy = true;
    try {
      value = await importImageFile(file);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      input.value = '';
    }
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    use(e.dataTransfer?.files[0]);
  }
</script>

<input bind:this={input} type="file" accept="image/*" hidden onchange={(e) => use(e.currentTarget.files?.[0])} />

{#if value && !busy}
  <div class="preview" style:background-image="url('{imageUrl(value)}')">
    <div class="actions">
      <button type="button" class="btn small" onclick={() => input.click()}><RefreshCw size={14} /> {t('dd.changePhoto')}</button>
      <button type="button" class="btn small" onclick={() => (value = null)}><Trash size={14} /> {t('dd.removePhoto')}</button>
    </div>
  </div>
{:else}
  <button
    type="button"
    class="drop"
    class:over={dragOver}
    disabled={busy}
    onclick={() => input.click()}
    ondragover={(e) => { e.preventDefault(); dragOver = true; }}
    ondragleave={() => (dragOver = false)}
    ondrop={drop}
  >
    {#if busy}
      <span class="spin"><LoaderCircle size={22} /></span>
      <span>{t('dd.optimizing')}</span>
    {:else}
      <ImagePlus size={22} />
      <span class="strong">{t('dd.addPhoto')}</span>
      <span class="faint small">{t('dd.addPhotoHint')}</span>
    {/if}
  </button>
{/if}
{#if error}<p class="error">{error}</p>{/if}

<style>
  .drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    width: 100%;
    min-height: 120px;
    padding: 16px;
    border: 2px dashed var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    color: var(--muted);
    text-align: center;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }
  .drop:hover,
  .drop.over {
    border-color: var(--primary);
    color: var(--primary);
    background: var(--primary-soft);
  }
  .strong {
    font-weight: 650;
  }
  .preview {
    position: relative;
    height: 150px;
    border-radius: var(--radius);
    background-size: cover;
    background-position: center;
    overflow: hidden;
  }
  .actions {
    position: absolute;
    right: 10px;
    bottom: 10px;
    display: flex;
    gap: 6px;
  }
  .actions .btn {
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
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
  .error {
    color: var(--danger);
    font-weight: 600;
    font-size: 13px;
  }
</style>
