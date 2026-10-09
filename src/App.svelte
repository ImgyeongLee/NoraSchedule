<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { CircleCheck, PanelLeftClose, PanelLeftOpen, Pause, Play, Settings as SettingsIcon, Sticker } from '@lucide/svelte';
  import { api, type TrackerStatus } from './lib/api';
  import { pomodoro, PHASES } from './lib/pomodoro.svelte';
  import { initSidebar, initTheme, setSidebarCollapsed, toast, ui } from './lib/state.svelte';
  import { initLocale, t } from './lib/i18n.svelte';
  import { initPages, visiblePages } from './lib/pages.svelte';
  import { trackState } from './lib/tracker';
  import { clipboard, isMac, pageTarget, pointer, shortcut } from './lib/clipboard.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import { isSecondaryClick } from './lib/menu.svelte';
  import ScopeDialog from './components/ScopeDialog.svelte';
  import EventHoverCard from './components/EventHoverCard.svelte';
  import { refreshTags } from './lib/tags.svelte';
  import { initReminders } from './lib/reminders';
  import { initZoom, stepZoom } from './lib/window.svelte';
  import StickerLayer from './components/StickerLayer.svelte';
  import { loadStickers, setDecorating, stickers } from './lib/stickers.svelte';
  import Home from './pages/Home.svelte';
  import Calendar from './pages/Calendar.svelte';
  import DDays from './pages/DDays.svelte';
  import Todos from './pages/Todos.svelte';
  import Pomodoro from './pages/Pomodoro.svelte';
  import Memos from './pages/Memos.svelte';
  import Tracking from './pages/Tracking.svelte';
  import Analytics from './pages/Analytics.svelte';
  import Bookmarks from './pages/Bookmarks.svelte';
  import Expenses from './pages/Expenses.svelte';
  import Trpg from './pages/Trpg.svelte';
  import Reading from './pages/Reading.svelte';
  import Settings from './pages/Settings.svelte';

  /** Sidebar pages in the user's order, without the ones turned off in Settings. */
  const NAV = $derived(visiblePages());

  /** Below this window width the sidebar shows icons only, whatever the user chose. */
  const AUTO_COLLAPSE = 900;

  let tracker = $state<TrackerStatus | null>(null);
  let windowWidth = $state(1280);
  const narrow = $derived(windowWidth < AUTO_COLLAPSE);
  const collapsed = $derived(ui.sidebarCollapsed || narrow);

  onMount(() => {
    initTheme();
    initLocale();
    initSidebar();
    initPages();
    initZoom();
    pomodoro.load();
    refreshTags();
    loadStickers();
    initReminders();
    const poll = async () => (tracker = await api.trackerStatus().catch(() => null));
    poll();
    const timer = setInterval(poll, 2000);
    return () => clearInterval(timer);
  });

  // ---- copy & paste of events/todos -------------------------------------------
  const isTyping = (el: EventTarget | null) =>
    el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement ||
    (el instanceof HTMLElement && el.isContentEditable);
  const hasTextSelection = () => !!window.getSelection()?.toString();
  const modalOpen = () => !!document.querySelector('.overlay');
  // Hover targets belong to the page that set them.
  $effect(() => {
    ui.page;
    pointer.copy = null;
    pointer.paste = null;
  });

  /** Set when a shortcut was handled by keydown, so the matching copy/paste event is ignored. */
  let handledAt = 0;

  function itemCopy(target: EventTarget | null): boolean {
    if (isTyping(target) || hasTextSelection() || modalOpen() || !pointer.copy) return false;
    pointer.copy();
    return true;
  }

  function itemPaste(target: EventTarget | null): boolean {
    const paste = pointer.paste ?? pageTarget.paste;
    if (isTyping(target) || modalOpen() || !paste) return false;
    if (clipboard.item) paste();
    else toast(t('clip.nothing'));
    return true;
  }

  // On macOS ⌘C/⌘V may go through the Edit menu and arrive as copy/paste events instead.
  function oncopy(e: ClipboardEvent) {
    if (Date.now() - handledAt > 300 && itemCopy(e.target)) e.preventDefault();
  }
  function onpaste(e: ClipboardEvent) {
    if (Date.now() - handledAt > 300 && itemPaste(e.target)) e.preventDefault();
  }

  // Runs before any other click handler: a right-click (or macOS Control-click) must only
  // open the context menu, never also trigger the item's normal click action.
  function swallowSecondaryClick(e: MouseEvent) {
    if ((e.target as HTMLElement | null)?.closest?.('.menu')) return;
    if (isSecondaryClick(e)) {
      e.stopPropagation();
      e.preventDefault();
    }
  }

  // Our own right-click menus call preventDefault; elsewhere hide the browser menu,
  // except in text fields where Cut/Copy/Paste are useful.
  function oncontextmenu(e: MouseEvent) {
    if (!isTyping(e.target)) e.preventDefault();
  }

  function onkeydown(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    const key = e.key.toLowerCase();
    if (!e.shiftKey && !e.altKey && (key === 'c' || key === 'v')) {
      if (key === 'c' ? itemCopy(e.target) : itemPaste(e.target)) {
        e.preventDefault();
        handledAt = Date.now();
      }
      return;
    }
    if (e.key === ',') {
      e.preventDefault();
      ui.page = 'settings';
    } else if (e.key === '\\') {
      e.preventDefault();
      setSidebarCollapsed(!ui.sidebarCollapsed);
    } else if (e.key === '=' || e.key === '+' || e.key === '-') {
      // ⌘+ / ⌘- zoom the whole app (⌘0 is taken by the tenth page; reset in Settings).
      e.preventDefault();
      stepZoom(e.key === '-' ? -1 : 1);
    } else {
      // ⌘1 … ⌘9 open the first nine visible pages, ⌘0 the tenth.
      const n = e.key === '0' ? 10 : Number(e.key);
      if (n >= 1 && n <= NAV.length) {
        e.preventDefault();
        ui.page = NAV[n - 1].id;
      }
    }
  }

  /** Tooltip: the label when collapsed (so icons stay understandable), plus the shortcut. */
  const tip = (label: string, shortcut: string) => (collapsed ? `${label} (${shortcut})` : shortcut);
</script>

<svelte:window
  {onkeydown}
  {oncopy}
  {onpaste}
  {oncontextmenu}
  onclickcapture={swallowSecondaryClick}
  ondblclickcapture={swallowSecondaryClick}
  bind:innerWidth={windowWidth}
/>

<div class="app" class:collapsed class:mac={isMac}>
  <aside class="sidebar">
    <div class="drag" data-tauri-drag-region></div>
    <div class="brand">
      <div class="logo"><CircleCheck size={20} strokeWidth={2.5} /></div>
      {#if !collapsed}
        <div class="brand-text" transition:fade={{ duration: 120 }}>
          <div class="name">Nora</div>
          <div class="tag truncate">{t('app.tagline')}</div>
        </div>
      {/if}
    </div>

    <nav>
      {#each NAV as item, i (item.id)}
        <button
          class="nav-item"
          class:active={ui.page === item.id}
          onclick={() => (ui.page = item.id)}
          title={tip(t(item.label), i < 10 ? shortcut(String((i + 1) % 10)) : '')}
          aria-label={t(item.label)}
        >
          <item.icon size={19} />
          <span class="label-text">{t(item.label)}</span>
        </button>
      {/each}
    </nav>

    <div class="spacer"></div>

    {#if pomodoro.started}
      <button
        class="mini-timer"
        onclick={() => (ui.page = 'pomodoro')}
        transition:fade={{ duration: 150 }}
        title={collapsed ? `${PHASES[pomodoro.phase].name()} · ${pomodoro.text}` : undefined}
      >
        <span class="mini-dot" style:background={PHASES[pomodoro.phase].color}></span>
        <span class="mini-time tabular">{pomodoro.text}</span>
        {#if !collapsed}
          <span class="muted small truncate">{PHASES[pomodoro.phase].name()}</span>
          <span
            class="mini-toggle"
            role="button"
            tabindex="0"
            aria-label={pomodoro.running ? t('common.pause') : t('common.resume')}
            onclick={(e) => { e.stopPropagation(); pomodoro.toggle(); }}
            onkeydown={(e) => e.key === 'Enter' && pomodoro.toggle()}
          >
            {#if pomodoro.running}<Pause size={14} />{:else}<Play size={14} />{/if}
          </span>
        {/if}
      </button>
    {/if}

    {#if tracker}
      {@const s = trackState(tracker.state)}
      <button class="tracker-pill" onclick={() => (ui.page = 'tracking')} title={collapsed ? `${s.label} — ${s.hint}` : s.hint}>
        <span class="pulse" class:live={tracker.state === 'tracking'} style:background={s.color}></span>
        <span class="label-text truncate">{s.label}{tracker.state === 'tracking' && tracker.app ? ` · ${tracker.app}` : ''}</span>
      </button>
    {/if}

    <button
      class="nav-item"
      class:active={stickers.editing}
      onclick={() => setDecorating(!stickers.editing)}
      title={collapsed ? t('sticker.decorate') : t('sticker.decorateHint')}
      aria-label={t('sticker.decorate')}
      aria-pressed={stickers.editing}
    >
      <Sticker size={19} />
      <span class="label-text">{t('sticker.decorate')}</span>
    </button>

    <button
      class="nav-item"
      class:active={ui.page === 'settings'}
      onclick={() => (ui.page = 'settings')}
      title={tip(t('nav.settings'), shortcut(','))}
      aria-label={t('nav.settings')}
    >
      <SettingsIcon size={19} />
      <span class="label-text">{t('nav.settings')}</span>
    </button>

    {#if !narrow}
      <button
        class="nav-item collapse-btn"
        onclick={() => setSidebarCollapsed(!ui.sidebarCollapsed)}
        title={tip(collapsed ? t('nav.expand') : t('nav.collapse'), shortcut('\\'))}
        aria-label={collapsed ? t('nav.expand') : t('nav.collapse')}
      >
        {#if collapsed}<PanelLeftOpen size={19} />{:else}<PanelLeftClose size={19} />{/if}
        <span class="label-text">{t('nav.collapse')}</span>
      </button>
    {/if}
  </aside>

  <main>
    <div class="drag-top" data-tauri-drag-region></div>
    {#key ui.page}
      <div class="page-host" in:fly={{ y: 8, duration: 220 }}>
        {#if ui.page === 'home'}<Home />
        {:else if ui.page === 'calendar'}<Calendar />
        {:else if ui.page === 'ddays'}<DDays />
        {:else if ui.page === 'todos'}<Todos />
        {:else if ui.page === 'pomodoro'}<Pomodoro />
        {:else if ui.page === 'memos'}<Memos />
        {:else if ui.page === 'tracking'}<Tracking />
        {:else if ui.page === 'analytics'}<Analytics />
        {:else if ui.page === 'bookmarks'}<Bookmarks />
        {:else if ui.page === 'expenses'}<Expenses />
        {:else if ui.page === 'trpg'}<Trpg />
        {:else if ui.page === 'reading'}<Reading />
        {:else}<Settings />{/if}
      </div>
    {/key}
    <StickerLayer page={ui.page} />
  </main>

  <ContextMenu />
  <ScopeDialog />
  <EventHoverCard />

  <!-- Keyed list so a leaving toast keeps rendering its own data while it animates out. -->
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
  .app {
    --sidebar-w: 236px;
    display: grid;
    grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
    height: 100vh;
    transition: grid-template-columns 0.22s ease;
  }
  .app.collapsed {
    --sidebar-w: 76px;
  }
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 20px 14px 14px;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  /* macOS draws its window buttons over the top of the sidebar. */
  .mac .sidebar {
    padding-top: 44px;
  }
  .drag {
    position: absolute;
    inset: 0 0 auto 0;
    height: 40px;
  }
  .brand {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 5px 18px;
    min-width: 0;
  }
  .brand-text {
    min-width: 0;
  }
  .logo {
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: 13px;
    display: grid;
    place-items: center;
    color: var(--primary-text);
    background: linear-gradient(140deg, var(--primary), var(--accent-2));
    box-shadow: 0 6px 16px color-mix(in srgb, var(--primary) 40%, transparent);
  }
  .name {
    font-weight: 750;
    font-size: 17px;
    letter-spacing: -0.02em;
  }
  .tag {
    font-size: 12px;
    color: var(--faint);
  }
  /* In short windows the page list scrolls, so the buttons below it always stay visible. */
  nav {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 0 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--surface-3) transparent;
  }
  @media (max-height: 820px) {
    .brand {
      padding-bottom: 10px;
    }
    .sidebar .nav-item {
      height: 36px;
    }
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 42px;
    padding: 0 14px;
    border: none;
    border-radius: 13px;
    background: transparent;
    color: var(--muted);
    font-weight: 600;
    font-size: 14.5px;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    flex: none;
    transition: background 0.15s, color 0.15s;
  }
  .nav-item :global(svg) {
    flex: none;
  }
  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .collapse-btn {
    color: var(--faint);
  }
  .collapsed .label-text {
    display: none;
  }
  .collapsed .nav-item {
    padding: 0;
    justify-content: center;
  }
  .mini-timer {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    margin-bottom: 6px;
    min-width: 0;
  }
  .collapsed .mini-timer {
    flex-direction: column;
    gap: 4px;
    padding: 8px 0;
  }
  .collapsed .mini-time {
    font-size: 12px;
  }
  .mini-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .mini-time {
    font-weight: 700;
    font-size: 15px;
  }
  .mini-toggle {
    margin-left: auto;
    width: 26px;
    height: 26px;
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--text);
  }
  .tracker-pill {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
    text-align: left;
    min-width: 0;
    flex: none;
  }
  .collapsed .tracker-pill {
    justify-content: center;
    padding: 10px 0;
  }
  .tracker-pill:hover {
    background: var(--surface-2);
  }
  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .pulse.live {
    animation: pulse 2s infinite;
  }
  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--success) 60%, transparent); }
    70% { box-shadow: 0 0 0 7px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }
  main {
    position: relative;
    min-width: 0;
    min-height: 0;
    container-type: inline-size;
    container-name: main;
  }
  .drag-top {
    position: absolute;
    inset: 0 0 auto 0;
    height: 24px;
    z-index: 5;
  }
  .page-host {
    height: 100%;
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
    right: 24px;
    bottom: 24px;
    z-index: 100;
    max-width: min(420px, calc(100vw - 48px));
    padding: 12px 18px;
    border-radius: var(--radius);
    background: var(--text);
    color: var(--bg);
    font-weight: 550;
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
