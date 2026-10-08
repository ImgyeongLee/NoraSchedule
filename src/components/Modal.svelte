<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { X } from '@lucide/svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    title,
    onclose,
    children,
    footer,
    width = 520,
  }: { title: string; onclose: () => void; children: Snippet; footer?: Snippet; width?: number } = $props();

  let overlay: HTMLDivElement;

  /** Only the topmost dialog reacts to Escape (dialogs can stack). */
  function isTopmost() {
    const all = document.querySelectorAll('.overlay');
    return all[all.length - 1] === overlay;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isTopmost()) onclose();
  }
</script>

<svelte:window {onkeydown} />

<div
  bind:this={overlay}
  class="overlay"
  transition:fade={{ duration: 140 }}
  onmousedown={(e) => e.target === e.currentTarget && onclose()}
  role="presentation"
>
  <div class="modal" style:width="{width}px" transition:scale={{ start: 0.96, duration: 160 }} role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h2>{title}</h2>
      <button class="icon-btn" onclick={onclose} aria-label={t('common.close')}><X size={18} /></button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--overlay);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
  }
  .modal {
    max-width: 100%;
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 26px;
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 20px 4px 24px;
  }
  .body {
    padding: 14px 24px 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  /* Long forms scroll instead of squashing their fields. */
  .body > :global(*) {
    flex-shrink: 0;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
</style>
