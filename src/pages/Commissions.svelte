<script lang="ts">
  // Commissions (외주): a board with one column per stage (drag cards along), the money received
  // and still owed, deadlines, and the clients behind them.
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    ArrowRight, Brush, CalendarClock, ChevronsLeftRight, ChevronsRightLeft, Contact, ExternalLink, HandCoins, Link,
    Minus, Pencil, Plus, ReceiptText, Search, SquareKanban, Trash, Users, Wallet, X,
  } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import DateField from '../components/DateField.svelte';
  import ImagePicker from '../components/ImagePicker.svelte';
  import CommissionLedger from '../components/CommissionLedger.svelte';
  import StatCard from '../components/StatCard.svelte';
  import { api, type Commission, type CommissionClient, type CommissionStatus } from '../lib/api';
  import {
    KIND_PRESETS, PLATFORM_PRESETS, STAGES, byDeadline, byDelivered, commissionFocus, daysLeft, feeOf, netOf, netPayment, nextStage, paidOf,
    payState, stageOf, unpaidOf,
  } from '../lib/commissions.svelte';
  import { fmt, today } from '../lib/dates';
  import { currencySymbol, expenseSettings, fmtMoney, loadExpenseSettings } from '../lib/expenses.svelte';
  import { imageUrl } from '../lib/images';
  import { t } from '../lib/i18n.svelte';
  import { openMenu, type MenuItem } from '../lib/menu.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';

  type Tab = 'board' | 'ledger' | 'clients';
  // Per-viewer conveniences, remembered in this browser only.
  const PREFS_KEY = 'nora.commissions';
  const ALL_STAGES = STAGES.map((s) => s.id);
  function readPrefs(): { tab?: Tab; stages?: CommissionStatus[]; folded?: CommissionStatus[] } {
    try {
      return JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') ?? {};
    } catch {
      return {};
    }
  }
  const prefs = readPrefs();
  const stageList = (v: unknown) => (Array.isArray(v) ? ALL_STAGES.filter((id) => v.includes(id)) : null);
  let tab = $state<Tab>(prefs.tab === 'ledger' || prefs.tab === 'clients' ? prefs.tab : 'board');
  /** Stages shown on the board (at least one). */
  let shownStages = $state<CommissionStatus[]>(stageList(prefs.stages)?.length ? stageList(prefs.stages)! : [...ALL_STAGES]);
  /** Columns folded into a narrow strip. */
  let folded = $state<CommissionStatus[]>(stageList(prefs.folded) ?? []);
  $effect(() => {
    const value = JSON.stringify({ tab, stages: shownStages, folded });
    try {
      localStorage.setItem(PREFS_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });

  function toggleStage(id: CommissionStatus) {
    if (shownStages.includes(id)) {
      if (shownStages.length > 1) shownStages = shownStages.filter((x) => x !== id);
    } else {
      shownStages = ALL_STAGES.filter((x) => x === id || shownStages.includes(x));
    }
  }
  const toggleFold = (id: CommissionStatus) => (folded = folded.includes(id) ? folded.filter((x) => x !== id) : [...folded, id]);

  let commissions = $state<Commission[]>([]);
  let clients = $state<CommissionClient[]>([]);
  let query = $state('');

  onMount(() => {
    if (!expenseSettings.loaded) loadExpenseSettings();
  });

  $effect(() => {
    data.version;
    load(api.commissions(), []).then((c) => {
      commissions = c;
      // Opened from the calendar or the Overview tile.
      const focus = c.find((x) => x.id === commissionFocus.id);
      commissionFocus.id = null;
      if (focus) edit(focus);
    });
    load(api.commissionClients(), []).then((c) => (clients = c));
  });

  const clientById = $derived(new Map(clients.map((c) => [c.id, c])));
  const clientName = (c: Commission) => (c.client_id !== null ? (clientById.get(c.client_id)?.name ?? '') : '');

  const matching = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commissions;
    return commissions.filter((c) => [c.title, clientName(c), c.kind, c.memo].some((s) => s.toLowerCase().includes(q)));
  });
  const columns = $derived(
    STAGES.filter((s) => shownStages.includes(s.id)).map((s) => ({
      ...s,
      folded: folded.includes(s.id),
      items: matching.filter((c) => c.status === s.id).sort(s.id === 'done' ? byDelivered : byDeadline),
    })),
  );
  /** Folded columns take a narrow strip; the others share the rest. */
  const boardColumns = $derived(columns.map((c) => (c.folded ? '52px' : 'minmax(210px, 1fr)')).join(' '));

  // ---- numbers at the top
  const todayStr = $derived(today());
  const thisMonth = $derived(todayStr.slice(0, 7));
  const open = $derived(commissions.filter((c) => c.status !== 'inquiry' && c.status !== 'done'));
  const inquiries = $derived(commissions.filter((c) => c.status === 'inquiry').length);
  const monthPayments = $derived(
    commissions.flatMap((c) => c.payments.map((p) => ({ ...p, net: netPayment(c, p.amount) }))).filter((p) => p.date.startsWith(thisMonth)),
  );
  /** What actually came in this month, after platform fees. */
  const monthIncome = $derived(monthPayments.reduce((s, p) => s + p.net, 0));
  /** Accepted work only: an inquiry has no agreed price yet. */
  const owing = $derived(commissions.filter((c) => c.status !== 'inquiry' && unpaidOf(c) > 0));
  const unpaidTotal = $derived(owing.reduce((s, c) => s + unpaidOf(c), 0));
  const nextDue = $derived(open.filter((c) => c.deadline).sort(byDeadline)[0]);

  /** "D-3", "Due today", "2 days late". */
  function dueLabel(c: Commission) {
    const n = daysLeft(c, todayStr);
    if (n === null) return '';
    return n === 0 ? t('cm.dueToday') : n > 0 ? `D-${n}` : t('cm.overdue', { n: -n });
  }
  const dueTone = (c: Commission) => {
    const n = daysLeft(c, todayStr);
    return n === null ? '' : n < 0 ? 'late' : n <= 3 ? 'soon' : '';
  };

  // ---- drag a card onto another column to move it to that stage
  let dragId = $state<number | null>(null);
  let dropStage = $state<CommissionStatus | null>(null);

  function dragStart(e: DragEvent, c: Commission) {
    e.dataTransfer?.setData('application/x-nora-commission', String(c.id));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    dragId = c.id;
  }
  function dragEnd() {
    dragId = null;
    dropStage = null;
  }
  function dropOn(e: DragEvent, stage: CommissionStatus) {
    e.preventDefault();
    const c = commissions.find((x) => x.id === dragId);
    dragEnd();
    if (c && c.status !== stage) moveTo(c, stage);
  }

  /** Changes the stage. Shown right away; the database catches up in the background. */
  function moveTo(c: Commission, stage: CommissionStatus) {
    const moved: Commission = { ...c, status: stage, delivered: stage === 'done' ? (c.delivered ?? todayStr) : null };
    commissions = commissions.map((x) => (x.id === c.id ? moved : x));
    mutate(api.saveCommission(moved), t('cm.movedTo', { name: c.title, stage: t(stageOf(stage).label) }));
  }

  function payRest(c: Commission) {
    const rest = unpaidOf(c);
    if (rest <= 0) return;
    mutate(api.saveCommission({ ...c, payments: [...c.payments, { amount: rest, date: todayStr }] }), t('cm.paidToast'));
  }

  async function remove(c: Commission) {
    editing = null;
    try {
      const stored = await api.deleteCommission(c.id);
      data.version++;
      toast(t('clip.deleted', { name: c.title }), 'info', {
        label: t('common.undo'),
        run: () => stored && mutate(api.saveCommission({ ...stored, id: 0 }), t('cm.saved')),
      });
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  const openLink = (link: string) => openUrl(link).catch((err) => toast(String(err), 'error'));

  function cardMenu(ev: MouseEvent, c: Commission) {
    const next = nextStage(c.status);
    openMenu(ev, [
      { label: t('common.edit'), icon: Pencil, action: () => edit(c) },
      ...(c.status !== 'done' ? [{ label: t('cm.moveNext', { stage: t(stageOf(next).label) }), icon: ArrowRight, action: () => moveTo(c, next) }] : []),
      ...(unpaidOf(c) > 0 ? [{ label: t('cm.payRest'), icon: HandCoins, action: () => payRest(c) }] : []),
      ...(c.links.length ? (['separator', ...c.links.map((l): MenuItem => ({ label: l, icon: ExternalLink, action: () => openLink(l) }))] as MenuItem[]) : []),
      'separator',
      { label: t('common.delete'), icon: Trash, danger: true, action: () => remove(c) },
    ]);
  }

  // ---- editor
  let editing = $state<Commission | null>(null);
  let clientText = $state('');
  let newLink = $state('');
  let payAmount = $state('');
  let payDate = $state(today());
  let error = $state('');

  function blank(status: CommissionStatus = 'inquiry'): Commission {
    return {
      id: 0, title: '', client_id: null, kind: '', status, price: 0, payments: [], deadline: null, revisions: 0, links: [], image: null,
      memo: '', created_at: 0, delivered: null, platform: '', fee_kind: 'percent', fee: 0,
    };
  }

  function edit(c: Commission) {
    error = '';
    newLink = '';
    payAmount = '';
    payDate = today();
    clientText = clientName(c);
    editing = { ...c, payments: [...c.payments], links: [...c.links] };
  }

  /** Platforms used so far plus common ones, offered while typing. */
  const platforms = $derived([...new Set([...commissions.map((c) => c.platform).filter(Boolean), ...PLATFORM_PRESETS])]);

  /** Picking a platform used before brings back its fee (each site takes its own cut). */
  function setPlatform(name: string) {
    if (!editing) return;
    editing.platform = name;
    const before = commissions.filter((c) => c.platform.trim() === name.trim() && c.id !== editing!.id && c.fee > 0).sort((a, b) => b.created_at - a.created_at)[0];
    if (before && !(Number(editing.fee) > 0)) {
      editing.fee = before.fee;
      editing.fee_kind = before.fee_kind;
    }
  }

  /** Kinds used so far plus the common ones, offered while typing. */
  const kinds = $derived([...new Set([...commissions.map((c) => c.kind).filter(Boolean), ...KIND_PRESETS])]);

  const normalizeLink = (s: string) => {
    const link = s.trim();
    return link && !/^[a-z][a-z0-9+.-]*:/i.test(link) ? `https://${link}` : link;
  };
  function addLink() {
    if (!editing || !newLink.trim()) return;
    editing.links = [...editing.links, normalizeLink(newLink)];
    newLink = '';
  }

  function addPayment(amount = Number(payAmount)) {
    if (!editing) return;
    if (!(amount > 0)) return toast(t('ex.invalidAmount'), 'error');
    editing.payments = [...editing.payments, { amount, date: payDate || today() }].sort((a, b) => a.date.localeCompare(b.date));
    payAmount = '';
  }

  const setRevisions = (n: number) => editing && (editing.revisions = Math.min(999, Math.max(0, Math.round(n) || 0)));

  /** Closing without saving: drop an image that was uploaded but not kept. */
  function cancel() {
    const uploaded = editing?.image && editing.image !== commissions.find((x) => x.id === editing?.id)?.image;
    editing = null;
    if (uploaded) api.removeUnusedImages().catch(() => {});
  }

  /** The client typed in the editor: an existing one (by name), a new one, or none. */
  async function resolveClient(): Promise<number | null | undefined> {
    const name = clientText.trim();
    if (!name) return null;
    const known = clients.find((c) => c.name.trim().toLowerCase() === name.toLowerCase());
    if (known) return known.id;
    try {
      return await api.saveCommissionClient({ id: 0, name, contact: '', memo: '' });
    } catch (err) {
      toast(String(err), 'error');
      return undefined;
    }
  }

  async function save() {
    if (!editing) return;
    if (!editing.title.trim()) return (error = t('cm.needTitle'));
    const client_id = await resolveClient();
    if (client_id === undefined || !editing) return;
    const links = [...editing.links, ...(newLink.trim() ? [normalizeLink(newLink)] : [])];
    const ok = await mutate(
      api.saveCommission({ ...editing, title: editing.title.trim(), client_id, links, price: Number(editing.price) || 0 }),
      editing.id ? t('cm.saved') : t('cm.added'),
    );
    if (ok !== undefined) editing = null;
  }

  // ---- clients
  const clientStats = $derived(
    clients.map((cl) => {
      const works = commissions.filter((c) => c.client_id === cl.id);
      return {
        client: cl,
        works: [...works].sort((a, b) => b.created_at - a.created_at),
        paid: works.reduce((s, c) => s + paidOf(c), 0),
        unpaid: works.filter((c) => c.status !== 'inquiry').reduce((s, c) => s + unpaidOf(c), 0),
      };
    }),
  );
  const shownClients = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? clientStats.filter((x) => [x.client.name, x.client.contact, x.client.memo].some((s) => s.toLowerCase().includes(q))) : clientStats;
  });

  let editingClient = $state<CommissionClient | null>(null);
  let clientError = $state('');
  const editingClientWorks = $derived(editingClient ? commissions.filter((c) => c.client_id === editingClient!.id) : []);

  function editClient(c: CommissionClient) {
    clientError = '';
    editingClient = { ...c };
  }

  async function saveClient() {
    if (!editingClient) return;
    if (!editingClient.name.trim()) return (clientError = t('cm.clientNeedName'));
    const ok = await mutate(api.saveCommissionClient(editingClient), t('cm.clientSaved'));
    if (ok !== undefined) editingClient = null;
  }

  async function removeClient(c: CommissionClient) {
    editingClient = null;
    await mutate(api.deleteCommissionClient(c.id), t('cm.clientDeleted'));
  }

  function newCommissionFor(c: CommissionClient) {
    editingClient = null;
    edit(blank());
    clientText = c.name;
  }

  const PAY_LABEL = { free: 'cm.pay.free', unpaid: 'cm.pay.unpaid', partial: 'cm.pay.partial', paid: 'cm.pay.paid' } as const;
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.commissions')}</h1>
      <p class="sub">{t('cm.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <button class="btn primary" onclick={() => edit(blank())}><Plus size={17} /> {t('cm.new')}</button>
  </div>

  <div class="stats">
    <StatCard label={t('cm.statActive')} value={String(open.length)} sub={inquiries ? t('cm.statInquiries', { n: inquiries }) : ''} icon={Brush} />
    <StatCard
      label={t('cm.statIncome')}
      value={fmtMoney(monthIncome)}
      sub={monthPayments.length ? t('cm.statPaymentsNet', { n: monthPayments.length }) : ''}
      icon={Wallet}
      tint="var(--success)"
    />
    <StatCard
      label={t('cm.statUnpaid')}
      value={fmtMoney(unpaidTotal)}
      sub={owing.length ? t('cm.statCount', { n: owing.length }) : ''}
      icon={HandCoins}
      tint={unpaidTotal > 0 ? 'var(--warning)' : 'var(--success)'}
    />
    <StatCard
      label={t('cm.statNext')}
      value={nextDue ? dueLabel(nextDue) : '—'}
      sub={nextDue ? nextDue.title : ''}
      icon={CalendarClock}
      tint={nextDue && dueTone(nextDue) === 'late' ? 'var(--danger)' : 'var(--long-break)'}
    />
  </div>

  <div class="toolbar">
    <div class="segmented">
      <button class:active={tab === 'board'} onclick={() => (tab = 'board')}><SquareKanban size={15} /> {t('cm.board')}</button>
      <button class:active={tab === 'ledger'} onclick={() => (tab = 'ledger')}><ReceiptText size={15} /> {t('cm.ledger')}</button>
      <button class:active={tab === 'clients'} onclick={() => (tab = 'clients')}><Users size={15} /> {t('cm.clients')}</button>
    </div>
    {#if tab === 'board'}
      <div class="stage-filter" role="group" aria-label={t('cm.showStages')}>
        {#each STAGES as s (s.id)}
          <button
            type="button"
            class="stage-chip"
            class:on={shownStages.includes(s.id)}
            style:--c={s.color}
            aria-pressed={shownStages.includes(s.id)}
            title={t('cm.showStage', { stage: t(s.label) })}
            onclick={() => toggleStage(s.id)}
          ><span class="dot"></span>{t(s.label)}</button>
        {/each}
      </div>
    {/if}
    <span class="spacer"></span>
    <label class="search">
      <Search size={15} />
      <input class="input" placeholder={t(tab === 'clients' ? 'cm.searchClients' : 'cm.search')} bind:value={query} />
    </label>
    {#if tab === 'clients'}
      <button class="btn" onclick={() => editClient({ id: 0, name: '', contact: '', memo: '' })}><Plus size={16} /> {t('cm.newClient')}</button>
    {/if}
  </div>

  {#if tab === 'board'}
    {#if commissions.length === 0}
      <div class="card empty">
        <span class="empty-icon"><Brush size={30} /></span>
        <h2>{t('cm.empty')}</h2>
        <p>{t('cm.emptyBody')}</p>
        <button class="btn primary" onclick={() => edit(blank())}><Plus size={17} /> {t('cm.new')}</button>
      </div>
    {:else}
      <div class="board" style:grid-template-columns={boardColumns}>
        {#each columns as col (col.id)}
          <section
            class="column"
            class:folded={col.folded}
            class:drop={dropStage === col.id}
            style:--c={col.color}
            ondragover={(e) => { if (dragId === null) return; e.preventDefault(); dropStage = col.id; }}
            ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) dropStage = null; }}
            ondrop={(e) => dropOn(e, col.id)}
            aria-label={t(col.label)}
          >
            {#if col.folded}
              <!-- Folded: a narrow strip; click to unfold. Cards can still be dropped on it. -->
              <button class="fold-strip" onclick={() => toggleFold(col.id)} title={t('cm.unfold')} aria-label={t('cm.unfold')}>
                <span class="col-icon"><col.icon size={15} /></span>
                <span class="count">{col.items.length}</span>
                <span class="strip-label strong">{t(col.label)}</span>
                <ChevronsLeftRight size={15} />
              </button>
            {:else}
            <div class="col-head">
              <span class="col-icon"><col.icon size={15} /></span>
              <span class="strong truncate">{t(col.label)}</span>
              <span class="count">{col.items.length}</span>
              <span class="spacer"></span>
              <button class="icon-btn small-btn" onclick={() => toggleFold(col.id)} title={t('cm.fold')} aria-label={t('cm.fold')}><ChevronsRightLeft size={15} /></button>
              <button class="icon-btn small-btn" onclick={() => edit(blank(col.id))} title={t('cm.new')} aria-label={t('cm.new')}><Plus size={15} /></button>
            </div>
            {#each col.items as c (c.id)}
              {@const pay = payState(c)}
              <div
                class="cm-card"
                class:dragging={dragId === c.id}
                role="button"
                tabindex="0"
                draggable="true"
                ondragstart={(e) => dragStart(e, c)}
                ondragend={dragEnd}
                onclick={() => edit(c)}
                onkeydown={(e) => e.key === 'Enter' && edit(c)}
                oncontextmenu={(e) => cardMenu(e, c)}
              >
                {#if c.image}<img class="thumb" src={imageUrl(c.image, 'commission')} alt="" draggable="false" />{/if}
                <span class="cm-title">{c.title}</span>
                {#if clientName(c) || c.kind}
                  <span class="muted small truncate">{[clientName(c), c.kind].filter(Boolean).join(' · ')}</span>
                {/if}
                <span class="badges">
                  {#if dueLabel(c)}<span class="badge due {dueTone(c)}"><CalendarClock size={11} /> {dueLabel(c)}</span>{/if}
                  {#if c.price > 0}<span class="badge price-tag">{fmtMoney(c.price)}</span>{/if}
                  {#if pay !== 'free'}<span class="badge pay {pay}">{t(PAY_LABEL[pay])}</span>{/if}
                  {#if c.revisions > 0}<span class="badge">{t('cm.revisionsN', { n: c.revisions })}</span>{/if}
                </span>
              </div>
            {:else}
              <p class="faint small col-empty">{query.trim() ? t('trpg.noMatch') : t('cm.emptyColumn')}</p>
            {/each}
            {/if}
          </section>
        {/each}
      </div>
    {/if}
  {:else if tab === 'ledger'}
    <CommissionLedger commissions={matching} {clientName} onopen={edit} />
  {:else if shownClients.length}
    <div class="clients">
      {#each shownClients as x (x.client.id)}
        <button class="card client" onclick={() => editClient(x.client)}>
          <span class="client-head">
            <span class="avatar">{x.client.name.slice(0, 1)}</span>
            <span class="client-name">
              <span class="strong truncate">{x.client.name}</span>
              {#if x.client.contact}<span class="muted small truncate">{x.client.contact}</span>{/if}
            </span>
          </span>
          <span class="client-nums small">
            <span>{t('cm.clientWorks', { n: x.works.length })}</span>
            <span class="paid">{t('cm.clientPaid', { amount: fmtMoney(x.paid) })}</span>
            {#if x.unpaid > 0}<span class="owes">{t('cm.clientUnpaid', { amount: fmtMoney(x.unpaid) })}</span>{/if}
          </span>
          {#if x.works.length}
            <span class="client-works">
              {#each x.works.slice(0, 3) as c (c.id)}
                <span class="work" style:--c={stageOf(c.status).color}><span class="dot"></span><span class="truncate">{c.title}</span></span>
              {/each}
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {:else}
    <div class="card empty">
      <span class="empty-icon"><Contact size={30} /></span>
      <p>{query.trim() ? t('trpg.noMatch') : t('cm.clientEmpty')}</p>
    </div>
  {/if}
</div>

{#if editing}
  {@const paid = paidOf(editing)}
  {@const rest = Math.max(0, (Number(editing.price) || 0) - paid)}
  <Modal title={editing.id ? t('cm.edit') : t('cm.new')} onclose={cancel} width={580}>
    <div class="field">
      <label for="cm-title">{t('cm.title')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="cm-title" class="input" placeholder={t('cm.titlePlaceholder')} bind:value={editing.title} autofocus
        onkeydown={(e) => e.key === 'Enter' && save()} />
    </div>
    <div class="two">
      <div class="field">
        <label for="cm-client">{t('cm.client')}</label>
        <input id="cm-client" class="input" list="cm-clients" placeholder={t('cm.clientPlaceholder')} bind:value={clientText} />
        <datalist id="cm-clients">{#each clients as c (c.id)}<option value={c.name}></option>{/each}</datalist>
      </div>
      <div class="field">
        <label for="cm-kind">{t('cm.kind')}</label>
        <input id="cm-kind" class="input" list="cm-kinds" placeholder={t('cm.kindPlaceholder')} bind:value={editing.kind} />
        <datalist id="cm-kinds">{#each kinds as k (k)}<option value={k}></option>{/each}</datalist>
      </div>
    </div>
    <div class="field">
      <span class="label">{t('cm.stage')}</span>
      <div class="segmented stages">
        {#each STAGES as s (s.id)}
          <button type="button" class:active={editing.status === s.id} style:--c={s.color} onclick={() => editing && (editing.status = s.id)}>
            <s.icon size={14} /> {t(s.label)}
          </button>
        {/each}
      </div>
    </div>
    <div class="two">
      <div class="field">
        <label for="cm-deadline">{t('cm.deadline')}</label>
        <DateField id="cm-deadline" value={editing.deadline} clearable onchange={(v) => editing && (editing.deadline = v)} />
      </div>
      <div class="field">
        <label for="cm-revisions">{t('cm.revisions')}</label>
        <div class="stepper">
          <button type="button" onclick={() => editing && setRevisions(editing.revisions - 1)} disabled={editing.revisions <= 0} aria-label={t('pomo.decrease')}><Minus size={14} /></button>
          <input id="cm-revisions" class="tabular" type="number" min="0" max="999" value={editing.revisions} oninput={(e) => setRevisions(Number(e.currentTarget.value))} />
          <span class="unit">{t('trpg.playUnit')}</span>
          <button type="button" onclick={() => editing && setRevisions(editing.revisions + 1)} aria-label={t('pomo.increase')}><Plus size={14} /></button>
        </div>
      </div>
    </div>

    <div class="money-box">
      <div class="field">
        <label for="cm-price">{t('cm.price')}</label>
        <div class="money">
          <span class="symbol">{currencySymbol()}</span>
          <input id="cm-price" class="tabular" type="number" inputmode="decimal" min="0" step="any" placeholder="0" bind:value={editing.price} />
        </div>
      </div>
      <div class="fee-row">
        <div class="field">
          <label for="cm-platform">{t('cm.platform')}</label>
          <input id="cm-platform" class="input" list="cm-platforms" placeholder={t('cm.platformPlaceholder')} value={editing.platform}
            oninput={(e) => setPlatform(e.currentTarget.value)} />
          <datalist id="cm-platforms">{#each platforms as p (p)}<option value={p}></option>{/each}</datalist>
        </div>
        <div class="field">
          <label for="cm-fee">{t('cm.fee')}</label>
          <div class="fee-input">
            <div class="money small-money">
              {#if editing.fee_kind === 'amount'}<span class="symbol">{currencySymbol()}</span>{/if}
              <input id="cm-fee" class="tabular" type="number" inputmode="decimal" min="0" step="any" placeholder="0" bind:value={editing.fee} />
              {#if editing.fee_kind === 'percent'}<span class="symbol">%</span>{/if}
            </div>
            <div class="segmented fee-kind">
              <button type="button" class:active={editing.fee_kind === 'percent'} onclick={() => editing && (editing.fee_kind = 'percent')}>%</button>
              <button type="button" class:active={editing.fee_kind === 'amount'} onclick={() => editing && (editing.fee_kind = 'amount')}>{currencySymbol()}</button>
            </div>
          </div>
        </div>
      </div>
      {#if Number(editing.price) > 0}
        <div class="net small">
          <span class="muted">{t('cm.netLabel')}</span>
          <span class="net-amount tabular">{fmtMoney(netOf(editing))}</span>
          {#if feeOf(editing) > 0}<span class="faint">{t('cm.feeAmount', { amount: fmtMoney(feeOf(editing)) })}</span>{/if}
        </div>
      {/if}
      <div class="field">
        <span class="label">{t('cm.payments')}</span>
        {#each editing.payments as p, i (i)}
          <div class="pay-row">
            <span class="muted small">{fmt(p.date, { year: 'numeric', month: 'short', day: 'numeric' })}</span>
            <span class="strong tabular">{fmtMoney(p.amount)}</span>
            <span class="spacer"></span>
            <button class="icon-btn" onclick={() => editing && (editing.payments = editing.payments.filter((_, k) => k !== i))} aria-label={t('common.delete')}><X size={14} /></button>
          </div>
        {:else}
          <p class="faint small none">{t('cm.noPayments')}</p>
        {/each}
        <div class="pay-add">
          <div class="pay-date"><DateField value={payDate} label={t('common.date')} onchange={(v) => v && (payDate = v)} /></div>
          <div class="money small-money">
            <span class="symbol">{currencySymbol()}</span>
            <input class="tabular" type="number" inputmode="decimal" min="0" step="any" placeholder="0" bind:value={payAmount}
              onkeydown={(e) => e.key === 'Enter' && addPayment()} aria-label={t('ex.amount')} />
          </div>
          <button class="btn" onclick={() => addPayment()}><Plus size={15} /> {t('cm.addPayment')}</button>
        </div>
        {#if Number(editing.price) > 0}
          <div class="pay-summary small">
            <span class="muted">{t('cm.paidOf', { paid: fmtMoney(paid), price: fmtMoney(Number(editing.price)) })}</span>
            {#if rest > 0}
              <span class="owes strong">{t('cm.left', { amount: fmtMoney(rest) })}</span>
              <button class="btn small" onclick={() => addPayment(rest)}><HandCoins size={14} /> {t('cm.payRest')}</button>
            {:else}
              <span class="paid strong">{t('cm.pay.paid')}</span>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="field">
      <span class="label">{t('cm.image')}</span>
      <ImagePicker bind:value={editing.image} />
    </div>
    <div class="field">
      <span class="label">{t('common.links')}</span>
      {#each editing.links as l, i (i)}
        <div class="link-row">
          <Link size={14} />
          <button class="link-text truncate" onclick={() => openLink(l)}>{l}</button>
          <button class="icon-btn" onclick={() => editing && (editing.links = editing.links.filter((_, k) => k !== i))} aria-label={t('event.removeLink')}><X size={14} /></button>
        </div>
      {/each}
      <div class="link-add">
        <input class="input" placeholder={t('cm.linkPlaceholder')} bind:value={newLink} onkeydown={(e) => e.key === 'Enter' && addLink()} />
        <button class="btn" onclick={addLink}><Plus size={15} /> {t('common.add')}</button>
      </div>
    </div>
    <div class="field">
      <label for="cm-memo">{t('common.memo')}</label>
      <textarea id="cm-memo" class="textarea" rows="3" placeholder={t('cm.memoPlaceholder')} bind:value={editing.memo}></textarea>
    </div>
    {#if error}<p class="error small">{error}</p>{/if}
    {#snippet footer()}
      {#if editing?.id}<ConfirmButton onconfirm={() => editing && remove(editing)} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={cancel}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={save}>{editing?.id ? t('common.save') : t('common.add')}</button>
    {/snippet}
  </Modal>
{/if}

{#if editingClient}
  <Modal title={editingClient.id ? t('cm.editClient') : t('cm.newClient')} onclose={() => (editingClient = null)} width={480}>
    <div class="field">
      <label for="cl-name">{t('common.name')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="cl-name" class="input" bind:value={editingClient.name} autofocus onkeydown={(e) => e.key === 'Enter' && saveClient()} />
    </div>
    <div class="field">
      <label for="cl-contact">{t('cm.contact')}</label>
      <input id="cl-contact" class="input" placeholder={t('cm.contactPlaceholder')} bind:value={editingClient.contact} />
    </div>
    <div class="field">
      <label for="cl-memo">{t('common.memo')}</label>
      <textarea id="cl-memo" class="textarea" rows="3" placeholder={t('cm.clientMemoPlaceholder')} bind:value={editingClient.memo}></textarea>
    </div>
    {#if editingClient.id}
      <div class="field">
        <span class="label">{t('cm.clientHistory')}</span>
        {#each editingClientWorks as c (c.id)}
          <button class="history-row" style:--c={stageOf(c.status).color} onclick={() => { editingClient = null; edit(c); }}>
            <span class="dot"></span>
            <span class="truncate strong">{c.title}</span>
            <span class="spacer"></span>
            <span class="muted small">{t(stageOf(c.status).label)}</span>
            {#if c.price > 0}<span class="small tabular">{fmtMoney(c.price)}</span>{/if}
          </button>
        {:else}
          <p class="faint small none">{t('cm.noWorks')}</p>
        {/each}
        <button class="btn small add-for" onclick={() => editingClient && newCommissionFor(editingClient)}><Plus size={14} /> {t('cm.new')}</button>
      </div>
    {/if}
    {#if clientError}<p class="error small">{clientError}</p>{/if}
    {#snippet footer()}
      {#if editingClient?.id}<ConfirmButton question={t('cm.deleteClientQ')} onconfirm={() => editingClient && removeClient(editingClient)} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingClient = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveClient}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .strong {
    font-weight: 650;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr));
    gap: 14px;
    margin-bottom: 16px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 14px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    width: min(280px, 100%);
  }
  .search :global(svg) {
    position: absolute;
    left: 12px;
    color: var(--muted);
  }
  .search .input {
    padding-left: 34px;
  }

  .stage-filter {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .stage-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 11px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--faint);
    font-size: 12.5px;
    font-weight: 650;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s, color 0.12s;
  }
  .stage-chip .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c);
    opacity: 0.35;
  }
  .stage-chip.on {
    border-color: color-mix(in srgb, var(--c) 60%, var(--border));
    background: color-mix(in srgb, var(--c) 10%, var(--surface));
    color: var(--text);
  }
  .stage-chip.on .dot {
    opacity: 1;
  }

  /* ---- board */
  .board {
    display: grid;
    gap: 12px;
    align-items: start;
    overflow-x: auto;
    padding-bottom: 6px;
  }
  .column {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 140px;
    padding: 10px;
    border: 1.5px solid transparent;
    border-radius: var(--radius-lg);
    background: var(--surface-2);
    transition: border-color 0.12s, background 0.12s;
  }
  .column.folded {
    padding: 6px;
  }
  .fold-strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 220px;
    padding: 6px 0;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .fold-strip:hover {
    background: var(--surface);
  }
  .strip-label {
    writing-mode: vertical-rl;
    color: var(--text);
    letter-spacing: 0.04em;
  }
  .column.drop {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 8%, var(--surface-2));
  }
  .col-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 2px 4px;
  }
  .col-icon {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
  }
  .count {
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    border-radius: var(--radius-pill);
    display: grid;
    place-items: center;
    font-size: 11.5px;
    font-weight: 700;
    color: var(--muted);
    background: var(--surface);
  }
  .small-btn {
    width: 28px;
    height: 28px;
  }
  .col-empty {
    margin: 0;
    padding: 14px 6px;
    text-align: center;
    border: 1.5px dashed var(--border);
    border-radius: var(--radius);
  }
  .cm-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 11px 12px;
    border: 1px solid var(--border);
    border-left: 3px solid var(--c);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    cursor: grab;
    transition: transform 0.12s, box-shadow 0.12s;
  }
  .cm-card:hover {
    transform: translateY(-1px);
    box-shadow: var(--shadow);
  }
  .cm-card:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 1px;
  }
  .cm-card.dragging {
    opacity: 0.4;
  }
  .thumb {
    width: 100%;
    height: 96px;
    margin-bottom: 4px;
    object-fit: cover;
    border-radius: 8px;
  }
  .cm-title {
    font-weight: 650;
    overflow-wrap: anywhere;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 20px;
    padding: 0 7px;
    border-radius: var(--radius-pill);
    font-size: 11px;
    font-weight: 650;
    color: var(--muted);
    background: var(--surface-2);
    white-space: nowrap;
  }
  .badge.due.soon {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 14%, transparent);
  }
  .badge.due.late {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
  .badge.price-tag {
    color: var(--text);
  }
  .badge.pay.paid {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 13%, transparent);
  }
  .badge.pay.partial {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 14%, transparent);
  }

  /* ---- clients */
  .clients {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(260px, 100%), 1fr));
    gap: 14px;
  }
  .client {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .client:hover {
    border-color: color-mix(in srgb, var(--primary) 40%, var(--border));
  }
  .client-head {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .avatar {
    width: 38px;
    height: 38px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-weight: 750;
    color: var(--primary);
    background: var(--primary-soft);
  }
  .client-name {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .client-nums {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    color: var(--muted);
  }
  .paid {
    color: var(--success);
  }
  .owes {
    color: var(--warning);
  }
  .client-works {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .work,
  .history-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 13px;
  }
  .work .dot,
  .history-row .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }
  .history-row {
    width: 100%;
    padding: 7px 8px;
    border: none;
    border-radius: 10px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .history-row:hover {
    background: var(--surface-2);
  }
  .add-for {
    align-self: flex-start;
    margin-top: 4px;
  }

  /* ---- editor */
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 12px;
  }
  .stages {
    display: flex;
    flex-wrap: wrap;
  }
  .stages button {
    padding: 0 10px;
  }
  .stages button.active :global(svg) {
    color: var(--c);
  }
  .stepper {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 40px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .stepper:focus-within {
    border-color: var(--primary);
  }
  .stepper button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .stepper button:hover:not(:disabled) {
    background: var(--surface);
    color: var(--text);
  }
  .stepper button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .stepper input {
    width: 44px;
    border: none;
    background: none;
    outline: none;
    font-size: 15px;
    font-weight: 700;
    text-align: center;
    color: var(--text);
  }
  .stepper .unit {
    margin-right: 4px;
    font-size: 13px;
    color: var(--muted);
  }
  input[type='number']::-webkit-inner-spin-button,
  input[type='number']::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .money-box {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .money {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 44px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .money:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 18%, transparent);
  }
  .money input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    outline: none;
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
  }
  .symbol {
    font-weight: 700;
    color: var(--muted);
  }
  .small-money {
    height: 40px;
    flex: 1;
    min-width: 110px;
  }
  .small-money input {
    font-size: 14px;
  }
  .fee-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 12px;
  }
  .fee-row .input {
    background: var(--surface);
  }
  .fee-input {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .fee-kind button {
    min-width: 34px;
    padding: 0 8px;
  }
  .net {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 6px 10px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--surface);
  }
  .net-amount {
    font-size: 16px;
    font-weight: 750;
    color: var(--success);
  }
  .pay-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 4px 4px 10px;
    border-radius: 10px;
    background: var(--surface);
  }
  .pay-add {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .pay-add > .btn {
    height: 40px;
  }
  .pay-date {
    width: 200px;
  }
  .pay-date :global(.date-field) {
    background: var(--surface);
  }
  .pay-summary {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 12px;
  }
  .none {
    margin: 0;
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
    padding: 0;
    border: none;
    background: none;
    color: var(--primary);
    text-align: left;
    cursor: pointer;
  }
  .link-add {
    display: flex;
    gap: 8px;
  }
  .link-add .input {
    flex: 1;
  }
  .error {
    margin: 0;
    color: var(--danger);
  }
  @container main (max-width: 640px) {
    .two {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
