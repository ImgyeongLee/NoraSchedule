<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    Bookmark as BookmarkIcon, ChevronRight, Copy, ExternalLink, Folder, FolderOpen, FolderPlus, Inbox, Layers, Pencil, Plus,
    Search, Trash,
  } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import { api, type Bookmark, type BookmarkFolder } from '../lib/api';
  import { KINDS, detectKind, hostOf, kindOf, normalizeUrl, titleFromUrl } from '../lib/bookmarks';
  import { copyOnHover, shortcut } from '../lib/clipboard.svelte';
  import { PALETTE, hex } from '../lib/colors';
  import { t } from '../lib/i18n.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';

  type View = 'all' | 'unsorted' | number;
  let view = $state<View>('all');
  let folders = $state<BookmarkFolder[]>([]);
  let links = $state<Bookmark[]>([]);
  let search = $state('');
  let editingLink = $state<Bookmark | null>(null);
  let kindTouched = $state(false);
  let linkError = $state('');
  let editingFolder = $state<BookmarkFolder | null>(null);
  let folderError = $state('');
  let dropTarget = $state<View | null>(null);
  let collapsed = $state(new Set<number>());

  $effect(() => {
    data.version;
    load(api.bookmarkFolders(), []).then((f) => {
      folders = f;
      if (typeof view === 'number' && !f.some((x) => x.id === view)) view = 'all';
    });
    load(api.bookmarks(), []).then((b) => (links = b));
  });

  const childrenOf = (parent: number | null) => folders.filter((f) => f.parent_id === parent);
  const folderById = $derived(new Map(folders.map((f) => [f.id, f])));

  /** Folder ids from the top level down to `id`. */
  function pathTo(id: number): BookmarkFolder[] {
    const path: BookmarkFolder[] = [];
    let current = folderById.get(id);
    while (current && path.length < 50) {
      path.unshift(current);
      current = current.parent_id !== null ? folderById.get(current.parent_id) : undefined;
    }
    return path;
  }

  function descendantsOf(id: number): Set<number> {
    const out = new Set<number>([id]);
    let added = true;
    while (added) {
      added = false;
      for (const f of folders) if (f.parent_id !== null && out.has(f.parent_id) && !out.has(f.id)) (out.add(f.id), (added = true));
    }
    return out;
  }

  /** Folders as an indented list, for the folder pickers. */
  function flatFolders(exclude = new Set<number>()): { folder: BookmarkFolder; depth: number }[] {
    const out: { folder: BookmarkFolder; depth: number }[] = [];
    const walk = (parent: number | null, depth: number) => {
      for (const f of childrenOf(parent)) {
        if (exclude.has(f.id)) continue;
        out.push({ folder: f, depth });
        walk(f.id, depth + 1);
      }
    };
    walk(null, 0);
    return out;
  }

  const query = $derived(search.trim().toLowerCase());
  const visible = $derived(
    query
      ? links.filter((b) => `${b.title} ${b.url} ${b.note}`.toLowerCase().includes(query))
      : view === 'all'
        ? links
        : links.filter((b) => b.folder_id === (view === 'unsorted' ? null : view)),
  );
  const subfolders = $derived(typeof view === 'number' && !query ? childrenOf(view) : view === 'all' && !query ? childrenOf(null) : []);
  const countIn = (id: number) => links.filter((b) => b.folder_id === id).length;
  const heading = $derived(view === 'all' ? t('bm.all') : view === 'unsorted' ? t('bm.unsorted') : (folderById.get(view)?.name ?? ''));

  // ---- links ------------------------------------------------------------------

  function newLink() {
    linkError = '';
    kindTouched = false;
    editingLink = { id: 0, folder_id: typeof view === 'number' ? view : null, title: '', url: '', kind: 'other', note: '', created_at: 0 };
  }

  function editLink(b: Bookmark) {
    linkError = '';
    kindTouched = true;
    editingLink = { ...b };
  }

  function urlChanged() {
    if (editingLink && !kindTouched) editingLink.kind = detectKind(editingLink.url);
  }

  async function saveLink() {
    if (!editingLink) return;
    const url = editingLink.url.trim() ? normalizeUrl(editingLink.url) : '';
    if (!url || /^javascript:/i.test(url)) return (linkError = t('bm.invalidUrl'));
    const link = { ...editingLink, url, title: editingLink.title.trim() || titleFromUrl(url) };
    const ok = await mutate(api.saveBookmark(link), link.id ? t('bm.saved') : t('bm.added'));
    if (ok !== undefined) editingLink = null;
  }

  function copyLink(b: Bookmark) {
    navigator.clipboard?.writeText(b.url).catch(() => {});
    toast(t('bm.linkCopied'));
  }

  async function deleteLink(b: Bookmark) {
    try {
      const stored = await api.deleteBookmark(b.id);
      data.version++;
      toast(t('clip.deleted', { name: b.title }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveBookmark({ ...stored, id: 0 }), t('bm.saved')),
      });
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function linkMenu(e: MouseEvent, b: Bookmark) {
    openMenu(e, [
      { label: t('bm.open'), icon: ExternalLink, action: () => openUrl(b.url) },
      { label: t('menu.edit'), icon: Pencil, action: () => editLink(b) },
      { label: t('bm.copyLink'), icon: Copy, shortcut: shortcut('C'), action: () => copyLink(b) },
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: () => deleteLink(b) },
    ]);
  }

  // Drag a link box onto a folder in the sidebar to move it.
  function dragLink(e: DragEvent, b: Bookmark) {
    e.dataTransfer?.setData('application/x-nora-bookmark', String(b.id));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function dropOn(e: DragEvent, target: View) {
    e.preventDefault();
    dropTarget = null;
    const id = Number(e.dataTransfer?.getData('application/x-nora-bookmark'));
    const link = links.find((b) => b.id === id);
    if (!link || target === 'all') return;
    const folderId = target === 'unsorted' ? null : target;
    if (link.folder_id !== folderId) mutate(api.saveBookmark({ ...link, folder_id: folderId }), t('bm.saved'));
  }

  function dropZone(target: View) {
    return {
      ondragover: (e: DragEvent) => {
        if (target === 'all' || !e.dataTransfer?.types.includes('application/x-nora-bookmark')) return;
        e.preventDefault();
        dropTarget = target;
      },
      ondragleave: () => dropTarget === target && (dropTarget = null),
      ondrop: (e: DragEvent) => dropOn(e, target),
    };
  }

  // ---- folders ----------------------------------------------------------------

  function newFolder(parent: number | null = typeof view === 'number' ? view : null) {
    folderError = '';
    editingFolder = { id: 0, name: '', parent_id: parent, color: PALETTE[folders.length % PALETTE.length].value };
  }

  async function saveFolder() {
    if (!editingFolder || !editingFolder.name.trim()) return;
    try {
      const id = await api.saveBookmarkFolder(editingFolder);
      data.version++;
      toast(t('bm.folderSaved'), 'success');
      if (editingFolder.id === 0) view = id;
      editingFolder = null;
    } catch (e) {
      folderError = String(e).includes('folder_cycle') ? t('bm.folderCycle') : String(e);
    }
  }

  async function deleteFolder() {
    if (!editingFolder) return;
    await mutate(api.deleteBookmarkFolder(editingFolder.id), t('bm.folderDeleted'));
    editingFolder = null;
  }

  function folderMenu(e: MouseEvent, f: BookmarkFolder) {
    openMenu(e, [
      { label: t('bm.addLink'), icon: Plus, action: () => { view = f.id; newLink(); } },
      { label: t('bm.newFolder'), icon: FolderPlus, action: () => newFolder(f.id) },
      { label: t('bm.editFolder'), icon: Pencil, action: () => { folderError = ''; editingFolder = { ...f }; } },
    ]);
  }
</script>

{#snippet tree(parent: number | null, depth: number)}
  {#each childrenOf(parent) as f (f.id)}
    {@const kids = childrenOf(f.id)}
    <div class="tree-row" style:--depth={depth}>
      {#if kids.length}
        <button
          class="twisty"
          class:open={!collapsed.has(f.id)}
          onclick={() => {
            const next = new Set(collapsed);
            if (!next.delete(f.id)) next.add(f.id);
            collapsed = next;
          }}
          aria-label={f.name}><ChevronRight size={14} /></button
        >
      {:else}
        <span class="twisty"></span>
      {/if}
      <button
        class="folder"
        class:active={view === f.id}
        class:drop={dropTarget === f.id}
        onclick={() => { view = f.id; search = ''; }}
        oncontextmenu={(e) => folderMenu(e, f)}
        {...dropZone(f.id)}
      >
        {#if view === f.id}<FolderOpen size={16} color={hex(f.color)} />{:else}<Folder size={16} color={hex(f.color)} />{/if}
        <span class="truncate">{f.name}</span>
        <span class="count">{countIn(f.id) || ''}</span>
      </button>
    </div>
    {#if !collapsed.has(f.id)}{@render tree(f.id, depth + 1)}{/if}
  {/each}
{/snippet}

<div class="bookmarks">
  <aside class="side">
    <h1>{t('nav.bookmarks')}</h1>
    <div class="list">
      <button class="folder top" class:active={view === 'all'} onclick={() => { view = 'all'; search = ''; }}>
        <Layers size={16} /> <span>{t('bm.all')}</span> <span class="count">{links.length}</span>
      </button>
      <button class="folder top" class:active={view === 'unsorted'} class:drop={dropTarget === 'unsorted'} onclick={() => { view = 'unsorted'; search = ''; }} {...dropZone('unsorted')}>
        <Inbox size={16} /> <span>{t('bm.unsorted')}</span> <span class="count">{links.filter((b) => b.folder_id === null).length}</span>
      </button>
    </div>
    <div class="label folders-label">{t('bm.folders')}</div>
    <div class="list">
      {@render tree(null, 0)}
      <button class="folder add" onclick={() => newFolder(null)}><FolderPlus size={16} /> <span>{t('bm.newFolder')}</span></button>
    </div>
  </aside>

  <section class="main">
    <div class="head">
      <div class="title-block">
        {#if typeof view === 'number'}
          <div class="crumbs">
            <button onclick={() => (view = 'all')}>{t('bm.all')}</button>
            {#each pathTo(view) as f (f.id)}
              <ChevronRight size={13} />
              <button onclick={() => (view = f.id)}>{f.name}</button>
            {/each}
          </div>
        {/if}
        <h2 class="heading">{query ? t('bm.search') : heading}</h2>
        <p class="muted small">{t('bm.count', { n: visible.length })}</p>
      </div>
      <span class="spacer"></span>
      <div class="search">
        <Search size={16} />
        <input class="input" placeholder={t('bm.search')} bind:value={search} />
      </div>
      {#if typeof view === 'number'}
        <button class="btn" onclick={() => { const f = folderById.get(view as number); if (f) { folderError = ''; editingFolder = { ...f }; } }}>
          <Pencil size={15} /> {t('bm.editFolder')}
        </button>
      {/if}
      <button class="btn primary" onclick={newLink}><Plus size={17} /> {t('bm.addLink')}</button>
    </div>

    {#if subfolders.length}
      <div class="subfolders">
        {#each subfolders as f (f.id)}
          <button class="subfolder" style:--c={hex(f.color)} onclick={() => (view = f.id)} oncontextmenu={(e) => folderMenu(e, f)} {...dropZone(f.id)} class:drop={dropTarget === f.id}>
            <span class="sub-icon"><Folder size={18} /></span>
            <span class="truncate">{f.name}</span>
            <span class="faint small">{countIn(f.id)}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if visible.length}
      <div class="grid">
        {#each visible as b (b.id)}
          {@const kind = kindOf(b.kind)}
          <div
            class="link-card"
            role="link"
            tabindex="0"
            draggable="true"
            title={b.url}
            style:--c={kind.color}
            onclick={() => openUrl(b.url)}
            onkeydown={(e) => e.key === 'Enter' && openUrl(b.url)}
            ondragstart={(e) => dragLink(e, b)}
            oncontextmenu={(e) => linkMenu(e, b)}
            {...copyOnHover(() => copyLink(b))}
          >
            <span class="kind-icon"><kind.icon size={22} /></span>
            <span class="card-text">
              <span class="card-title">{b.title}</span>
              <span class="card-host truncate">{hostOf(b.url)}</span>
              {#if b.note}<span class="card-note truncate">{b.note}</span>{/if}
            </span>
            <span class="card-actions">
              <button class="icon-btn" onclick={(e) => { e.stopPropagation(); editLink(b); }} title={t('menu.edit')}><Pencil size={14} /></button>
              <button class="icon-btn danger" onclick={(e) => { e.stopPropagation(); deleteLink(b); }} title={t('menu.delete')}><Trash size={14} /></button>
            </span>
          </div>
        {/each}
      </div>
    {:else if !subfolders.length}
      <div class="card empty">
        <span class="empty-icon"><BookmarkIcon size={30} /></span>
        <h2>{query ? t('bm.noMatch') : t('bm.emptyTitle')}</h2>
        {#if !query}
          <p>{t('bm.emptyBody')}</p>
          <button class="btn primary" onclick={newLink}><Plus size={17} /> {t('bm.addLink')}</button>
        {/if}
      </div>
    {/if}
  </section>
</div>

{#if editingLink}
  <Modal title={editingLink.id ? t('bm.editLink') : t('bm.addLink')} onclose={() => (editingLink = null)} width={560}>
    <div class="field">
      <label for="bm-url">{t('bm.url')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="bm-url" class="input" placeholder={t('bm.urlPlaceholder')} bind:value={editingLink.url} oninput={urlChanged} autofocus
        onkeydown={(e) => e.key === 'Enter' && saveLink()} />
    </div>
    <div class="field">
      <label for="bm-title">{t('bm.title')}</label>
      <input id="bm-title" class="input" placeholder={editingLink.url ? titleFromUrl(normalizeUrl(editingLink.url)) : t('bm.titlePlaceholder')}
        bind:value={editingLink.title} onkeydown={(e) => e.key === 'Enter' && saveLink()} />
    </div>
    <div class="field">
      <span class="label">{t('bm.type')}</span>
      <div class="kinds">
        {#each KINDS as k (k.id)}
          <button type="button" class="kind" class:on={editingLink.kind === k.id} style:--c={k.color}
            onclick={() => { if (editingLink) { editingLink.kind = k.id; kindTouched = true; } }}>
            <span class="kind-dot"><k.icon size={16} /></span>
            <span class="truncate">{t(k.label)}</span>
          </button>
        {/each}
      </div>
      <span class="faint small">{t('bm.typeHint')}</span>
    </div>
    <div class="field">
      <label for="bm-folder">{t('bm.folder')}</label>
      <select id="bm-folder" class="select" bind:value={editingLink.folder_id}>
        <option value={null}>{t('bm.unsorted')}</option>
        {#each flatFolders() as { folder, depth } (folder.id)}
          <option value={folder.id}>{'   '.repeat(depth)}{folder.name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="bm-note">{t('bm.note')}</label>
      <textarea id="bm-note" class="textarea" rows="2" placeholder={t('bm.notePlaceholder')} bind:value={editingLink.note}></textarea>
    </div>
    {#if linkError}<p class="error">{linkError}</p>{/if}
    {#snippet footer()}
      {#if editingLink?.id}
        <ConfirmButton onconfirm={() => { const b = editingLink; editingLink = null; if (b) deleteLink(b); }} />
      {/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingLink = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveLink}>{editingLink?.id ? t('common.save') : t('bm.addLink')}</button>
    {/snippet}
  </Modal>
{/if}

{#if editingFolder}
  <Modal title={editingFolder.id ? t('bm.editFolder') : t('bm.newFolder')} onclose={() => (editingFolder = null)} width={440}>
    <div class="field">
      <label for="bm-fname">{t('bm.folderName')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="bm-fname" class="input" placeholder={t('bm.folderNamePlaceholder')} bind:value={editingFolder.name} autofocus
        onkeydown={(e) => e.key === 'Enter' && saveFolder()} />
    </div>
    <div class="field">
      <label for="bm-fparent">{t('bm.parent')}</label>
      <select id="bm-fparent" class="select" bind:value={editingFolder.parent_id}>
        <option value={null}>{t('bm.topLevel')}</option>
        {#each flatFolders(editingFolder.id ? descendantsOf(editingFolder.id) : new Set()) as { folder, depth } (folder.id)}
          <option value={folder.id}>{'   '.repeat(depth)}{folder.name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <span class="label">{t('common.color')}</span>
      <ColorPicker bind:value={editingFolder.color} />
    </div>
    {#if folderError}<p class="error">{folderError}</p>{/if}
    {#if editingFolder.id}<p class="faint small">{t('bm.folderDeleteHint')}</p>{/if}
    {#snippet footer()}
      {#if editingFolder?.id}<ConfirmButton onconfirm={deleteFolder} question={t('bm.deleteFolderQ')} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingFolder = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveFolder}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .bookmarks {
    display: grid;
    grid-template-columns: 250px minmax(0, 1fr);
    height: 100%;
  }
  .side {
    padding: 28px 14px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }
  .side h1 {
    padding: 0 10px 18px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .folders-label {
    padding: 18px 12px 8px;
  }
  .tree-row {
    display: flex;
    align-items: center;
    padding-left: calc(var(--depth) * 16px);
  }
  .twisty {
    width: 20px;
    height: 20px;
    flex: none;
    display: grid;
    place-items: center;
    border: none;
    background: none;
    color: var(--faint);
    border-radius: 6px;
    cursor: pointer;
    transition: transform 0.15s;
  }
  .twisty.open {
    transform: rotate(90deg);
  }
  .folder {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    height: 36px;
    padding: 0 10px;
    border: 1.5px solid transparent;
    border-radius: 11px;
    background: transparent;
    color: var(--muted);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .folder.top {
    padding-left: 12px;
  }
  .folder:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .folder.active {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .folder.drop,
  .subfolder.drop {
    border-color: var(--primary);
    border-style: dashed;
    background: var(--primary-soft);
  }
  .folder.add {
    color: var(--faint);
    padding-left: 12px;
  }
  .count {
    margin-left: auto;
    font-size: 12px;
    font-weight: 700;
    color: var(--faint);
  }
  .main {
    padding: 28px 32px;
    overflow-y: auto;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: flex-end;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 18px;
  }
  .title-block {
    min-width: 0;
  }
  .crumbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    color: var(--faint);
    font-size: 12.5px;
  }
  .crumbs button {
    border: none;
    background: none;
    padding: 0;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .crumbs button:hover {
    color: var(--primary);
  }
  .heading {
    font-size: 22px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--faint);
  }
  .search :global(svg) {
    position: absolute;
    left: 12px;
  }
  .search .input {
    width: 220px;
    padding-left: 36px;
    border-radius: var(--radius);
  }
  .subfolders {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 10px;
    margin-bottom: 16px;
  }
  .subfolder {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-weight: 650;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .subfolder:hover {
    border-color: var(--c);
  }
  .sub-icon {
    display: grid;
    color: var(--c);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(250px, 100%), 1fr));
    gap: 14px;
  }
  .link-card {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: 20px;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    transition: transform 0.12s, box-shadow 0.15s, border-color 0.15s;
    min-width: 0;
    outline: none;
  }
  .link-card:hover,
  .link-card:focus-visible {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
    border-color: color-mix(in srgb, var(--c) 45%, var(--border));
  }
  .kind-icon {
    width: 46px;
    height: 46px;
    flex: none;
    border-radius: 15px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .card-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .card-title {
    font-weight: 700;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .card-host {
    font-size: 12.5px;
    color: var(--muted);
  }
  .card-note {
    font-size: 12.5px;
    color: var(--faint);
  }
  .card-actions {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 10px;
    background: var(--surface);
    opacity: 0;
    transition: opacity 0.12s;
  }
  .link-card:hover .card-actions {
    opacity: 1;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 6px;
  }
  .kind {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 40px;
    padding: 0 10px;
    border: 1.5px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .kind:hover {
    border-color: var(--c);
  }
  .kind.on {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 10%, var(--surface));
  }
  .kind-dot {
    display: grid;
    flex: none;
    color: var(--c);
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
  @container main (max-width: 760px) {
    .bookmarks {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto minmax(0, 1fr);
    }
    .side {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 26px 14px 10px;
      border-right: none;
      border-bottom: 1px solid var(--border);
      overflow-x: auto;
    }
    .side h1,
    .folders-label,
    .folder.add {
      display: none;
    }
    .list {
      flex-direction: row;
    }
    .tree-row {
      padding-left: 0;
    }
    .twisty {
      display: none;
    }
    .folder {
      white-space: nowrap;
    }
    .main {
      padding: 18px 14px;
    }
    .search .input {
      width: 160px;
    }
  }
</style>
