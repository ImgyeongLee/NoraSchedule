<script lang="ts">
  import { CircleCheck, ClipboardPaste, Copy, ListTodo, Pencil, Trash } from '@lucide/svelte';
  import {
    clipboard, copyOnHover, copyTodo, deleteTodoWithUndo, pasteAsTodo, pasteOnHover, shortcut, todoChildrenMap, todoTree,
  } from '../lib/clipboard.svelte';
  import { openMenu } from '../lib/menu.svelte';
  import { api, type Todo, type TodoGroup } from '../lib/api';
  import { hex } from '../lib/colors';
  import { deadlineKey, deadlineState, fmtDeadline } from '../lib/deadline';
  import { t } from '../lib/i18n.svelte';
  import { data, load, mutate, ui } from '../lib/state.svelte';

  let todos = $state<Todo[]>([]);
  let groups = $state<Map<number, TodoGroup>>(new Map());
  let newTitle = $state('');

  $effect(() => {
    data.version;
    load(api.todos(), []).then((list) => (todos = list));
    load(api.todoGroups(), []).then((g) => (groups = new Map(g.map((x) => [x.id, x]))));
  });

  // Open todos, the most urgent deadline first; undated todos last.
  const open = $derived(
    todos.filter((x) => !x.done).sort((a, b) => deadlineKey(a).localeCompare(deadlineKey(b)) || a.id - b.id),
  );

  const children = $derived(todoChildrenMap(todos));

  function showMenu(e: MouseEvent, todo: Todo) {
    const tree = todoTree(todo, children);
    openMenu(e, [
      { label: t('menu.edit'), icon: Pencil, action: () => (ui.page = 'todos') },
      { label: t('menu.markDone'), icon: CircleCheck, action: () => mutate(api.setTodoDone(todo.id, true)) },
      'separator',
      { label: t('menu.copy'), icon: Copy, shortcut: shortcut('C'), action: () => copyTodo(tree) },
      { label: t('menu.paste'), icon: ClipboardPaste, shortcut: shortcut('V'), disabled: !clipboard.item, action: () => pasteAsTodo(null) },
      'separator',
      { label: t('menu.delete'), icon: Trash, danger: true, action: () => deleteTodoWithUndo(tree) },
    ]);
  }

  async function add() {
    const title = newTitle.trim();
    if (!title) return;
    newTitle = '';
    await mutate(api.addTodo(null, null, title));
  }
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'todos')}><span class="w-icon"><ListTodo size={15} /></span>{t('w.todos')}</button>
    <span class="chip">{open.length}</span>
  </div>
  <div class="w-body" role="list" {...pasteOnHover(() => pasteAsTodo(null))}>
    {#each open as todo (todo.id)}
      {@const g = todo.group_id !== null ? groups.get(todo.group_id) : undefined}
      <div
        class="row-todo"
        role="listitem"
        {...copyOnHover(() => copyTodo(todoTree(todo, children)))}
        oncontextmenu={(e) => showMenu(e, todo)}
      >
        <input type="checkbox" class="check" onchange={() => mutate(api.setTodoDone(todo.id, true))} aria-label={t('todo.markDone')} />
        <span class="title truncate">{todo.title}</span>
        {#if todo.due}
          <span class="due {deadlineState(todo) ?? ''}">{fmtDeadline(todo.due, todo.due_time)}</span>
        {/if}
        {#if g}<span class="dot" style:background={hex(g.color)} title={g.name}></span>{/if}
      </div>
    {:else}
      <div class="w-empty">{t('w.todos.empty')}</div>
    {/each}
  </div>
  <input class="input quick" placeholder={t('w.todos.add')} bind:value={newTitle} onkeydown={(e) => e.key === 'Enter' && add()} />
</div>

<style>
  .row-todo {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 4px;
    border-radius: 10px;
    cursor: pointer;
  }
  .row-todo:hover {
    background: var(--surface-2);
  }
  .check {
    width: 19px;
    height: 19px;
  }
  .title {
    flex: 1;
    font-weight: 550;
  }
  .due {
    font-size: 11.5px;
    font-weight: 650;
    color: var(--faint);
    white-space: nowrap;
  }
  .due.today {
    color: var(--primary);
  }
  .due.soon {
    color: var(--warning);
  }
  .due.overdue {
    color: var(--danger);
  }
  .quick {
    flex: none;
    margin-top: 8px;
    height: 36px;
  }
</style>
