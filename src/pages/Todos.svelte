<script lang="ts">
  import { FolderPlus, Inbox, Layers, PartyPopper, Pencil, Plus, Sparkles, Sun } from '@lucide/svelte';
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

  type Filter = 'all' | 'today' | 'ungrouped' | number;
  let filter = $state<Filter>('all');
  let showDone = $state(true);
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
    f === 'all' ? true : f === 'today' ? !!t.due && t.due <= todayStr && !t.done : f === 'ungrouped' ? t.group_id === null : t.group_id === f;
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

  async function saveGroup() {
    if (!editingGroup || !editingGroup.name.trim()) return;
    const g = { ...editingGroup, name: editingGroup.name.trim() };
    if (g.id === 0) {
      const id = await mutate(api.addTodoGroup(g.name, g.color), t('todo.groupCreated'));
      if (id !== undefined) filter = id;
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
      <button class="group add" onclick={() => (editingGroup = { id: 0, name: '', color: PALETTE[groups.length % PALETTE.length].value })}>
        <FolderPlus size={17} /> <span>{t('todo.newGroup')}</span>
      </button>
    </div>
  </aside>

  <section class="main">
    <div class="head">
      <div>
        <h2 class="heading">{heading}</h2>
        <p class="muted small">{roots.length ? t('todo.doneOf', { done: doneCount, total: roots.length }) : t('todo.nothingYet')}</p>
      </div>
      <span class="spacer"></span>
      <label class="row show-done">
        <input type="checkbox" class="switch" bind:checked={showDone} /> {t('todo.showCompleted')}
      </label>
    </div>
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
        <select id="todo-group" class="select" bind:value={editing.group_id}>
          <option value={null}>{t('todo.ungrouped')}</option>
          {#each groups as g (g.id)}<option value={g.id}>{g.name}</option>{/each}
        </select>
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
