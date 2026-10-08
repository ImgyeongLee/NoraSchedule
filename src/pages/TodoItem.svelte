<script lang="ts">
  import { slide } from 'svelte/transition';
  import {
    CalendarClock, ChevronRight, CircleCheck, Circle, ClipboardPaste, Copy, CopyPlus, NotebookPen, Pencil, Plus, Trash,
  } from '@lucide/svelte';
  import {
    clipboard, copyOnHover, copyTodo, deleteTodoWithUndo, duplicateTodo, pasteAsTodo, pasteOnHover, shortcut, todoTree,
  } from '../lib/clipboard.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import TodoItem from './TodoItem.svelte';
  import { api, type Todo, type TodoGroup } from '../lib/api';
  import { hex } from '../lib/colors';
  import { deadlineState, fmtDeadline } from '../lib/deadline';
  import { mutate } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    todo,
    children,
    groups,
    showDone,
    showGroup,
    depth = 0,
    onedit,
  }: {
    todo: Todo;
    children: Map<number, Todo[]>;
    groups: Map<number, TodoGroup>;
    showDone: boolean;
    showGroup: boolean;
    depth?: number;
    onedit: (t: Todo) => void;
  } = $props();

  let open = $state(true);
  let adding = $state(false);
  let childTitle = $state('');

  const kids = $derived(children.get(todo.id) ?? []);
  const visibleKids = $derived(kids.filter((k) => showDone || !k.done));
  const kidsDone = $derived(kids.filter((k) => k.done).length);
  const group = $derived(todo.group_id !== null ? groups.get(todo.group_id) : undefined);
  const dl = $derived(deadlineState(todo));

  const tree = () => todoTree(todo, children);

  function showMenu(e: MouseEvent) {
    openMenu(e, [
      { label: t('menu.edit'), icon: Pencil, action: () => onedit(todo) },
      { label: t('menu.addSub'), icon: Plus, action: () => (adding = true) },
      todo.done
        ? { label: t('menu.markUndone'), icon: Circle, action: () => mutate(api.setTodoDone(todo.id, false)) }
        : { label: t('menu.markDone'), icon: CircleCheck, action: () => mutate(api.setTodoDone(todo.id, true)) },
      'separator',
      { label: t('menu.copy'), icon: Copy, shortcut: shortcut('C'), action: () => copyTodo(tree()) },
      { label: t('menu.pasteAsSub'), icon: ClipboardPaste, disabled: !clipboard.item, action: () => pasteAsTodo(todo.group_id, todo.id) },
      { label: t('menu.duplicate'), icon: CopyPlus, action: () => duplicateTodo(tree()) },
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: () => deleteTodoWithUndo(tree()) },
    ]);
  }

  async function addChild() {
    const title = childTitle.trim();
    if (!title) return;
    childTitle = '';
    open = true;
    await mutate(api.addTodo(todo.group_id, todo.id, title));
  }
</script>

<div class="item" style:--depth={depth} transition:slide={{ duration: 160 }}>
  <div
    class="row-main"
    class:done={todo.done}
    role="listitem"
    {...copyOnHover(() => copyTodo(tree()))}
    {...pasteOnHover(() => pasteAsTodo(todo.group_id, todo.parent_id))}
    oncontextmenu={showMenu}
  >
    {#if kids.length}
      <button class="toggle" class:open onclick={() => (open = !open)} aria-label={open ? t('todo.collapse') : t('todo.expand')}>
        <ChevronRight size={15} />
      </button>
    {:else}
      <span class="toggle-space"></span>
    {/if}
    <input type="checkbox" class="check" checked={todo.done} onchange={(e) => mutate(api.setTodoDone(todo.id, e.currentTarget.checked))}
      aria-label={t('todo.markDone')} />
    <button class="title" ondblclick={() => onedit(todo)} onclick={() => onedit(todo)}>{todo.title}</button>
    <div class="meta">
      {#if todo.notes}<span class="chip" title={todo.notes}><NotebookPen size={12} /></span>{/if}
      {#if kids.length}<span class="chip">{kidsDone}/{kids.length}</span>{/if}
      {#if todo.due}
        <span class="chip deadline {todo.done ? '' : (dl ?? '')}" title={dl === 'overdue' ? t('dl.overdue') : t('dl.deadline')}>
          <CalendarClock size={12} />
          {fmtDeadline(todo.due, todo.due_time)}
        </span>
      {/if}
      {#if showGroup && group}
        <span class="chip" style:color={hex(group.color)} style:background="color-mix(in srgb, {hex(group.color)} 12%, transparent)">
          {group.name}
        </span>
      {/if}
    </div>
    <div class="actions">
      <button class="icon-btn" onclick={() => (adding = !adding)} title={t('todo.addSub')}><Plus size={16} /></button>
      <button class="icon-btn" onclick={() => onedit(todo)} title={t('common.edit')}><Pencil size={15} /></button>
      <button class="icon-btn danger" onclick={() => mutate(api.deleteTodo(todo.id), kids.length ? t('todo.deletedWithSubs') : t('todo.deleted'))} title={t('todo.deleteHint')}>
        <Trash size={15} />
      </button>
    </div>
  </div>

  {#if adding}
    <div class="add-child" transition:slide={{ duration: 140 }}>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="input"
        placeholder={t('todo.subPlaceholder')}
        bind:value={childTitle}
        autofocus
        onkeydown={(e) => {
          if (e.key === 'Enter') addChild();
          if (e.key === 'Escape') adding = false;
        }}
        onblur={() => !childTitle && (adding = false)}
      />
    </div>
  {/if}

  {#if open}
    {#each visibleKids as kid (kid.id)}
      <TodoItem todo={kid} {children} {groups} {showDone} showGroup={false} depth={depth + 1} {onedit} />
    {/each}
  {/if}
</div>

<style>
  .row-main {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 46px;
    padding: 6px 8px 6px calc(8px + var(--depth) * 30px);
    border-radius: 14px;
    transition: background 0.12s;
  }
  .row-main:hover {
    background: var(--surface-2);
  }
  .toggle,
  .toggle-space {
    width: 22px;
    height: 22px;
    flex: none;
  }
  .toggle {
    display: grid;
    place-items: center;
    border: none;
    background: none;
    color: var(--faint);
    border-radius: 6px;
    cursor: pointer;
    transition: transform 0.15s;
  }
  .toggle.open {
    transform: rotate(90deg);
  }
  .title {
    flex: 0 1 auto;
    border: none;
    background: none;
    padding: 0;
    font-size: 14.5px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .done .title {
    color: var(--faint);
    text-decoration: line-through;
  }
  .meta {
    display: flex;
    gap: 6px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
  .deadline.overdue {
    color: var(--danger);
    background: var(--danger-soft);
  }
  .deadline.today {
    color: var(--primary);
    background: var(--primary-soft);
  }
  .deadline.soon {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 14%, transparent);
  }
  .actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .row-main:hover .actions {
    opacity: 1;
  }
  .add-child {
    padding: 4px 8px 8px calc(70px + var(--depth) * 30px);
  }
</style>
