<script lang="ts">
  // Reading log: books being read (with progress), books to read, and books read with stars and a review.
  import { BookCheck, BookOpenText, BookPlus, ChevronDown, Pencil, Play, Plus, Search, Trash } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import DateField from '../components/DateField.svelte';
  import ImagePicker from '../components/ImagePicker.svelte';
  import BookCover from '../components/BookCover.svelte';
  import Bookshelf from '../components/Bookshelf.svelte';
  import BookViewToggle from '../components/BookViewToggle.svelte';
  import StarRating from '../components/StarRating.svelte';
  import { api, type Book, type BookStatus } from '../lib/api';
  import { bookViews } from '../lib/bookView.svelte';
  import { diffDays, fmt, today } from '../lib/dates';
  import { t, type Key } from '../lib/i18n.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';

  const loadRichEditor = () => import('../components/RichEditor.svelte');

  type Sort = 'recent' | 'title' | 'rating';
  const TABS: { id: BookStatus; label: Key; icon: typeof BookOpenText }[] = [
    { id: 'reading', label: 'read.reading', icon: BookOpenText },
    { id: 'want', label: 'read.want', icon: BookPlus },
    { id: 'read', label: 'read.read', icon: BookCheck },
  ];

  // Per-viewer conveniences, remembered in this browser only.
  const PREFS_KEY = 'nora.reading';
  function readPrefs(): { tab?: BookStatus; sort?: Sort; collapsed?: string[] } {
    try {
      return JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') ?? {};
    } catch {
      return {};
    }
  }
  const prefs = readPrefs();

  let books = $state<Book[]>([]);
  let tab = $state<BookStatus>(TABS.some((x) => x.id === prefs.tab) ? prefs.tab! : 'reading');
  let sort = $state<Sort>(prefs.sort === 'title' || prefs.sort === 'rating' ? prefs.sort : 'recent');
  /** Years folded away in the Read list. */
  let collapsed = $state<string[]>(Array.isArray(prefs.collapsed) ? prefs.collapsed : []);
  let query = $state('');
  let editing = $state<Book | null>(null);
  let error = $state('');
  /** Page numbers typed into the Reading cards, by book id, until saved. */
  let pageDraft = $state<Record<number, string>>({});

  $effect(() => {
    const value = JSON.stringify({ tab, sort, collapsed });
    try {
      localStorage.setItem(PREFS_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });

  $effect(() => {
    data.version;
    load(api.books(), []).then((b) => (books = b));
  });

  /** ㄱ–ㅎ for Korean titles, A–Z for the rest. */
  const collator = new Intl.Collator('ko', { numeric: true, sensitivity: 'base' });
  const SORTS: Record<Sort, (a: Book, b: Book) => number> = {
    recent: (a, b) => (b.finished ?? '').localeCompare(a.finished ?? '') || b.created_at - a.created_at,
    title: (a, b) => collator.compare(a.title, b.title),
    rating: (a, b) => b.rating - a.rating || collator.compare(a.title, b.title),
  };

  const matching = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return books;
    return books.filter((b) => [b.title, b.author, b.publisher, b.review].some((s) => s.toLowerCase().includes(q)));
  });
  const countOf = (status: BookStatus) => books.filter((b) => b.status === status).length;
  const reading = $derived(matching.filter((b) => b.status === 'reading').sort((a, b) => progress(b) - progress(a)));
  const wanted = $derived(matching.filter((b) => b.status === 'want').sort(SORTS[sort === 'rating' ? 'title' : sort]));
  /** Read books by the year they were finished, newest first; undated ones last. */
  const readByYear = $derived.by(() => {
    const groups = new Map<string, Book[]>();
    for (const b of matching.filter((x) => x.status === 'read')) {
      const year = b.finished?.slice(0, 4) ?? '';
      groups.set(year, [...(groups.get(year) ?? []), b]);
    }
    return [...groups.entries()]
      .sort(([a], [b]) => (a === '' ? 1 : b === '' ? -1 : b.localeCompare(a)))
      .map(([year, list]) => [year, list.sort(SORTS[sort])] as const);
  });

  const thisYear = today().slice(0, 4);
  const readThisYear = $derived(books.filter((b) => b.status === 'read' && b.finished?.startsWith(thisYear)).length);
  const rated = $derived(books.filter((b) => b.status === 'read' && b.rating > 0));
  const averageRating = $derived(rated.length ? rated.reduce((s, b) => s + b.rating, 0) / rated.length / 2 : 0);

  /** 0–1, or 0 when the page count is unknown. */
  const progress = (b: Book) => (b.total_pages > 0 ? Math.min(1, b.current_page / b.total_pages) : 0);
  const pct = (b: Book) => `${Math.round(progress(b) * 100)}%`;

  function toggleYear(year: string) {
    collapsed = collapsed.includes(year) ? collapsed.filter((y) => y !== year) : [...collapsed, year];
  }

  function blank(status: BookStatus): Book {
    return {
      id: 0, title: '', author: '', publisher: '', status, image: null, total_pages: 0, current_page: 0, rating: 0,
      started: status === 'reading' ? today() : null, finished: status === 'read' ? today() : null, review: '', created_at: 0,
    };
  }

  function edit(b: Book) {
    error = '';
    editing = { ...b };
  }

  /** Fills in what a status change implies: start date, finish date, last page. */
  function withStatus(b: Book, status: BookStatus): Book {
    const next = { ...b, status };
    if (status !== 'want' && !next.started) next.started = today();
    if (status === 'read') {
      next.finished ??= today();
      if (next.total_pages > 0) next.current_page = next.total_pages;
    } else {
      next.finished = null;
    }
    return next;
  }

  function setStatus(status: BookStatus) {
    if (editing) editing = withStatus(editing, status);
  }

  /** Finishing a book opens the editor so the rating and review can be added right away. */
  function finish(b: Book) {
    edit(withStatus(b, 'read'));
  }

  async function startReading(b: Book) {
    await mutate(api.saveBook(withStatus(b, 'reading')), t('read.started'));
    tab = 'reading';
  }

  async function savePage(b: Book) {
    const raw = pageDraft[b.id];
    delete pageDraft[b.id];
    if (raw === undefined || raw.trim() === '') return;
    const page = Math.max(0, Math.floor(Number(raw)));
    if (!Number.isFinite(page) || page === b.current_page) return;
    const updated = { ...b, current_page: b.total_pages ? Math.min(page, b.total_pages) : page };
    await mutate(api.saveBook(updated));
    if (b.total_pages && page >= b.total_pages) toast(t('read.reachedEnd'), 'success', { label: t('read.finish'), run: () => finish(updated) });
  }

  /** Closing without saving: drop a cover that was uploaded but not kept. */
  function cancel() {
    const uploaded = editing?.image && editing.image !== books.find((x) => x.id === editing?.id)?.image;
    editing = null;
    if (uploaded) api.removeUnusedImages().catch(() => {});
  }

  async function save() {
    if (!editing) return;
    if (!editing.title.trim()) return (error = t('read.needTitle'));
    const total = Math.max(0, Math.floor(Number(editing.total_pages) || 0));
    const current = Math.max(0, Math.floor(Number(editing.current_page) || 0));
    const book: Book = { ...editing, total_pages: total, current_page: total ? Math.min(current, total) : current };
    const ok = await mutate(api.saveBook(book), editing.id ? t('read.saved') : t('read.added'));
    if (ok !== undefined) {
      tab = book.status;
      editing = null;
    }
  }

  async function remove(b: Book) {
    editing = null;
    try {
      const stored = await api.deleteBook(b.id);
      data.version++;
      toast(t('clip.deleted', { name: b.title }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveBook({ ...stored, id: 0 }), t('read.saved')),
      });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function bookMenu(ev: MouseEvent, b: Book) {
    openMenu(ev, [
      { label: t('common.edit'), icon: Pencil, action: () => edit(b) },
      ...(b.status === 'want' ? [{ label: t('read.start'), icon: Play, action: () => startReading(b) }] : []),
      ...(b.status === 'reading' ? [{ label: t('read.finish'), icon: BookCheck, action: () => finish(b) }] : []),
      'separator',
      { label: t('common.delete'), icon: Trash, danger: true, action: () => remove(b) },
    ]);
  }

  /** "Day 8" of reading, counting the start day as day 1. */
  const readingDays = (b: Book) => (b.started ? diffDays(today(), b.started) + 1 : 0);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.reading')}</h1>
      <p class="sub">{t('read.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <div class="summary">
      <span><strong class="tabular">{readThisYear}</strong> {t('read.thisYear')}</span>
      {#if rated.length}<span class="avg"><StarRating value={Math.round(averageRating * 2)} size={14} /> <strong>{averageRating.toFixed(1)}</strong></span>{/if}
    </div>
    <button class="btn primary" onclick={() => edit(blank(tab))}><Plus size={17} /> {t('read.add')}</button>
  </div>

  <div class="toolbar">
    <div class="segmented tabs">
      {#each TABS as x (x.id)}
        <button class:active={tab === x.id} onclick={() => (tab = x.id)}>
          <x.icon size={15} /> {t(x.label)} <span class="tab-count">{countOf(x.id)}</span>
        </button>
      {/each}
    </div>
    <span class="spacer"></span>
    {#if tab !== 'reading'}
      <div class="segmented">
        <button class:active={sort === 'recent'} onclick={() => (sort = 'recent')}>{t(tab === 'read' ? 'read.sortRecent' : 'read.sortAdded')}</button>
        <button class:active={sort === 'title'} onclick={() => (sort = 'title')}>{t('read.sortTitle')}</button>
        {#if tab === 'read'}<button class:active={sort === 'rating'} onclick={() => (sort = 'rating')}>{t('read.sortRating')}</button>{/if}
      </div>
      <BookViewToggle page="reading" />
    {/if}
    <label class="search">
      <Search size={15} />
      <input class="input" placeholder={t('read.search')} bind:value={query} />
    </label>
  </div>

  {#if tab === 'reading'}
    {#if reading.length}
      <div class="reading-grid">
        {#each reading as b (b.id)}
          <div class="card now" oncontextmenu={(ev) => bookMenu(ev, b)} role="group">
            <button class="now-cover" onclick={() => edit(b)} aria-label={t('common.edit')}>
              <BookCover title={b.title} author={b.author} image={b.image} />
            </button>
            <div class="now-body">
              <button class="now-title" onclick={() => edit(b)}>
                <span class="strong two-lines">{b.title}</span>
                {#if b.author}<span class="muted small truncate">{b.author}</span>{/if}
              </button>
              {#if b.total_pages}
                <div class="bar" aria-hidden="true"><span style:width={pct(b)}></span></div>
              {/if}
              <div class="progress-line small">
                <label class="page-input">
                  <input
                    class="input"
                    type="number"
                    min="0"
                    max={b.total_pages || undefined}
                    value={pageDraft[b.id] ?? String(b.current_page)}
                    oninput={(e) => (pageDraft[b.id] = e.currentTarget.value)}
                    onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
                    onblur={() => savePage(b)}
                    aria-label={t('read.currentPage')}
                  />
                  <span class="muted">{b.total_pages ? t('read.ofPages', { n: b.total_pages }) : t('read.pageUnit')}</span>
                </label>
                <span class="spacer"></span>
                {#if b.total_pages}<strong class="tabular">{pct(b)}</strong>{/if}
              </div>
              <div class="now-foot">
                {#if b.started}<span class="faint small">{t('read.dayN', { n: readingDays(b) })}</span>{/if}
                <span class="spacer"></span>
                <button class="btn small" onclick={() => finish(b)}><BookCheck size={14} /> {t('read.finish')}</button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="card">{@render empty('reading')}</div>
    {/if}
  {:else if tab === 'want'}
    <div class="card shelf-card">
      {#if wanted.length}
        <Bookshelf items={wanted} view={bookViews.reading} author={(b) => b.author} onopen={edit} onmenu={bookMenu}>
          {#snippet meta(b)}{b.author}{/snippet}
          {#snippet trailing(b)}
            <button class="btn small ghost" onclick={(e) => { e.stopPropagation(); startReading(b); }}><Play size={14} /> {t('read.start')}</button>
          {/snippet}
        </Bookshelf>
      {:else}
        {@render empty('want')}
      {/if}
    </div>
  {:else}
    {#each readByYear as [year, list] (year)}
      {@const expanded = !collapsed.includes(year)}
      <div class="card shelf-card year-card">
        <button class="year-head" onclick={() => toggleYear(year)} aria-expanded={expanded}>
          <span class="chev" class:closed={!expanded}><ChevronDown size={16} /></span>
          <span class="strong">{year ? t('read.year', { year }) : t('read.noDate')}</span>
          <span class="spacer"></span>
          <span class="muted small">{t('read.booksN', { n: list.length })}</span>
        </button>
        {#if expanded}
          <Bookshelf items={list} view={bookViews.reading} author={(b) => b.author} onopen={edit} onmenu={bookMenu}>
            {#snippet meta(b)}
              {#if b.rating}<StarRating value={b.rating} size={12} />{/if}<span class="truncate">{b.author}</span>
            {/snippet}
            {#snippet trailing(b)}
              {#if b.finished}<span class="faint small">{fmt(b.finished, { month: 'short', day: 'numeric' })}</span>{/if}
            {/snippet}
          </Bookshelf>
        {/if}
      </div>
    {:else}
      <div class="card">{@render empty('read')}</div>
    {/each}
  {/if}
</div>

{#snippet empty(status: BookStatus)}
  <div class="empty small-empty">
    {#if query.trim()}
      <p>{t('read.noMatch')}</p>
    {:else}
      <p>{t(`read.empty.${status}`)}</p>
      <button class="btn small" onclick={() => edit(blank(status))}><Plus size={14} /> {t('read.add')}</button>
    {/if}
  </div>
{/snippet}

{#if editing}
  <Modal title={editing.id ? t('read.edit') : t('read.add')} onclose={cancel} width={640}>
    <div class="editor">
      <div class="side">
        <ImagePicker bind:value={editing.image} />
      </div>
      <div class="main-fields">
        <div class="field">
          <label for="book-title">{t('read.title')}</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input id="book-title" class="input" bind:value={editing.title} autofocus={!editing.id} onkeydown={(e) => e.key === 'Enter' && save()} />
        </div>
        <div class="two">
          <div class="field">
            <label for="book-author">{t('read.author')}</label>
            <input id="book-author" class="input" bind:value={editing.author} />
          </div>
          <div class="field">
            <label for="book-publisher">{t('read.publisher')}</label>
            <input id="book-publisher" class="input" bind:value={editing.publisher} />
          </div>
        </div>
        <div class="field">
          <span class="label">{t('read.status')}</span>
          <div class="segmented status">
            {#each TABS as x (x.id)}
              <button type="button" class:active={editing.status === x.id} onclick={() => setStatus(x.id)}>{t(x.label)}</button>
            {/each}
          </div>
        </div>
      </div>
    </div>

    {#if editing.status !== 'want'}
      <div class="progress-dates">
        <div class="field">
          <label for="book-current">{t('read.progress')}</label>
          <div class="pages">
            <input id="book-current" class="input" type="number" min="0" bind:value={editing.current_page} aria-label={t('read.currentPage')} />
            <span class="muted">/</span>
            <input class="input" type="number" min="0" bind:value={editing.total_pages} placeholder={t('read.totalPages')} aria-label={t('read.totalPages')} />
            <span class="muted">{t('read.pageUnit')}</span>
          </div>
        </div>
        <div class="field">
          <span class="label">{t('read.dates')}</span>
          <div class="dates">
            <DateField value={editing.started} clearable placeholder={t('read.startedOn')} onchange={(v) => editing && (editing.started = v)} />
            {#if editing.status === 'read'}
              <span class="muted">–</span>
              <DateField value={editing.finished} clearable placeholder={t('read.finishedOn')} onchange={(v) => editing && (editing.finished = v)} />
            {/if}
          </div>
        </div>
      </div>
    {/if}

    {#if editing.status === 'read'}
      <div class="field">
        <span class="label">{t('read.rating')}</span>
        <div class="rating-row">
          <StarRating value={editing.rating} size={26} onchange={(v) => editing && (editing.rating = v)} />
          <span class="muted">{editing.rating ? (editing.rating / 2).toFixed(1) : t('read.noRating')}</span>
        </div>
      </div>
    {/if}

    {#if editing.status !== 'want'}
      <div class="field">
        <span class="label">{editing.status === 'read' ? t('read.review') : t('read.notes')}</span>
        <div class="review">
          {#await loadRichEditor() then { default: RichEditor }}
            <RichEditor
              value={editing.review}
              placeholder={t('read.reviewPlaceholder')}
              onchange={(md) => editing && (editing.review = md)}
            />
          {/await}
        </div>
      </div>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}
    {#snippet footer()}
      {#if editing?.id}<ConfirmButton onconfirm={() => editing && remove(editing)} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={cancel}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={save}>{editing?.id ? t('common.save') : t('common.add')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .summary {
    display: flex;
    align-items: center;
    gap: 14px;
    color: var(--muted);
    font-size: 13px;
  }
  .summary strong {
    color: var(--text);
    font-size: 15px;
  }
  .avg {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 16px;
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .tab-count {
    font-size: 11.5px;
    font-weight: 700;
    opacity: 0.6;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 200px;
    color: var(--faint);
  }
  .search .input {
    height: 34px;
  }

  /* ---- reading now */
  .reading-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(340px, 100%), 1fr));
    gap: 16px;
  }
  .now {
    display: flex;
    gap: 16px;
    padding: 16px;
  }
  .now-cover {
    --cover-w: 92px;
    --cover-h: 132px;
    flex: none;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
  }
  .now-cover :global(.cover-title) {
    font-size: 11.5px;
  }
  .now-cover :global(.cover) {
    padding: 10px 8px 8px 12px;
  }
  .now-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .now-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .two-lines {
    font-size: 15px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .bar {
    height: 8px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--primary);
    transition: width 0.3s;
  }
  .progress-line {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .page-input {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .page-input .input {
    width: 76px;
    height: 30px;
    padding: 0 8px;
    text-align: right;
  }
  .now-foot {
    display: flex;
    align-items: center;
    margin-top: auto;
  }

  /* ---- shelves */
  .shelf-card {
    padding: 0;
    overflow: hidden;
  }
  .year-card {
    margin-bottom: 16px;
  }
  .year-head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 14px 18px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .chev {
    display: grid;
    color: var(--muted);
    transition: transform 0.15s;
  }
  .chev.closed {
    transform: rotate(-90deg);
  }
  .year-card:has(.chev.closed) .year-head {
    border-bottom: none;
  }
  .small-empty {
    padding: 32px 12px;
    gap: 8px;
  }

  /* ---- editor */
  .editor {
    display: grid;
    grid-template-columns: 170px minmax(0, 1fr);
    gap: 16px;
  }
  .side :global(.drop),
  .side :global(.preview) {
    min-height: 0;
    height: 220px;
  }
  .side :global(.actions) {
    left: 8px;
    right: 8px;
    flex-direction: column;
  }
  .main-fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .status {
    align-self: flex-start;
  }
  .progress-dates {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .pages,
  .dates {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pages .input {
    width: 78px;
  }
  .dates :global(.date-field) {
    flex: 1;
    min-width: 0;
  }
  .rating-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .review {
    min-height: 160px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    overflow: hidden;
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
  @container main (max-width: 640px) {
    .summary {
      display: none;
    }
    .search {
      width: 100%;
    }
  }
</style>
