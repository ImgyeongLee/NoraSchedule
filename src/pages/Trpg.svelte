<script lang="ts">
  // TRPG log: sessions (want to play / played), rule books and scenario books (shelf or list), and a report.
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    BookOpen, ChartPie, ChevronDown, ExternalLink, Library, Link, Pencil, Plus, ScrollText, Search, Sparkles, Swords, Trash, X,
  } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import DateField from '../components/DateField.svelte';
  import ImagePicker from '../components/ImagePicker.svelte';
  import TrpgReport from '../components/TrpgReport.svelte';
  import Bookshelf from '../components/Bookshelf.svelte';
  import BookViewToggle from '../components/BookViewToggle.svelte';
  import { bookViews } from '../lib/bookView.svelte';
  import { api, type TrpgEntry, type TrpgKind } from '../lib/api';
  import { fmt, today } from '../lib/dates';
  import { t, type Key } from '../lib/i18n.svelte';
  import { openMenu, type MenuItem } from '../lib/menu.svelte';
  import { hideHoverCard, showNoteCard } from '../lib/hovercard.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';

  type Tab = 'sessions' | 'rulebook' | 'scenario_book' | 'report';
  type Sort = 'date' | 'title';

  const TABS: { id: Tab; label: Key; icon: typeof BookOpen }[] = [
    { id: 'sessions', label: 'trpg.sessions', icon: ScrollText },
    { id: 'rulebook', label: 'trpg.rulebooks', icon: BookOpen },
    { id: 'scenario_book', label: 'trpg.scenarioBooks', icon: Library },
    { id: 'report', label: 'trpg.report', icon: ChartPie },
  ];
  const isBook = (kind: TrpgKind) => kind === 'rulebook' || kind === 'scenario_book';


  // Per-viewer conveniences, remembered in this browser only.
  const PREFS_KEY = 'nora.trpg';
  /** How the Played column is grouped: by year, or by character pair. */
  type PlayedGroup = 'date' | 'pair';
  function readPrefs(): { tab?: Tab; sort?: Sort; collapsed?: string[]; playedGroup?: PlayedGroup } {
    try {
      return JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') ?? {};
    } catch {
      return {};
    }
  }
  const prefs = readPrefs();

  let entries = $state<TrpgEntry[]>([]);
  let tab = $state<Tab>(TABS.some((x) => x.id === prefs.tab) ? prefs.tab! : 'sessions');
  let sort = $state<Sort>(prefs.sort === 'title' ? 'title' : 'date');
  /** Groups folded away in the Played column (years, or "pair:<name>"). */
  let collapsed = $state<string[]>(Array.isArray(prefs.collapsed) ? prefs.collapsed : []);
  let playedGroup = $state<PlayedGroup>(prefs.playedGroup === 'pair' ? 'pair' : 'date');
  let query = $state('');
  let editing = $state<TrpgEntry | null>(null);
  let newLink = $state('');
  let error = $state('');

  $effect(() => {
    const value = JSON.stringify({ tab, sort, collapsed, playedGroup });
    try {
      localStorage.setItem(PREFS_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });

  $effect(() => {
    data.version;
    load(api.trpgEntries(), []).then((e) => (entries = e));
  });

  /** ㄱ–ㅎ for Korean titles, A–Z for the rest; "Scenario 2" before "Scenario 10". */
  const collator = new Intl.Collator('ko', { numeric: true, sensitivity: 'base' });
  const byTitle = (a: TrpgEntry, b: TrpgEntry) => collator.compare(a.title, b.title);
  const byDate = (a: TrpgEntry, b: TrpgEntry) => (b.date ?? '').localeCompare(a.date ?? '') || b.created_at - a.created_at;

  const matching = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => [e.title, e.writer, e.system, e.memo].some((s) => s.toLowerCase().includes(q)));
  });
  const of = (kind: TrpgKind) => matching.filter((e) => e.kind === kind);

  const wishlist = $derived(
    of('wishlist').sort(sort === 'title' ? byTitle : (a, b) => b.created_at - a.created_at),
  );
  const played = $derived(of('played'));
  /** Played sessions by year, newest year first; undated ones last. */
  const playedByYear = $derived.by(() => {
    const groups = new Map<string, TrpgEntry[]>();
    for (const e of played) {
      const year = e.date?.slice(0, 4) ?? '';
      groups.set(year, [...(groups.get(year) ?? []), e]);
    }
    return [...groups.entries()]
      .sort(([a], [b]) => (a === '' ? 1 : b === '' ? -1 : b.localeCompare(a)))
      .map(([year, list]) => [year, list.sort(sort === 'title' ? byTitle : byDate)] as const);
  });
  /** Played sessions by character pair, the most played pair first; sessions without one last ("Other"). */
  const playedByPair = $derived.by(() => {
    const groups = new Map<string, TrpgEntry[]>();
    for (const e of played) {
      const pair = e.pair.trim();
      groups.set(pair, [...(groups.get(pair) ?? []), e]);
    }
    return [...groups.entries()]
      .sort(([a, la], [b, lb]) => (a === '' ? 1 : b === '' ? -1 : lb.length - la.length || collator.compare(a, b)))
      .map(([pair, list]) => [pair, list.sort(sort === 'title' ? byTitle : byDate)] as const);
  });
  /** The Played column's groups for the chosen grouping. */
  const playedGroups = $derived(
    playedGroup === 'pair'
      ? playedByPair.map(([pair, list]) => ({ key: `pair:${pair}`, label: pair || t('trpg.pairOther'), list }))
      : playedByYear.map(([year, list]) => ({ key: year, label: year ? t('trpg.year', { year }) : t('trpg.noDate'), list })),
  );
  /** Character pairs used so far, offered as suggestions in the editor. */
  const pairs = $derived([...new Set(entries.map((e) => e.pair.trim()).filter(Boolean))].sort(collator.compare));
  const books = $derived(tab === 'rulebook' || tab === 'scenario_book' ? of(tab).sort(byTitle) : []);
  /** Rule systems used so far, offered as suggestions in the editor. */
  const systems = $derived([...new Set(entries.map((e) => e.system).filter(Boolean))].sort(collator.compare));

  function toggleYear(year: string) {
    collapsed = collapsed.includes(year) ? collapsed.filter((y) => y !== year) : [...collapsed, year];
  }

  function blank(kind: TrpgKind): TrpgEntry {
    return {
      id: 0, kind, title: '', writer: '', system: '', links: [], image: null,
      date: kind === 'played' ? today() : null, role: '', pair: '', memo: '', created_at: 0,
    };
  }

  /** Details line for the memo hover card: date, writer, system and role. */
  const noteMeta = (e: TrpgEntry) =>
    [e.date && e.kind === 'played' ? fmt(e.date, { year: 'numeric', month: 'short', day: 'numeric' }) : '', e.pair, e.writer, e.system, e.role.toUpperCase()]
      .filter(Boolean)
      .join(' · ');

  function edit(e: TrpgEntry) {
    error = '';
    newLink = '';
    editing = { ...e, links: [...e.links] };
  }

  /** Moves a wished-for scenario to the played list, dated today. */
  function markPlayed(e: TrpgEntry) {
    edit({ ...e, kind: 'played', date: e.date ?? today() });
  }

  /** "example.com/x" → "https://example.com/x"; links with a scheme are kept. */
  const normalizeLink = (s: string) => {
    const link = s.trim();
    return link && !/^[a-z][a-z0-9+.-]*:/i.test(link) ? `https://${link}` : link;
  };

  function addLink() {
    if (!editing || !newLink.trim()) return;
    editing.links = [...editing.links, normalizeLink(newLink)];
    newLink = '';
  }

  /** Closing without saving: drop a cover that was uploaded but not kept. */
  function cancel() {
    const uploaded = editing?.image && editing.image !== entries.find((x) => x.id === editing?.id)?.image;
    editing = null;
    if (uploaded) api.removeUnusedImages().catch(() => {});
  }

  async function save() {
    if (!editing) return;
    if (!editing.title.trim()) return (error = t('trpg.needTitle'));
    // A link typed but not added yet still counts.
    const links = [...editing.links, ...(newLink.trim() ? [normalizeLink(newLink)] : [])];
    const entry: TrpgEntry = {
      ...editing,
      links,
      // Only played sessions keep a date and a role; only books keep a cover.
      date: editing.kind === 'played' ? editing.date : null,
      role: editing.kind === 'played' ? editing.role : '',
      pair: editing.kind === 'played' ? editing.pair.trim() : '',
      image: isBook(editing.kind) ? editing.image : null,
    };
    const ok = await mutate(api.saveTrpgEntry(entry), editing.id ? t('trpg.saved') : t('trpg.added'));
    if (ok !== undefined) {
      if (isBook(entry.kind)) tab = entry.kind as Tab;
      editing = null;
    }
  }

  async function remove(e: TrpgEntry) {
    editing = null;
    try {
      const stored = await api.deleteTrpgEntry(e.id);
      data.version++;
      toast(t('clip.deleted', { name: e.title }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveTrpgEntry({ ...stored, id: 0 }), t('trpg.saved')),
      });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function open(link: string) {
    openUrl(link).catch((err) => toast(String(err), 'error'));
  }

  const linkItems = (e: TrpgEntry): MenuItem[] => e.links.map((l) => ({ label: l, icon: ExternalLink, action: () => open(l) }));

  /** One link opens directly; several open a menu to pick from. */
  function openLinks(ev: MouseEvent, e: TrpgEntry) {
    if (e.links.length === 1) open(e.links[0]);
    else openMenu(ev, linkItems(e));
  }

  function entryMenu(ev: MouseEvent, e: TrpgEntry) {
    const links = linkItems(e);
    openMenu(ev, [
      { label: t('common.edit'), icon: Pencil, action: () => edit(e) },
      ...(e.kind === 'wishlist' ? [{ label: t('trpg.markPlayed'), icon: Swords, action: () => markPlayed(e) }] : []),
      ...(links.length ? ['separator' as const, ...links] : []),
      'separator',
      { label: t('common.delete'), icon: Trash, danger: true, action: () => remove(e) },
    ]);
  }
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.trpg')}</h1>
      <p class="sub">{t('trpg.subtitle')}</p>
    </div>
  </div>

  <div class="toolbar">
    <div class="segmented tabs">
      {#each TABS as x (x.id)}
        <button class:active={tab === x.id} onclick={() => (tab = x.id)}><x.icon size={15} /> {t(x.label)}</button>
      {/each}
    </div>
    <span class="spacer"></span>
    {#if tab !== 'report'}
      {#if tab === 'sessions'}
        <div class="segmented">
          <button class:active={sort === 'date'} onclick={() => (sort = 'date')}>{t('trpg.sortDate')}</button>
          <button class:active={sort === 'title'} onclick={() => (sort = 'title')}>{t('trpg.sortTitle')}</button>
        </div>
      {/if}
      <label class="search">
        <Search size={15} />
        <input class="input" placeholder={t('trpg.search')} bind:value={query} />
      </label>
      {#if tab !== 'sessions'}
        <BookViewToggle page="trpg" />
        <button class="btn primary" onclick={() => edit(blank(tab as TrpgKind))}><Plus size={16} /> {t(`trpg.add.${tab as 'rulebook' | 'scenario_book'}`)}</button>
      {/if}
    {/if}
  </div>

  {#if tab === 'sessions'}
    <div class="columns">
      <section class="card column" style:--c="#f2668b">
        <div class="col-head">
          <span class="col-icon"><Sparkles size={16} /></span>
          <h3>{t('trpg.wishlist')}</h3>
          <span class="count">{wishlist.length}</span>
          <span class="spacer"></span>
          <button class="icon-btn" onclick={() => edit(blank('wishlist'))} title={t('trpg.add.wishlist')} aria-label={t('trpg.add.wishlist')}><Plus size={17} /></button>
        </div>
        {#each wishlist as e (e.id)}{@render session(e)}{:else}{@render empty('wishlist')}{/each}
      </section>

      <section class="card column" style:--c="#7c74ff">
        <div class="col-head">
          <span class="col-icon"><ScrollText size={16} /></span>
          <h3>{t('trpg.played')}</h3>
          <span class="count">{played.length}</span>
          <span class="spacer"></span>
          <div class="segmented small-seg" role="group" aria-label={t('trpg.groupBy')}>
            <button class:active={playedGroup === 'date'} onclick={() => (playedGroup = 'date')}>{t('trpg.groupDate')}</button>
            <button class:active={playedGroup === 'pair'} onclick={() => (playedGroup = 'pair')}>{t('trpg.groupPair')}</button>
          </div>
          <button class="icon-btn" onclick={() => edit(blank('played'))} title={t('trpg.add.played')} aria-label={t('trpg.add.played')}><Plus size={17} /></button>
        </div>
        {#each playedGroups as group (group.key)}
          {@const list = group.list}
          {@const expanded = !collapsed.includes(group.key)}
          <button class="year-head" onclick={() => toggleYear(group.key)} aria-expanded={expanded}>
            <span class="chev" class:closed={!expanded}><ChevronDown size={15} /></span>
            <span class="strong truncate">{group.label}</span>
            <span class="spacer"></span>
            <span class="muted small">{t('trpg.times', { n: list.length })}</span>
          </button>
          {#if expanded}
            {#each list as e (e.id)}{@render session(e)}{/each}
          {/if}
        {:else}
          {@render empty('played')}
        {/each}
      </section>
    </div>
  {:else if tab === 'report'}
    <TrpgReport {entries} />
  {:else}
    <div class="card shelf-card">
      {#if books.length}
        <Bookshelf items={books} view={bookViews.trpg} author={(b) => b.writer} onopen={edit} onmenu={entryMenu}>
          {#snippet meta(b)}{[b.system, b.writer].filter(Boolean).join(' · ')}{/snippet}
          {#snippet trailing(b)}
            {#if b.links.length}<span class="faint small row-links"><ExternalLink size={13} /> {b.links.length}</span>{/if}
          {/snippet}
        </Bookshelf>
      {:else}
        {@render empty(tab as TrpgKind)}
      {/if}
    </div>
  {/if}
</div>

{#snippet session(e: TrpgEntry)}
  <div
    class="entry"
    oncontextmenu={(ev) => { hideHoverCard(); entryMenu(ev, e); }}
    onmouseenter={(ev) => showNoteCard(ev.currentTarget, { title: e.title, meta: noteMeta(e), memo: e.memo })}
    onmouseleave={hideHoverCard}
    onpointerdown={hideHoverCard}
    role="listitem"
  >
    <button class="entry-main" onclick={() => edit(e)}>
      {#if e.kind === 'played'}
        <span class="when tabular">{e.date ? fmt(e.date, { month: '2-digit', day: '2-digit' }) : '—'}</span>
      {/if}
      <span class="entry-text">
        <span class="entry-title truncate">{e.title}</span>
        {#if e.writer || e.system}<span class="muted small truncate">{[e.writer, e.system].filter(Boolean).join(' · ')}</span>{/if}
      </span>
      {#if e.role}<span class="role {e.role}">{e.role.toUpperCase()}</span>{/if}
    </button>
    {#if e.kind === 'wishlist'}
      <button class="icon-btn" onclick={() => markPlayed(e)} title={t('trpg.markPlayedHint')} aria-label={t('trpg.markPlayed')}><Swords size={15} /></button>
    {/if}
    {#if e.links.length}
      <button class="icon-btn links" onclick={(ev) => openLinks(ev, e)} title={e.links.join('\n')} aria-label={t('trpg.openLink')}>
        <ExternalLink size={15} />{#if e.links.length > 1}<span class="link-count">{e.links.length}</span>{/if}
      </button>
    {/if}
  </div>
{/snippet}

{#snippet empty(kind: TrpgKind)}
  <div class="empty small-empty">
    {#if query.trim()}
      <p>{t('trpg.noMatch')}</p>
    {:else}
      <p>{t(`trpg.empty.${kind}`)}</p>
      <button class="btn small" onclick={() => edit(blank(kind))}><Plus size={14} /> {t(`trpg.add.${kind}`)}</button>
    {/if}
  </div>
{/snippet}

{#if editing}
  <Modal title={editing.id ? t('trpg.edit') : t(`trpg.add.${editing.kind}`)} onclose={cancel} width={520}>
    <div class="field">
      <span class="label">{t('trpg.list')}</span>
      <div class="segmented kind-pick">
        {#each (isBook(editing.kind) ? ['rulebook', 'scenario_book'] : ['wishlist', 'played']) as TrpgKind[] as k (k)}
          <button type="button" class:active={editing.kind === k} onclick={() => editing && (editing.kind = k)}>{t(`trpg.kind.${k}`)}</button>
        {/each}
      </div>
    </div>
    <div class="field">
      <label for="trpg-title">{t('trpg.title')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="trpg-title" class="input" placeholder={t(isBook(editing.kind) ? 'trpg.bookPlaceholder' : 'trpg.titlePlaceholder')}
        bind:value={editing.title} autofocus onkeydown={(e) => e.key === 'Enter' && save()} />
    </div>
    <div class="two">
      <div class="field">
        <label for="trpg-writer">{isBook(editing.kind) ? t('trpg.author') : t('trpg.writer')}</label>
        <input id="trpg-writer" class="input" bind:value={editing.writer} />
      </div>
      <div class="field">
        <label for="trpg-system">{t('trpg.system')}</label>
        <input id="trpg-system" class="input" list="trpg-systems" placeholder={t('trpg.systemPlaceholder')} bind:value={editing.system} />
        <datalist id="trpg-systems">{#each systems as s (s)}<option value={s}></option>{/each}</datalist>
      </div>
    </div>
    {#if editing.kind === 'played'}
      <div class="two">
        <div class="field">
          <label for="trpg-date">{t('trpg.date')}</label>
          <DateField id="trpg-date" value={editing.date} clearable onchange={(v) => editing && (editing.date = v)} />
        </div>
        <div class="field">
          <span class="label">{t('trpg.role')}</span>
          <div class="segmented">
            {#each [['', t('trpg.roleNone')], ['gm', 'GM'], ['pl', 'PL']] as [id, label] (id)}
              <button type="button" class:active={editing.role === id} onclick={() => editing && (editing.role = id as TrpgEntry['role'])}>{label}</button>
            {/each}
          </div>
        </div>
      </div>
      <div class="field">
        <label for="trpg-pair">{t('trpg.pair')}</label>
        <input id="trpg-pair" class="input" list="trpg-pairs" placeholder={t('trpg.pairPlaceholder')} bind:value={editing.pair} />
        <datalist id="trpg-pairs">{#each pairs as p (p)}<option value={p}></option>{/each}</datalist>
      </div>
    {/if}
    {#if isBook(editing.kind)}
      <div class="field">
        <span class="label">{t('trpg.cover')}</span>
        <ImagePicker bind:value={editing.image} />
      </div>
    {/if}
    <div class="field">
      <label for="trpg-link">{t('common.links')}</label>
      {#each editing.links as link, i (i)}
        <div class="link-row">
          <Link size={14} />
          <button class="link-text truncate" onclick={() => open(link)} title={link}>{link}</button>
          <button class="icon-btn" onclick={() => editing?.links.splice(i, 1)} aria-label={t('event.removeLink')}><X size={14} /></button>
        </div>
      {/each}
      <div class="row">
        <input id="trpg-link" class="input" placeholder={t('trpg.linkPlaceholder')} bind:value={newLink}
          onkeydown={(e) => e.key === 'Enter' && addLink()} />
        <button class="btn" onclick={addLink} disabled={!newLink.trim()}><Plus size={16} /> {t('common.add')}</button>
      </div>
    </div>
    <div class="field">
      <label for="trpg-memo">{t('common.memo')}</label>
      <textarea id="trpg-memo" class="textarea" rows="3" placeholder={t('trpg.memoPlaceholder')} bind:value={editing.memo}></textarea>
    </div>
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
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 220px;
    color: var(--faint);
  }
  .search .input {
    height: 34px;
  }

  /* ---- sessions */
  .columns {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
    align-items: start;
  }
  .col-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .col-icon {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .count {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 700;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 12%, transparent);
  }
  .small-seg {
    padding: 2px;
  }
  .small-seg button {
    height: 24px;
    padding: 0 9px;
    font-size: 12px;
  }
  .year-head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 12px 6px 6px;
    border: none;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
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
  .entry {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: 12px;
  }
  .entry:hover {
    background: var(--surface-2);
  }
  .entry-main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 8px 6px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .entry > .icon-btn:last-child {
    margin-right: 4px;
  }
  .when {
    flex: none;
    width: 52px;
    padding: 6px 0;
    border-radius: 10px;
    text-align: center;
    font-size: 13px;
    font-weight: 700;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 12%, transparent);
  }
  .entry-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .entry-title {
    font-weight: 650;
  }
  .role {
    flex: none;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.04em;
  }
  .role.gm {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 15%, transparent);
  }
  .role.pl {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 15%, transparent);
  }
  .links {
    position: relative;
  }
  .link-count {
    position: absolute;
    top: 0;
    right: 0;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 800;
    line-height: 15px;
    color: var(--primary-text);
    background: var(--primary);
  }
  .small-empty {
    padding: 28px 12px;
    gap: 8px;
  }

  .shelf-card {
    padding: 0;
    overflow: hidden;
  }
  .row-links {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  /* ---- editor */
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .kind-pick {
    align-self: flex-start;
  }
  .link-row {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .link-text {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    color: var(--primary);
    text-align: left;
    cursor: pointer;
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
  @container main (max-width: 820px) {
    .columns {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @container main (max-width: 560px) {
    .two {
      grid-template-columns: 1fr;
    }
    .search {
      width: 100%;
    }
  }
</style>
