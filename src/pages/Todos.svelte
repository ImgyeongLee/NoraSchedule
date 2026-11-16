<script lang="ts">
  import Select from '../components/Select.svelte';
  import { Columns3, FolderPlus, Inbox, Layers, List, PartyPopper, Pencil, Plus, Repeat, Sparkles, Sun } from '@lucide/svelte';
  import RoutineManager from '../components/RoutineManager.svelte';
  import TodoBoard from '../components/TodoBoard.svelte';
  import TodoItem from './TodoItem.svelte';
  import Modal from '../components/Modal.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import DeadlinePicker from '../components/DeadlinePicker.svelte';
  import { ClipboardPaste } from '@lucide/svelte';
  import { clipboard, pageTarget, pasteAsTodo, pasteOnHover, shortcut } from '../lib/clipboard.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { api, type Todo, type TodoGroup } from '../lib/api';
  import { PALETTE, hex } from '../lib/colors';
  import { today } from '../lib/dates';
  import { data, load, mutate } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  type Filter = 'all' | 'today' | 'ungrouped' | 'routines' | number;
  let filter = $state<Filter>('all');
  // Remembered in this browser, so leaving the page keeps finished to-dos hidden (or shown).
  const SHOW_DONE_KEY = 'nora.todos.showDone';
  function readShowDone() {
    try {
      return localStorage.getItem(SHOW_DONE_KEY) !== 'false';
    } catch {
      return true;
    }
  }
  let showDone = $state(readShowDone());
  $effect(() => {
    const value = showDone;
    try {
      localStorage.setItem(SHOW_DONE_KEY, String(value));
    } catch {
      /* not remembered; fine */
    }
  });
  // "All" as one list, or one column per group side by side; remembered in this browser.
  type Layout = 'list' | 'board';
  const LAYOUT_KEY = 'nora.todos.layout';
  function readLayout(): Layout {
    try {
      return localStorage.getItem(LAYOUT_KEY) === 'board' ? 'board' : 'list';
    } catch {
      return 'list';
    }
  }
  let layout = $state<Layout>(readLayout());
  $effect(() => {
    const value = layout;
    try {
      localStorage.setItem(LAYOUT_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });
  const board = $derived(layout === 'board' && filter === 'all');

  let todos = $state<Todo[]>([]);
  let groups = $state<TodoGroup[]>([]);
  let newTitle = $state('');
  let newDue = $state<string | null>(null);
  let newTime = $state<string | null>(null);
  let editing = $state<Todo | null>(null);
  let editingGroup = $state<TodoGroup | null>(null);

  $effect(() => {
    data.version;
    load(api.todos(), []).then((t) => (todos = t));
    load(api.todoGroups(), []).then((g) => {
      groups = g;
      if (typeof filter === 'number' && !g.some((x) => x.id === filter)) filter = 'all';
    });
  });

  const todayStr = today();
  const groupMap = $derived(new Map(groups.map((g) => [g.id, g])));
  const children = $derived.by(() => {
    const map = new Map<number, Todo[]>();
    for (const t of todos) if (t.parent_id !== null) map.set(t.parent_id, [...(map.get(t.parent_id) ?? []), t]);
    return map;
  });
  const matches = (t: Todo, f: Filter) =>
    f === 'all' || f === 'routines' ? true : f === 'today' ? !!t.due && t.due <= todayStr && !t.done : f === 'ungrouped' ? t.group_id === null : t.group_id === f;
  const roots = $derived(todos.filter((t) => (filter === 'today' ? true : t.parent_id === null) && matches(t, filter)));
  const visible = $derived(roots.filter((t) => showDone || !t.done));
  const doneCount = $derived(roots.filter((t) => t.done).length);
  const openCount = (f: Filter) => todos.filter((t) => (f === 'today' || t.parent_id === null) && matches(t, f) && !t.done).length;

  const heading = $derived(
    filter === 'all'
      ? t('todo.allTodos')
      : filter === 'today'
        ? t('todo.today')
        : filter === 'ungrouped'
          ? t('todo.ungrouped')
          : filter === 'routines'
            ? t('routine.title')
            : (groupMap.get(filter)?.name ?? ''),
  );

  /** Group that new and pasted todos go into for the current view. */
  const targetGroup = $derived(typeof filter === 'number' ? filter : null);

  // ⌘V anywhere on this page pastes into the current list.
  $effect(() => {
    const group = targetGroup;
    pageTarget.paste = () => pasteAsTodo(group);
    return () => (pageTarget.paste = null);
  });

  async function add() {
    const title = newTitle.trim();
    if (!title) return;
    const groupId = typeof filter === 'number' ? filter : null;
    // Todos added from the "Today" list are due today unless another deadline was picked.
    const due = newDue ?? (filter === 'today' ? todayStr : null);
    const time = newDue ? newTime : null;
    newTitle = '';
    newDue = null;
    newTime = null;
    await mutate(api.addTodo(groupId, null, title, due, time));
  }

  async function saveTodo() {
    if (!editing || !editing.title.trim()) return;
    await mutate(api.updateTodo({ ...editing, title: editing.title.trim(), due: editing.due || null, due_time: editing.due ? editing.due_time : null }), t('todo.saved'));
    editing = null;
  }

  const newGroup = () => (editingGroup = { id: 0, name: '', color: PALETTE[groups.length % PALETTE.length].value });

  async function saveGroup() {
    if (!editingGroup || !editingGroup.name.trim()) return;
    const g = { ...editingGroup, name: editingGroup.name.trim() };
    if (g.id === 0) {
      const id = await mutate(api.addTodoGroup(g.name, g.color), t('todo.groupCreated'));
      // On the board the new group just shows up as a new column.
      if (id !== undefined && !board) filter = id;
    } else {
      await mutate(api.updateTodoGroup(g), t('todo.groupSaved'));
    }
    editingGroup = null;
  }

  async function deleteGroup() {
    if (!editingGroup) return;
    await mutate(api.deleteTodoGroup(editingGroup.id), t('todo.groupDeleted'));
    editingGroup = null;
  }
</script>

<div class="todos">
  <aside class="groups">
    <h1>{t('nav.todos')}</h1>
    <div class="list">
      <button class="group" class:active={filter === 'all'} onclick={() => (filter = 'all')}>
        <Layers size={17} /> <span>{t('todo.all')}</span> <span class="count">{openCount('all')}</span>
      </button>
      <button class="group" class:active={filter === 'today'} onclick={() => (filter = 'today')}>
        <Sun size={17} /> <span>{t('todo.today')}</span> <span class="count">{openCount('today')}</span>
      </button>
      <button class="group" class:active={filter === 'ungrouped'} onclick={() => (filter = 'ungrouped')}>
        <Inbox size={17} /> <span>{t('todo.ungrouped')}</span> <span class="count">{openCount('ungrouped')}</span>
      </button>
      <button class="group" class:active={filter === 'routines'} onclick={() => (filter = 'routines')}>
        <Repeat size={17} /> <span>{t('routine.title')}</span>
      </button>
    </div>

    <div class="label groups-label">{t('todo.groups')}</div>
    <div class="list">
      {#each groups as g (g.id)}
        <div class="group-row">
          <button class="group" class:active={filter === g.id} onclick={() => (filter = g.id)}>
            <span class="dot" style:background={hex(g.color)}></span>
            <span class="truncate">{g.name}</span>
            <span class="count">{openCount(g.id)}</span>
          </button>
          <button class="icon-btn edit-group" onclick={() => (editingGroup = { ...g })} title={t('todo.editGroup')}><Pencil size={14} /></button>
        </div>
      {/each}
      <button class="group add" onclick={newGroup}>
        <FolderPlus size={17} /> <span>{t('todo.newGroup')}</span>
      </button>
    </div>
  </aside>

  <section class="main">
    {#if filter === 'routines'}
      <RoutineManager {groups} />
    {:else}
    <div class="head">
      <div>
        <h2 class="heading">{heading}</h2>
        <p class="muted small">{roots.length ? t('todo.doneOf', { done: doneCount, total: roots.length }) : t('todo.nothingYet')}</p>
      </div>
      <span class="spacer"></span>
      <label class="row show-done">
        <input type="checkbox" class="switch" bind:checked={showDone} /> {t('todo.showCompleted')}
      </label>
      {#if filter === 'all'}
        <div class="segmented layout-pick" role="group" aria-label={t('todo.layout')}>
          <button class:active={layout === 'list'} onclick={() => (layout = 'list')} title={t('todo.layoutList')}><List size={15} /> {t('todo.layoutList')}</button>
          <button class:active={layout === 'board'} onclick={() => (layout = 'board')} title={t('todo.layoutBoard')}><Columns3 size={15} /> {t('todo.layoutBoard')}</button>
        </div>
      {/if}
    </div>
    {#if board}
      <TodoBoard {todos} {groups} {children} {showDone} onedit={(x) => (editing = { ...x })} oneditgroup={(g) => (editingGroup = { ...g })} onnewgroup={newGroup} />
      <p class="faint small hint">{t('todo.boardTip')}</p>
    {:else}
    {#if roots.length}
      <div class="progress"><div style:width="{(doneCount / roots.length) * 100}%"></div></div>
    {/if}

    <div class="quick-add">
      <span class="plus"><Plus size={18} /></span>
      <input
        class="input"
        placeholder={filter === 'today' ? t('todo.addTodayPlaceholder') : t('todo.addPlaceholder')}
        bind:value={newTitle}
        onkeydown={(e) => e.key === 'Enter' && add()}
      />
      <div class="quick-deadline"><DeadlinePicker bind:due={newDue} bind:time={newTime} compact /></div>
    </div>

    <div
      class="items card"
      role="list"
      {...pasteOnHover(() => pasteAsTodo(targetGroup))}
      oncontextmenu={(e) =>
        openMenu(e, [
          { label: t('menu.paste'), icon: ClipboardPaste, shortcut: shortcut('V'), disabled: !clipboard.item, action: () => pasteAsTodo(targetGroup) },
        ])}
    >
      {#each visible as t (t.id)}
        <TodoItem todo={t} {children} groups={groupMap} {showDone} showGroup={typeof filter !== 'number'} onedit={(x) => (editing = { ...x })} />
      {:else}
        <div class="empty">
          <span class="empty-icon">{#if roots.length}<PartyPopper size={30} />{:else}<Sparkles size={30} />{/if}</span>
          <h2>{roots.length ? t('todo.allDoneTitle') : t('todo.freshTitle')}</h2>
          <p>{roots.length ? t('todo.allDoneBody') : t('todo.freshBody')}</p>
        </div>
      {/each}
    </div>
    <p class="faint small hint">{t('todo.tip')}</p>
    {/if}
    {/if}
  </section>
</div>

{#if editing}
  <Modal title={t('todo.editTitle')} onclose={() => (editing = null)} width={480}>
    <!-- svelte-ignore a11y_autofocus -->
    <input class="input title" bind:value={editing.title} autofocus onkeydown={(e) => e.key === 'Enter' && saveTodo()} />
    <div class="field">
      <span class="label">{t('dl.deadline')}</span>
      <DeadlinePicker bind:due={editing.due} bind:time={editing.due_time} />
    </div>
    {#if editing.parent_id === null}
      <div class="field">
        <label for="todo-group">{t('todo.group')}</label>
        <Select
          id="todo-group"
          bind:value={editing.group_id}
          options={[{ value: null, label: t('todo.ungrouped') }, ...groups.map((g) => ({ value: g.id as number | null, label: g.name, color: hex(g.color) }))]}
        />
      </div>
    {/if}
    <div class="field">
      <label for="todo-notes">{t('todo.notes')}</label>
      <textarea id="todo-notes" class="textarea" rows="4" placeholder={t('todo.notesPlaceholder')} bind:value={editing.notes}></textarea>
    </div>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editing = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveTodo}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

{#if editingGroup}
  <Modal title={editingGroup.id ? t('todo.editGroup') : t('todo.newGroup')} onclose={() => (editingGroup = null)} width={420}>
    <div class="field">
      <label for="group-name">{t('common.name')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="group-name" class="input" placeholder={t('todo.groupNamePlaceholder')} bind:value={editingGroup.name} autofocus
        onkeydown={(e) => e.key === 'Enter' && saveGroup()} />
    </div>
    <div class="field">
      <span class="label">{t('common.color')}</span>
      <ColorPicker bind:value={editingGroup.color} />
    </div>
    {#snippet footer()}
      {#if editingGroup?.id}<ConfirmButton onconfirm={deleteGroup} question={t('todo.deleteGroupQ')} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editingGroup = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveGroup}>{editingGroup?.id ? t('common.save') : t('todo.createGroup')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .todos {
    display: grid;
    grid-template-columns: 250px 1fr;
    height: 100%;
  }
  .groups {
    padding: 28px 16px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }
  .groups h1 {
    padding: 0 10px 18px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .groups-label {
    padding: 18px 12px 8px;
  }
  .group-row {
    position: relative;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 40px;
    padding: 0 12px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: var(--muted);
    font-weight: 600;
    cursor: pointer;
    text-align: left;
  }
  .group:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .group.active {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .group .dot {
    width: 10px;
    height: 10px;
  }
  .count {
    margin-left: auto;
    font-size: 12px;
    font-weight: 700;
    color: var(--faint);
  }
  .edit-group {
    position: absolute;
    right: 34px;
    top: 4px;
    opacity: 0;
  }
  .group-row:hover .edit-group {
    opacity: 1;
  }
  .group.add {
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
    gap: 12px;
    margin-bottom: 12px;
  }
  .heading {
    font-size: 22px;
  }
  .layout-pick button {
    gap: 5px;
  }
  .show-done {
    color: var(--muted);
    font-weight: 550;
    cursor: pointer;
  }
  .progress {
    height: 8px;
    border-radius: 8px;
    background: var(--surface-3);
    overflow: hidden;
    margin-bottom: 18px;
  }
  .progress div {
    height: 100%;
    border-radius: 8px;
    background: linear-gradient(90deg, var(--primary), var(--accent-2));
    transition: width 0.4s ease;
  }
  .quick-add {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: 14px;
    color: var(--primary);
  }
  .quick-add .plus {
    display: grid;
    position: absolute;
    left: 16px;
  }
  .quick-deadline {
    position: absolute;
    right: 8px;
  }
  .quick-add .input {
    height: 50px;
    padding-left: 44px;
    padding-right: 170px;
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    font-size: 15px;
  }
  .items {
    padding: 8px;
  }
  .hint {
    text-align: center;
    margin-top: 10px;
  }
  @container main (max-width: 760px) {
    .todos {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto minmax(0, 1fr);
    }
    .groups {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 26px 14px 10px;
      border-right: none;
      border-bottom: 1px solid var(--border);
      overflow-x: auto;
      overflow-y: hidden;
    }
    .groups h1,
    .groups-label,
    .edit-group {
      display: none;
    }
    .list {
      flex-direction: row;
      gap: 4px;
    }
    .group {
      width: auto;
      white-space: nowrap;
      height: 36px;
    }
    .main {
      padding: 18px 14px;
    }
  }
</style>
