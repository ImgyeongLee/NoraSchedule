<script lang="ts">
  // The overlay panel window (label "panel", see src-tauri/src/panel.rs): a drag bar and
  // the Overview widgets chosen in Settings → Overlay panel. It reuses the same widgets,
  // dialogs and stores as the main window; links to other pages open the main window.
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { AppWindow, ChevronDown, ChevronUp, CircleCheck, Pin, PinOff } from '@lucide/svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import ScopeDialog from './components/ScopeDialog.svelte';
  import EventHoverCard from './components/EventHoverCard.svelte';
  import { WIDGET_COMPONENTS } from './widgets/registry';
  import { WIDGETS, type WidgetId } from './lib/home.svelte';
  import { data, initTheme, ui } from './lib/state.svelte';
  import { initLocale, t } from './lib/i18n.svelte';
  import { refreshTags } from './lib/tags.svelte';
  import { loadCalPrefs } from './lib/calPrefs.svelte';
  import { initSync } from './lib/sync.svelte';
  import { loadPanelWidgets, panelApi } from './lib/panel';

  let widgets = $state<WidgetId[]>([]);
  let onTop = $state(true);
  let collapsed = $state(false);

  async function reloadPrefs() {
    await Promise.all([initTheme(), initLocale(), loadCalPrefs()]);
    widgets = await loadPanelWidgets();
    const status = await panelApi.status().catch(() => null);
    onTop = status?.on_top ?? onTop;
    collapsed = status?.collapsed ?? collapsed;
  }

  onMount(() => {
    document.documentElement.classList.add('panel-window');
    reloadPrefs();
    refreshTags();
    initSync(data, refreshTags, reloadPrefs);
  });

  // Widgets "navigate" by setting ui.page (and the day or memo to open). There are no
  // pages in the panel, so hand the request to the main window and stay put.
  $effect(() => {
    const page = ui.page;
    if (page === 'home') return;
    panelApi.showMain(page, ui.calendarFocus, ui.memoFocus).catch(() => {});
    ui.page = 'home';
    ui.calendarFocus = null;
    ui.memoFocus = null;
  });

  async function toggleCollapsed() {
    collapsed = !collapsed;
    await panelApi.setCollapsed(collapsed).catch(() => {});
  }

  async function toggleOnTop() {
    onTop = !onTop;
    await panelApi.setOnTop(onTop).catch(() => {});
  }

  /** Tall widgets (agenda, todos, calendar…) get twice the height of small ones. */
  const heightOf = (id: WidgetId) => (WIDGETS[id].defaultSize === 'tall' || WIDGETS[id].defaultSize === 'large' ? 360 : 184);
</script>

<div class="panel" class:collapsed>
  <header data-tauri-drag-region>
    <span class="logo" data-tauri-drag-region><CircleCheck size={15} strokeWidth={2.5} /></span>
    <span class="name" data-tauri-drag-region>Nora</span>
    <span class="spacer" data-tauri-drag-region></span>
    <button class="icon-btn" onclick={toggleOnTop} title={onTop ? t('panel.unpin') : t('panel.pin')} aria-pressed={onTop}>
      {#if onTop}<Pin size={15} />{:else}<PinOff size={15} />{/if}
    </button>
    <button class="icon-btn" onclick={toggleCollapsed} title={collapsed ? t('panel.expand') : t('panel.collapse')} aria-expanded={!collapsed}>
      {#if collapsed}<ChevronDown size={16} />{:else}<ChevronUp size={16} />{/if}
    </button>
    <button class="icon-btn" onclick={() => panelApi.showMain()} title={t('panel.openApp')}><AppWindow size={15} /></button>
  </header>

  <div class="body">
    {#each widgets as id (id)}
      {@const Widget = WIDGET_COMPONENTS[id]}
      <div class="tile" style:height="{heightOf(id)}px">
        <div class="content"><Widget ddayId={null} /></div>
      </div>
    {:else}
      <p class="muted small empty">{t('panel.empty')}</p>
    {/each}
  </div>

  <ContextMenu />
  <ScopeDialog />
  <EventHoverCard />

  {#each ui.toast ? [ui.toast] : [] as item (item.id)}
    <div class="toast {item.kind}" transition:fly={{ y: 16, duration: 200 }}>
      <span>{item.text}</span>
      {#if item.action}
        <button
          class="toast-action"
          onclick={() => {
            ui.toast = null;
            item.action?.run();
          }}>{item.action.label}</button
        >
      {/if}
    </div>
  {/each}
</div>

<style>
  :global(html.panel-window),
  :global(html.panel-window body) {
    overflow: hidden;
  }
  .panel {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    border: 1px solid var(--border);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    height: 40px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--border);
    background: var(--sidebar);
    cursor: grab;
    user-select: none;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 8px;
    background: linear-gradient(140deg, var(--primary), var(--accent-2));
    color: #fff;
  }
  .name {
    font-weight: 750;
  }
  .collapsed header {
    border-bottom: none;
  }
  .collapsed .body {
    display: none;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px;
  }
  .tile {
    flex: none;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    container-type: inline-size;
    container-name: tile;
  }
  .content {
    height: 100%;
  }
  .empty {
    padding: 16px 6px;
    text-align: center;
  }
  .toast-action {
    margin-left: 12px;
    padding: 4px 10px;
    border: none;
    border-radius: 8px;
    background: color-mix(in srgb, var(--bg) 22%, transparent);
    color: inherit;
    font-weight: 700;
    cursor: pointer;
  }
  .toast {
    display: flex;
    align-items: center;
    position: fixed;
    left: 12px;
    right: 12px;
    bottom: 12px;
    z-index: 100;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: var(--text);
    color: var(--bg);
    font-weight: 550;
    font-size: 13px;
    box-shadow: var(--shadow-lg);
  }
  .toast.error {
    background: var(--danger);
    color: #fff;
  }
  .toast.success {
    background: var(--success);
    color: #fff;
  }
</style>
