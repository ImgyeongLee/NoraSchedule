<script lang="ts">
  import { scale } from 'svelte/transition';
  import { closeMenu, menu } from '../lib/menu.svelte';

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  // Keep the menu inside the window.
  $effect(() => {
    if (!menu.open || !el) return;
    const { width, height } = el.getBoundingClientRect();
    pos = {
      x: Math.max(8, Math.min(menu.x, window.innerWidth - width - 8)),
      y: Math.max(8, Math.min(menu.y, window.innerHeight - height - 8)),
    };
  });

  function onWindowDown(e: MouseEvent) {
    if (menu.open && el && !el.contains(e.target as Node)) closeMenu();
  }
</script>

<svelte:window
  onmousedown={onWindowDown}
  onkeydown={(e) => e.key === 'Escape' && closeMenu()}
  onblur={closeMenu}
  onresize={closeMenu}
  onwheel={closeMenu}
/>

{#if menu.open}
  <div
    class="menu"
    bind:this={el}
    style:left="{pos.x || menu.x}px"
    style:top="{pos.y || menu.y}px"
    role="menu"
    tabindex="-1"
    transition:scale={{ start: 0.96, duration: 110 }}
  >
    {#each menu.items as item, i (i)}
      {#if item === 'separator'}
        <div class="sep"></div>
      {:else}
        <button
          class="item"
          class:danger={item.danger}
          disabled={item.disabled}
          role="menuitem"
          onclick={() => {
            closeMenu();
            item.action();
          }}
        >
          <span class="icon">{#if item.icon}<item.icon size={15} />{/if}</span>
          <span class="label">{item.label}</span>
          {#if item.shortcut}<span class="shortcut">{item.shortcut}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 200;
    min-width: 200px;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    box-shadow: var(--shadow-lg);
    transform-origin: top left;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: none;
    border-radius: 9px;
    background: none;
    font-size: 13.5px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
  }
  .item:hover:not(:disabled) {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .item.danger {
    color: var(--danger);
  }
  .item.danger:hover:not(:disabled) {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .icon {
    width: 16px;
    display: grid;
    place-items: center;
    color: inherit;
  }
  .label {
    flex: 1;
  }
  .shortcut {
    font-size: 12px;
    color: var(--faint);
  }
  .sep {
    height: 1px;
    margin: 5px 6px;
    background: var(--border);
  }
</style>
