<script lang="ts">
  import Select from '../components/Select.svelte';
  import { onDestroy, onMount } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Check, FolderPlus, Hash, Inbox, Layers, NotebookPen, Pencil, Plus, Search, Trash, Type } from '@lucide/svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import Modal from '../components/Modal.svelte';
  import { api, type Memo, type MemoGroup } from '../lib/api';
  import { PALETTE, hex } from '../lib/colors';
  import { fmtTs } from '../lib/dates';
  import { openMenu } from '../lib/menu.svelte';
  import { data, load, mutate, toast, ui } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  // The rich editor (ProseMirror) is large, so it is only loaded when first used.
  const loadRichEditor = () => import('../components/RichEditor.svelte');

  type Mode = 'rich' | 'markdown';
  type Filter = 'all' | 'ungrouped' | number;
  let memos = $state<Memo[]>([]);
  let groups = $state<MemoGroup[]>([]);
  let filter = $state<Filter>('all');
  let selectedId = $state<number | null>(null);
  let title = $state('');
  let body = $state('');
  /** Reading view (false) or editor (true). */
  let editing = $state(false);
  let mode = $state<Mode>('rich');
  let search = $state('');
  let dirty = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let editingGroup = $state<MemoGroup | null>(null);
  let dropTarget = $state<Filter | null>(null);

  $effect(() => {
    data.version;
    load(api.memoGroups(), []).then((g) => {
      groups = g;
      if (typeof filter === 'number' && !g.some((x) => x.id === filter)) filter = 'all';
    });
    load(api.memos(), []).then((m) => {
      memos = m;
      if (selectedId === null && m.length) select(m.find((x) => x.id === ui.memoFocus) ?? m[0]);
      ui.memoFocus = null;
    });
  });

  const groupById = $derived(new Map(groups.map((g) => [g.id, g])));
  const selected = $derived(memos.find((m) => m.id === selectedId) ?? null);
  const inFilter = (m: Memo, f: Filter) => (f === 'all' ? true : f === 'ungrouped' ? m.group_id === null : m.group_id === f);
  const countIn = (f: Filter) => memos.filter((m) => inFilter(m, f)).length;

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return memos.filter((m) => {
      const t = m.id === selectedId ? title : m.title;
      const b = m.id === selectedId ? body : m.body;
      return inFilter(m, filter) && (!q || t.toLowerCase().includes(q) || b.toLowerCase().includes(q));
    });
  });

  const html = $derived(DOMPurify.sanitize(marked.parse(body || t('memo.nothingWritten'), { async: false, gfm: true, breaks: true })));
  const words = $derived(body.trim() ? body.trim().split(/\s+/).length : 0);

  async function flush() {
    clearTimeout(saveTimer);
    if (!dirty || selectedId === null) return;
    dirty = false;
    try {
      await api.updateMemo(selectedId, title.trim() || t('common.untitled'), body);
      // Refresh the list order/preview without reloading the editor.
      memos = await api.memos();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function edited() {
    dirty = true;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(flush, 600);
  }

  async function select(m: Memo) {
    await flush();
    selectedId = m.id;
    title = m.title;
    body = m.body;
    // Memos open as they will look saved; an empty one goes straight to the editor.
    editing = !m.body.trim();
  }

  /** "Save": store now and switch to the reading view. */
  async function save() {
    await flush();
    editing = false;
    toast(t('memo.savedToast'), 'success');
  }

  async function create() {
    await flush();
    const id = await mutate(api.createMemo(t('common.untitled'), typeof filter === 'number' ? filter : null));
    if (id === undefined) return;
    memos = await api.memos();
    const m = memos.find((x) => x.id === id);
    if (m) await select(m);
    editing = true;
  }

  async function remove() {
    if (selectedId === null) return;
    clearTimeout(saveTimer);
    dirty = false;
    const id = selectedId;
    selectedId = null;
    await mutate(api.deleteMemo(id), t('memo.deleted'));
  }

  async function moveTo(id: number, groupId: number | null) {
    const name = groupId === null ? t('memo.ungrouped') : (groupById.get(groupId)?.name ?? '');
    await mutate(api.setMemoGroup(id, groupId), t('memo.movedTo', { name }));
  }

  function preview(m: Memo) {
    const b = m.id === selectedId ? body : m.body;
    return b.split('\n').find((l) => l.trim())?.replace(/^[#>\-*\s[\]x]+/, '') ?? t('memo.noContent');
  }

  // Open links from the preview in the system browser.
  function previewClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a');
    if (a?.href) {
      e.preventDefault();
      openUrl(a.href);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 's' && selectedId !== null) {
      e.preventDefault();
      if (editing) save();
    }
  }

  onMount(() => {
    // Older versions also stored "preview"; that is now the reading view.
    api.getSetting('memo.mode').then((m) => (m === 'markdown' || m === 'rich') && (mode = m)).catch(() => {});
  });

  function setMode(m: Mode) {
    mode = m;
    api.setSetting('memo.mode', m).catch(() => {});
  }

  function richChanged(markdown: string) {
    body = markdown;
    edited();
  }

  // ---- groups -----------------------------------------------------------------

  function newGroup() {
    editingGroup = { id: 0, name: '', color: PALETTE[groups.length % PALETTE.length].value };
  }

  async function saveGroup() {
    if (!editingGroup || !editingGroup.name.trim()) return;
    const id = await mutate(api.saveMemoGroup(editingGroup), t('memo.groupSaved'));
    if (id !== undefined && editingGroup.id === 0) filter = id;
    editingGroup = null;
  }

  async function deleteGroup() {
    if (!editingGroup) return;
    await mutate(api.deleteMemoGroup(editingGroup.id), t('memo.groupDeleted'));
    editingGroup = null;
  }

  function memoMenu(e: MouseEvent, m: Memo) {
    openMenu(e, [
      { label: t('memo.edit'), icon: Pencil, action: async () => { await select(m); editing = true; } },
      ...groups
        .filter((g) => g.id !== m.group_id)
        .map((g) => ({ label: t('memo.moveTo', { name: g.name }), icon: Layers, action: () => moveTo(m.id, g.id) })),
      ...(m.group_id !== null ? [{ label: t('memo.moveTo', { name: t('memo.ungrouped') }), icon: Inbox, action: () => moveTo(m.id, null) }] : []),
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: async () => { await select(m); remove(); } },
    ]);
  }

  // Drag a memo onto a group chip to move it.
  function dropZone(target: Filter) {
    return {
      ondragover: (e: DragEvent) => {
        if (target === 'all' || !e.dataTransfer?.types.includes('application/x-nora-memo')) return;
        e.preventDefault();
        dropTarget = target;
      },
      ondragleave: () => dropTarget === target && (dropTarget = null),
      ondrop: (e: DragEvent) => {
        e.preventDefault();
        dropTarget = null;
        const id = Number(e.dataTransfer?.getData('application/x-nora-memo'));
        const m = memos.find((x) => x.id === id);
        const groupId = target === 'ungrouped' ? null : (target as number);
        if (m && m.group_id !== groupId) moveTo(m.id, groupId);
      },
    };
  }

  onDestroy(() => {
    flush();
  });
</script>

<svelte:window {onkeydown} />

<div class="memos">
  <aside class="list-pane">
    <div class="list-head">
      <h1>{t('memo.title')}</h1>
      <button class="btn primary small" onclick={create}><Plus size={16} /> {t('memo.new')}</button>
    </div>

    <div class="groups" title={t('memo.dragHint')}>
      <button class="group-chip" class:active={filter === 'all'} onclick={() => (filter = 'all')}>
        <Layers size={13} /> {t('memo.allMemos')} <span class="n">{memos.length}</span>
      </button>
      <button class="group-chip" class:active={filter === 'ungrouped'} class:drop={dropTarget === 'ungrouped'}
        onclick={() => (filter = 'ungrouped')} {...dropZone('ungrouped')}>
        <Inbox size={13} /> {t('memo.ungrouped')} <span class="n">{countIn('ungrouped')}</span>
      </button>
      {#each groups as g (g.id)}
        <button
          class="group-chip"
          class:active={filter === g.id}
          class:drop={dropTarget === g.id}
          style:--c={hex(g.color)}
          onclick={() => (filter = g.id)}
          ondblclick={() => (editingGroup = { ...g })}
          oncontextmenu={(e) => openMenu(e, [{ label: t('memo.editGroup'), icon: Pencil, action: () => (editingGroup = { ...g }) }])}
          {...dropZone(g.id)}
        >
          <span class="dot" style:background="var(--c)"></span> <span class="truncate">{g.name}</span> <span class="n">{countIn(g.id)}</span>
        </button>
      {/each}
      <button class="group-chip add" onclick={newGroup} title={t('memo.newGroup')}><FolderPlus size={13} /> {t('memo.newGroup')}</button>
    </div>

    <div class="search">
      <Search size={16} />
      <input class="input" placeholder={t('memo.search')} bind:value={search} />
    </div>
    <div class="list">
      {#each filtered as m (m.id)}
        {@const g = m.group_id !== null ? groupById.get(m.group_id) : undefined}
        <!-- A plain box, not a <button>: the system WebKit lays out a button's inside its own
             way and long text could push past the card's border. -->
        <div
          class="memo-item"
          class:active={m.id === selectedId}
          role="button"
          tabindex="0"
          aria-current={m.id === selectedId}
          draggable="true"
          ondragstart={(e) => { e.dataTransfer?.setData('application/x-nora-memo', String(m.id)); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          onclick={() => m.id !== selectedId && select(m)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (m.id !== selectedId) select(m); } }}
          oncontextmenu={(e) => memoMenu(e, m)}
        >
          <span class="m-title">
            {#if g}<span class="dot" style:background={hex(g.color)} title={g.name}></span>{/if}
            <!-- The text needs its own box to end in "…"; a flex row cannot. -->
            <span class="truncate">{(m.id === selectedId ? title : m.title) || t('common.untitled')}</span>
          </span>
          <span class="m-preview truncate">{preview(m)}</span>
          <span class="m-date">{fmtTs(m.updated_at, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hour12: false })}</span>
        </div>
      {:else}
        <p class="muted small empty-list">{memos.length ? t('memo.noMatch') : t('memo.none')}</p>
      {/each}
    </div>
  </aside>

  <section class="editor">
    {#if selectedId === null}
      <div class="empty">
        <span class="empty-icon"><NotebookPen size={30} /></span>
        <h2>{t('memo.emptyTitle')}</h2>
        <p>{t('memo.emptyBody')}</p>
        <button class="btn primary" onclick={create}><Plus size={17} /> {t('memo.writeFirst')}</button>
      </div>
    {:else}
      <div class="toolbar">
        {#if editing}
          <input class="title-input" placeholder={t('common.untitled')} bind:value={title} oninput={edited} />
        {:else}
          <h1 class="title-view truncate">{title || t('common.untitled')}</h1>
        {/if}
        <div class="group-select">
          <Select
            value={selected?.group_id ?? null}
            options={[{ value: null, label: t('memo.ungrouped') }, ...groups.map((g) => ({ value: g.id as number | null, label: g.name, color: hex(g.color) }))]}
            onchange={(v) => selectedId !== null && moveTo(selectedId, v)}
            label={t('memo.group')}
          />
        </div>
        {#if editing}
          <div class="segmented">
            <button class:active={mode === 'rich'} onclick={() => setMode('rich')}><Type size={15} /> {t('memo.modeRich')}</button>
            <button class:active={mode === 'markdown'} onclick={() => setMode('markdown')}><Hash size={15} /> {t('memo.modeMarkdown')}</button>
          </div>
          <button class="btn primary" onclick={save}><Check size={16} /> {t('memo.save')}</button>
        {:else}
          <button class="btn primary" onclick={() => (editing = true)}><Pencil size={15} /> {t('memo.edit')}</button>
        {/if}
        <ConfirmButton onconfirm={remove} label="" question={t('memo.deleteQ')} />
      </div>
      <p class="status faint small">
        {#if editing}
          {dirty ? t('memo.saving') : t('memo.allSaved')} · {t('memo.words', { n: words })} · {t('memo.saveHint')}
        {:else}
          {selected ? t('memo.editedAt', { date: fmtTs(selected.updated_at, { dateStyle: 'medium', timeStyle: 'short' }) }) : ''} ·
          {t('memo.words', { n: words })} · {t('memo.viewHint')}
        {/if}
      </p>

      {#if !editing}
        <!-- Reading view: the memo as it looks saved. Double-click to edit. -->
        <div class="box">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="scroll prose reading" onclick={previewClick} ondblclick={() => (editing = true)}>{@html html}</div>
        </div>
      {:else}
        <div class="panes" class:split={mode === 'markdown'}>
          {#if mode === 'rich'}
            {#await loadRichEditor() then { default: RichEditor }}
              {#key selectedId}
                <RichEditor value={body} onchange={richChanged} />
              {/key}
            {/await}
          {:else}
            <div class="box">
              <textarea class="md-input" bind:value={body} oninput={edited} placeholder={t('memo.placeholder')}></textarea>
            </div>
            <div class="box">
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="scroll prose" onclick={previewClick}>{@html html}</div>
            </div>
          {/if}
        </div>
      {/if}
    {/if}
  </section>
</div>

{#if editingGroup}
  <Modal title={editingGroup.id ? t('memo.editGroup') : t('memo.newGroup')} onclose={() => (editingGroup = null)} width={420}>
    <div class="field">
      <label for="memo-group-name">{t('memo.groupName')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="memo-group-name" class="input" placeholder={t('memo.groupNamePlaceholder')} bind:value={editingGroup.name} autofocus
        onkeydown={(e) => e.key === 'Enter' && saveGroup()} />
    </div>
    <div class="field">
      <span class="label">{t('common.color')}</span>
      <ColorPicker bind:value={editingGroup.color} />
    </div>
    {#if editingGroup.id}<p class="faint small">{t('memo.groupDeleteHint')}</p>{/if}
    {#snippet footer()}
      {#if editingGroup?.id}<ConfirmButton onconfirm={deleteGroup} question={t('memo.deleteGroupQ')} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingGroup = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveGroup}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .memos {
    display: grid;
    grid-template-columns: 290px minmax(0, 1fr);
    height: 100%;
  }
  .list-pane {
    display: flex;
    flex-direction: column;
    padding: 28px 14px 14px;
    border-right: 1px solid var(--border);
    min-height: 0;
  }
  .list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 12px;
  }
  .groups {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 2px 12px;
  }
  .group-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
    height: 28px;
    padding: 0 10px;
    border: 1.5px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }
  .group-chip:hover {
    color: var(--text);
    border-color: color-mix(in srgb, var(--primary) 40%, var(--border));
  }
  .group-chip.active {
    background: var(--primary-soft);
    border-color: var(--primary);
    color: var(--primary);
  }
  .group-chip.drop {
    border-style: dashed;
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .group-chip.add {
    border-style: dashed;
    color: var(--faint);
  }
  .group-chip .n {
    font-size: 11px;
    font-weight: 700;
    opacity: 0.7;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--faint);
    margin-bottom: 10px;
  }
  .search :global(svg) {
    position: absolute;
    left: 12px;
  }
  .search .input {
    padding-left: 36px;
    border-radius: var(--radius);
  }
  .list {
    flex: 1;
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    /* Room for the selected card's outline and shadow, which the scroll area would clip. */
    padding: 3px 4px 8px;
  }
  .memo-item {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 2px;
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    padding: 11px 13px;
    /* A real border (not an outer ring) so the scroll area can never cut it off.
       Whole pixels: a 1.5px border can draw unevenly on some screens. */
    border: 2px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    /* Nothing inside may ever spill past the rounded border. */
    overflow: hidden;
  }
  .memo-item > * {
    display: block;
    min-width: 0;
    max-width: 100%;
  }
  .memo-item:focus-visible {
    outline: none;
    border-color: color-mix(in srgb, var(--primary) 55%, transparent);
  }
  .memo-item:hover {
    background: var(--surface-2);
  }
  .memo-item.active {
    background: var(--surface);
    border-color: var(--primary);
    box-shadow: var(--shadow-sm);
  }
  .memo-item > .m-title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-weight: 650;
  }
  .m-preview {
    font-size: 12.5px;
    color: var(--muted);
  }
  .m-date {
    font-size: 11.5px;
    color: var(--faint);
  }
  .empty-list {
    padding: 12px;
  }
  .editor {
    display: flex;
    flex-direction: column;
    padding: 24px 32px;
    min-width: 0;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .title-input,
  .title-view {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 26px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }
  .title-input {
    border: none;
    background: none;
    outline: none;
  }
  .group-select {
    width: 160px;
    flex: none;
  }
  .status {
    margin: 4px 0 14px;
  }
  .panes {
    flex: 1;
    display: grid;
    /* A fixed row keeps long memos scrolling inside the box instead of pushing its
       bottom edge out of the window. */
    grid-template-rows: minmax(0, 1fr);
    gap: 16px;
    min-height: 0;
  }
  .panes.split {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }
  /* Bordered boxes never scroll themselves; their inside does. This avoids WebKit
     leaving gaps in a rounded border while the content scrolls. */
  .box {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  .box:focus-within {
    border-color: var(--primary);
  }
  .scroll {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 20px 24px;
  }
  .reading {
    padding: 26px 32px;
    font-size: 15px;
  }
  .md-input {
    flex: 1;
    width: 100%;
    resize: none;
    padding: 20px;
    border: none;
    background: transparent;
    font-family: var(--mono);
    font-size: 13.5px;
    line-height: 1.7;
    outline: none;
  }
  @container main (max-width: 900px) {
    .memos {
      grid-template-columns: 230px minmax(0, 1fr);
    }
    .panes.split {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) minmax(0, 1fr);
    }
    .editor {
      padding: 22px 18px;
    }
  }
  @container main (max-width: 620px) {
    .memos {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: 260px minmax(0, 1fr);
    }
    .list-pane {
      padding: 22px 12px 8px;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .groups {
      flex-wrap: nowrap;
      overflow-x: auto;
    }
    .toolbar {
      flex-wrap: wrap;
    }
  }
</style>
