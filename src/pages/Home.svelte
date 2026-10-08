<script lang="ts">
  import { onMount, type Component } from 'svelte';
  import { api, type DDay } from '../lib/api';
  import { flip } from 'svelte/animate';
  import { fade } from 'svelte/transition';
  import { ChevronLeft, ChevronRight, GripVertical, ImagePlus, LayoutDashboard, Pencil, Plus, Puzzle, RotateCcw, X } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import { fmt, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import {
    HEADER_HEIGHTS, SIZES, WIDGETS, addTile, home, loadLayout, moveTile, removeTile, resetLayout, saveLayout, setHeader,
    setTileDday, setTileSize, type HeaderHeight, type TileSize, type WidgetId,
  } from '../lib/home.svelte';
  import { data, load, toast } from '../lib/state.svelte';
  import { byDdayTarget, ddayLabel, ddayUpcoming } from '../lib/dates';
  import DDayCardWidget from '../widgets/DDayCardWidget.svelte';
  import BookmarksWidget from '../widgets/BookmarksWidget.svelte';
  import ExpensesWidget from '../widgets/ExpensesWidget.svelte';
  import GreetingWidget from '../widgets/GreetingWidget.svelte';
  import AgendaWidget from '../widgets/AgendaWidget.svelte';
  import CalendarWidget from '../widgets/CalendarWidget.svelte';
  import TodosWidget from '../widgets/TodosWidget.svelte';
  import DDaysWidget from '../widgets/DDaysWidget.svelte';
  import PomodoroWidget from '../widgets/PomodoroWidget.svelte';
  import WorkingWidget from '../widgets/WorkingWidget.svelte';
  import WeekChartWidget from '../widgets/WeekChartWidget.svelte';
  import ProgressWidget from '../widgets/ProgressWidget.svelte';
  import MemosWidget from '../widgets/MemosWidget.svelte';
  import ImageCardWidget from '../widgets/ImageCardWidget.svelte';
  import ImageFrame from '../components/ImageFrame.svelte';
  import ImageFramer from '../components/ImageFramer.svelte';

  const COMPONENTS: Record<WidgetId, Component<any>> = {
    dday: DDayCardWidget,
    bookmarks: BookmarksWidget,
    expenses: ExpensesWidget,
    greeting: GreetingWidget,
    agenda: AgendaWidget,
    calendar: CalendarWidget,
    todos: TodosWidget,
    ddays: DDaysWidget,
    pomodoro: PomodoroWidget,
    working: WorkingWidget,
    weekChart: WeekChartWidget,
    progress: ProgressWidget,
    memos: MemosWidget,
    image: ImageCardWidget,
  };

  let editing = $state(false);
  let editingHeader = $state(false);
  let pageWidth = $state(900);
  // The header editor keeps its own height choice until saved.
  let headerHeight = $state<HeaderHeight>('md');
  function openHeaderEditor() {
    headerHeight = home.header.height;
    editingHeader = true;
  }
  let gallery = $state(false);
  let dragIndex = $state<number | null>(null);

  // Widgets marked `multiple` (D-Day cards) can always be added again.
  const available = $derived(
    (Object.keys(WIDGETS) as WidgetId[]).filter((id) => WIDGETS[id].multiple || !home.tiles.some((t) => t.id === id)),
  );

  let ddays = $state<DDay[]>([]);
  $effect(() => {
    data.version;
    load(api.ddays(), []).then((d) => (ddays = d));
  });

  /** A new D-Day card shows an upcoming D-Day that no other card shows yet. */
  function nextUnusedDday(): number | null {
    const used = new Set(home.tiles.filter((t) => t.id === 'dday').map((t) => t.ddayId));
    const upcoming = byDdayTarget(ddays.filter((d) => ddayUpcoming(d)));
    return (upcoming.find((d) => !used.has(d.id)) ?? upcoming[0] ?? ddays[0])?.id ?? null;
  }

  onMount(() => {
    if (!home.loaded) loadLayout();
  });

  function dragStart(i: number, e: DragEvent) {
    dragIndex = i;
    e.dataTransfer?.setData('text/plain', String(i));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  // Reorder live while dragging so the grid shows where the tile will land.
  function dragOver(i: number, e: DragEvent) {
    if (dragIndex === null) return;
    e.preventDefault();
    if (i !== dragIndex) {
      moveTile(dragIndex, i);
      dragIndex = i;
    }
  }

  function dragEnd() {
    dragIndex = null;
    saveLayout();
  }

  function move(i: number, dir: number) {
    moveTile(i, i + dir);
    saveLayout();
  }

  function add(id: WidgetId) {
    addTile(id, id === 'dday' ? nextUnusedDday() : null);
    if (available.length === 0) gallery = false;
  }
</script>

<div class="page">
  {#if home.header.image}
    <div class="home-header" style:height="{HEADER_HEIGHTS[home.header.height]}px">
      <ImageFrame image={home.header.image} framing={home.header.framing} />
      <button class="header-edit" onclick={openHeaderEditor}><Pencil size={14} /> {t('home.headerEdit')}</button>
    </div>
  {/if}
  <div class="page-header" bind:clientWidth={pageWidth}>
    <div>
      <h1>{t('nav.home')}</h1>
      <p class="sub">{editing ? t('home.dragHint') : fmt(today(), { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' })}</p>
    </div>
    <span class="spacer"></span>
    {#if editing}
      <button class="btn ghost" onclick={() => { resetLayout(); toast(t('home.resetDone'), 'success'); }}><RotateCcw size={16} /> {t('home.reset')}</button>
      <button class="btn" onclick={openHeaderEditor}><ImagePlus size={16} /> {home.header.image ? t('home.headerEdit') : t('home.headerAdd')}</button>
      <button class="btn" onclick={() => (gallery = true)}><Plus size={16} /> {t('home.addTile')}</button>
      <button class="btn primary" onclick={() => (editing = false)}>{t('home.done')}</button>
    {:else}
      <button class="btn" onclick={() => (editing = true)}><LayoutDashboard size={16} /> {t('home.customize')}</button>
    {/if}
  </div>

  {#if home.loaded && home.tiles.length === 0}
    <div class="card empty">
      <span class="empty-icon"><Puzzle size={30} /></span>
      <h2>{t('home.empty')}</h2>
      <p>{t('home.emptyBody')}</p>
      <button class="btn primary" onclick={() => (gallery = true)}><Plus size={17} /> {t('home.addTile')}</button>
    </div>
  {/if}

  <div class="tiles" class:editing role="list">
    {#each home.tiles as tile, i (tile.uid)}
      {@const Widget = COMPONENTS[tile.id]}
      {@const TileIcon = WIDGETS[tile.id].icon}
      <div
        class="tile {tile.size}"
        class:bleed={tile.id === 'image'}
        class:dragging={dragIndex === i}
        animate:flip={{ duration: 220 }}
        draggable={editing}
        ondragstart={(e) => dragStart(i, e)}
        ondragover={(e) => dragOver(i, e)}
        ondragend={dragEnd}
        ondrop={(e) => e.preventDefault()}
        role="listitem"
      >
        <div class="content" inert={editing}><Widget ddayId={tile.ddayId} uid={tile.uid} /></div>
        {#if editing}
          <div class="edit" transition:fade={{ duration: 120 }}>
            <div class="edit-top">
              <GripVertical size={16} />
              <span class="w-icon"><TileIcon size={15} /></span>
              <span class="edit-name truncate">{t(WIDGETS[tile.id].name)}</span>
              <button class="icon-btn danger" onclick={() => removeTile(i)} title={t('home.remove')} aria-label={t('home.remove')}><X size={16} /></button>
            </div>
            {#if tile.id === 'dday'}
              <select
                class="select dday-pick"
                value={tile.ddayId ?? ''}
                onchange={(e) => setTileDday(i, e.currentTarget.value ? Number(e.currentTarget.value) : null)}
                aria-label={t('w.dday.pick')}
              >
                <option value="">{t('w.dday.auto')}</option>
                {#each ddays as d (d.id)}<option value={d.id}>{ddayLabel(d)} · {d.title}</option>{/each}
              </select>
            {/if}
            <div class="sizes">
              {#each SIZES as s (s.id)}
                <button class:active={tile.size === s.id} onclick={() => setTileSize(i, s.id as TileSize)}>
                  <span class="size-icon {s.id}"></span>{t(s.label)}
                </button>
              {/each}
            </div>
            <div class="edit-bottom">
              <button class="icon-btn" onclick={() => move(i, -1)} disabled={i === 0} title={t('home.moveEarlier')}><ChevronLeft size={18} /></button>
              <button class="icon-btn" onclick={() => move(i, 1)} disabled={i === home.tiles.length - 1} title={t('home.moveLater')}><ChevronRight size={18} /></button>
            </div>
          </div>
        {/if}
      </div>
    {/each}
    {#if editing && available.length}
      <button class="tile sm add-tile" onclick={() => (gallery = true)}>
        <Plus size={22} />
        {t('home.addTile')}
      </button>
    {/if}
  </div>
</div>

{#if editingHeader}
  <ImageFramer
    title={t('home.headerTitle')}
    image={home.header.image}
    framing={home.header.framing}
    width={Math.max(320, pageWidth)}
    height={HEADER_HEIGHTS[headerHeight]}
    longSide={2400}
    purpose="header"
    onsave={(image, framing) => setHeader({ image, framing, height: headerHeight })}
    onremove={() => setHeader({ ...home.header, image: null })}
    onclose={() => (editingHeader = false)}
  >
    {#snippet extra()}
      <div class="row header-height">
        <span class="strong">{t('home.headerHeight')}</span>
        <div class="segmented">
          {#each Object.keys(HEADER_HEIGHTS) as hh (hh)}
            <button class:active={headerHeight === hh} onclick={() => (headerHeight = hh as HeaderHeight)}>{t(`home.headerHeight.${hh as HeaderHeight}`)}</button>
          {/each}
        </div>
      </div>
    {/snippet}
  </ImageFramer>
{/if}

{#if gallery}
  <Modal title={t('home.addTileTitle')} onclose={() => (gallery = false)} width={620}>
    <p class="muted">{t('home.addTileBody')}</p>
    {#if available.length === 0}
      <p class="muted">{t('home.allAdded')}</p>
    {/if}
    <div class="gallery">
      {#each available as id (id)}
        {@const GalleryIcon = WIDGETS[id].icon}
        <button class="gallery-item" onclick={() => add(id)}>
          <span class="g-icon"><GalleryIcon size={22} /></span>
          <span class="g-text">
            <span class="g-name">{t(WIDGETS[id].name)}</span>
            <span class="muted small">{t(WIDGETS[id].desc)}</span>
          </span>
          <span class="g-add"><Plus size={16} /></span>
        </button>
      {/each}
    </div>
  </Modal>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    grid-auto-rows: 184px;
    grid-auto-flow: row dense;
    gap: 16px;
  }
  .tile {
    position: relative;
    min-width: 0;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 24px;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    container-type: inline-size;
    container-name: tile;
    transition: box-shadow 0.15s, transform 0.15s, opacity 0.15s;
  }
  .tile:hover {
    box-shadow: var(--shadow);
  }
  .tile.wide {
    grid-column: span 2;
  }
  .tile.tall {
    grid-row: span 2;
  }
  .tile.large {
    grid-column: span 2;
    grid-row: span 2;
  }
  /* One column: wide tiles must not force a second column. */
  @container main (max-width: 560px) {
    .tile.wide,
    .tile.large {
      grid-column: span 1;
    }
  }
  .content {
    height: 100%;
  }
  .tile.bleed {
    padding: 0;
  }
  .home-header {
    position: relative;
    margin-bottom: 20px;
    border-radius: 24px;
    overflow: hidden;
    box-shadow: var(--shadow-sm);
    background: var(--surface-2);
  }
  .header-edit {
    position: absolute;
    right: 12px;
    bottom: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: none;
    border-radius: 999px;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    color: var(--text);
    font-weight: 600;
    font-size: 12.5px;
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .home-header:hover .header-edit,
  .header-edit:focus-visible {
    opacity: 1;
  }
  .header-height {
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .editing .tile {
    cursor: grab;
    border-style: dashed;
    border-color: color-mix(in srgb, var(--primary) 50%, var(--border));
  }
  .editing .tile.dragging {
    opacity: 0.4;
    transform: scale(0.97);
  }
  .edit {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 12px;
    background: color-mix(in srgb, var(--surface) 82%, transparent);
    backdrop-filter: blur(3px);
    -webkit-backdrop-filter: blur(3px);
  }
  .edit-top {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .edit-name {
    flex: 1;
    font-weight: 700;
    color: var(--text);
  }
  .dday-pick {
    height: 32px;
    font-size: 12.5px;
    background: var(--surface);
  }
  .sizes {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
    align-self: center;
    width: min(100%, 240px);
  }
  .sizes button {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 650;
    cursor: pointer;
  }
  .sizes button.active {
    border-color: var(--primary);
    color: var(--primary);
    background: var(--primary-soft);
  }
  .size-icon {
    display: inline-block;
    border-radius: 3px;
    border: 1.5px solid currentColor;
  }
  .size-icon.sm { width: 9px; height: 9px; }
  .size-icon.wide { width: 16px; height: 9px; }
  .size-icon.tall { width: 9px; height: 16px; }
  .size-icon.large { width: 16px; height: 16px; }
  .edit-bottom {
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .add-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 2px dashed var(--border);
    background: transparent;
    box-shadow: none;
    color: var(--muted);
    font-weight: 650;
    cursor: pointer;
  }
  .add-tile:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 10px;
  }
  .gallery-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.1s;
  }
  .gallery-item:hover {
    border-color: var(--primary);
  }
  .gallery-item:active {
    transform: scale(0.98);
  }
  .g-icon {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 14px;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: var(--primary-soft);
  }
  .g-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .g-name {
    font-weight: 700;
  }
  .g-add {
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: var(--primary-soft);
  }
</style>
