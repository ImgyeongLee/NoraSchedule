<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Eye, Hash, NotebookPen, Plus, Search, Type } from '@lucide/svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import { api, type Memo } from '../lib/api';
  import { fmtTs } from '../lib/dates';
  import { data, load, mutate, toast, ui } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  // The rich editor (ProseMirror) is large, so it is only loaded when first used.
  const loadRichEditor = () => import('../components/RichEditor.svelte');

  type Mode = 'rich' | 'markdown' | 'preview';
  const MODES: Mode[] = ['rich', 'markdown', 'preview'];
  let memos = $state<Memo[]>([]);
  let selectedId = $state<number | null>(null);
  let title = $state('');
  let body = $state('');
  let mode = $state<Mode>('rich');
  let search = $state('');
  let dirty = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    data.version;
    load(api.memos(), []).then((m) => {
      memos = m;
      if (selectedId === null && m.length) select(m.find((x) => x.id === ui.memoFocus) ?? m[0]);
      ui.memoFocus = null;
    });
  });

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return memos.filter((m) => {
      const t = m.id === selectedId ? title : m.title;
      const b = m.id === selectedId ? body : m.body;
      return !q || t.toLowerCase().includes(q) || b.toLowerCase().includes(q);
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
  }

  async function create() {
    await flush();
    const id = await mutate(api.createMemo(t('common.untitled')));
    if (id === undefined) return;
    memos = await api.memos();
    const m = memos.find((x) => x.id === id);
    if (m) await select(m);
    if (mode === 'preview') setMode('rich');
  }

  async function remove() {
    if (selectedId === null) return;
    clearTimeout(saveTimer);
    dirty = false;
    const id = selectedId;
    selectedId = null;
    await mutate(api.deleteMemo(id), t('memo.deleted'));
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
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      flush();
    }
  }

  onMount(() => {
    api.getSetting('memo.mode').then((m) => MODES.includes(m as Mode) && (mode = m as Mode)).catch(() => {});
  });

  function setMode(m: Mode) {
    mode = m;
    api.setSetting('memo.mode', m).catch(() => {});
  }

  function richChanged(markdown: string) {
    body = markdown;
    edited();
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
    <div class="search">
      <Search size={16} />
      <input class="input" placeholder={t('memo.search')} bind:value={search} />
    </div>
    <div class="list">
      {#each filtered as m (m.id)}
        <button class="memo-item" class:active={m.id === selectedId} onclick={() => select(m)}>
          <span class="m-title truncate">{(m.id === selectedId ? title : m.title) || t('common.untitled')}</span>
          <span class="m-preview truncate">{preview(m)}</span>
          <span class="m-date">{fmtTs(m.updated_at, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hour12: false })}</span>
        </button>
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
        <input class="title-input" placeholder={t('common.untitled')} bind:value={title} oninput={edited} />
        <div class="segmented">
          <button class:active={mode === 'rich'} onclick={() => setMode('rich')}><Type size={15} /> {t('memo.modeRich')}</button>
          <button class:active={mode === 'markdown'} onclick={() => setMode('markdown')}><Hash size={15} /> {t('memo.modeMarkdown')}</button>
          <button class:active={mode === 'preview'} onclick={() => setMode('preview')}><Eye size={15} /> {t('memo.modePreview')}</button>
        </div>
        <ConfirmButton onconfirm={remove} label="" question={t('memo.deleteQ')} />
      </div>
      <p class="status faint small">{dirty ? t('memo.saving') : t('memo.allSaved')} · {t('memo.words', { n: words })}{mode === 'markdown' ? ` · ${t('memo.markdown')}` : ''}</p>

      <div class="panes" class:split={mode === 'markdown'}>
        {#if mode === 'rich'}
          {#await loadRichEditor() then { default: RichEditor }}
            {#key selectedId}
              <RichEditor value={body} onchange={richChanged} />
            {/key}
          {/await}
        {:else if mode === 'markdown'}
          <textarea
            class="md-input"
            bind:value={body}
            oninput={edited}
            placeholder={t('memo.placeholder')}
          ></textarea>
        {/if}
        {#if mode !== 'rich'}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="markdown prose" onclick={previewClick}>{@html html}</div>
        {/if}
      </div>
    {/if}
  </section>
</div>

<style>
  .memos {
    display: grid;
    grid-template-columns: 290px 1fr;
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
    padding: 0 8px 16px;
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
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .memo-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 14px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .memo-item:hover {
    background: var(--surface-2);
  }
  .memo-item.active {
    background: var(--surface);
    box-shadow: var(--shadow-sm), 0 0 0 1.5px var(--primary);
  }
  .m-title {
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
    gap: 12px;
  }
  .title-input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    outline: none;
    font-size: 26px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }
  .status {
    margin: 4px 0 14px;
  }
  .panes {
    flex: 1;
    display: grid;
    gap: 16px;
    min-height: 0;
  }
  .panes.split {
    grid-template-columns: 1fr 1fr;
  }
  .md-input {
    width: 100%;
    height: 100%;
    resize: none;
    padding: 20px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    font-family: var(--mono);
    font-size: 13.5px;
    line-height: 1.7;
    outline: none;
    box-shadow: var(--shadow-sm);
  }
  .md-input:focus {
    border-color: var(--primary);
  }
  .markdown {
    overflow-y: auto;
    padding: 20px 24px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
    line-height: 1.7;
    user-select: text;
    -webkit-user-select: text;
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
      grid-template-rows: 210px minmax(0, 1fr);
    }
    .list-pane {
      padding: 22px 12px 8px;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .toolbar {
      flex-wrap: wrap;
    }
  }
</style>
