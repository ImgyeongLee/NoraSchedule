<script lang="ts">
  import { ChevronRight, CircleCheck, ClipboardPaste, Copy, ListTodo, Pencil, Trash } from '@lucide/svelte';
  import { SvelteSet } from 'svelte/reactivity';
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

  /** Open todos, the most urgent deadline first; undated todos last. */
  const openSorted = (list: Todo[]) =>
    list.filter((x) => !x.done).sort((a, b) => deadlineKey(a).localeCompare(deadlineKey(b)) || a.id - b.id);
  const open = $derived(openSorted(todos));

  const children = $derived(todoChildrenMap(todos));
  const byId = $derived(new Map(todos.map((x) => [x.id, x])));
  // Sub-todos sit under their parent, like on the to-do page. One whose parent is already done
  // (or gone) is shown on its own.
  const roots = $derived(
    open.filter((x) => x.parent_id === null || !byId.get(x.parent_id) || byId.get(x.parent_id)!.done),
  );
  const openKids = (todo: Todo) => openSorted(children.get(todo.id) ?? []);
  /** Whether any row has sub-todos, so rows leave room for the fold arrow. */
  const nested = $derived(roots.some((x) => openKids(x).length > 0));

  // Folded parents, remembered in this browser.
  const FOLD_KEY = 'nora.widget.todos.folded';
  function readFolded(): number[] {
    try {
      const list = JSON.parse(localStorage.getItem(FOLD_KEY) ?? '[]');
      return Array.isArray(list) ? list.filter((x) => typeof x === 'number') : [];
    } catch {
      return [];
    }
  }
  const folded = new SvelteSet<number>(readFolded());
  function toggleFold(id: number) {
    if (folded.has(id)) folded.delete(id);
    else folded.add(id);
    try {
      localStorage.setItem(FOLD_KEY, JSON.stringify([...folded].filter((x) => byId.has(x))));
    } catch {
      /* not remembered; fine */
    }
  }

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

{#snippet row(todo: Todo, depth: number)}
  {@const g = depth === 0 && todo.group_id !== null ? groups.get(todo.group_id) : undefined}
  {@const kids = openKids(todo)}
  {@const all = children.get(todo.id) ?? []}
  <div
    class="row-todo"
    class:sub={depth > 0}
    style:--depth={depth}
    role="listitem"
    {...copyOnHover(() => copyTodo(todoTree(todo, children)))}
    oncontextmenu={(e) => showMenu(e, todo)}
  >
    {#if kids.length}
      <button class="fold" class:open={!folded.has(todo.id)} onclick={() => toggleFold(todo.id)}
        aria-label={folded.has(todo.id) ? t('todo.expand') : t('todo.collapse')}><ChevronRight size={14} /></button>
    {:else if nested}
      <span class="fold-space"></span>
    {/if}
    <input type="checkbox" class="check" onchange={() => mutate(api.setTodoDone(todo.id, true))} aria-label={t('todo.markDone')} />
    <span class="title truncate">{todo.title}</span>
    {#if all.length}<span class="sub-count tabular">{all.length - kids.length}/{all.length}</span>{/if}
    {#if todo.due}
      <span class="due {deadlineState(todo) ?? ''}">{fmtDeadline(todo.due, todo.due_time)}</span>
    {/if}
    {#if g}<span class="dot" style:background={hex(g.color)} title={g.name}></span>{/if}
  </div>
  {#if kids.length && !folded.has(todo.id)}
    {#each kids as kid (kid.id)}
      {@render row(kid, depth + 1)}
    {/each}
  {/if}
{/snippet}

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'todos')}><span class="w-icon"><ListTodo size={15} /></span>{t('w.todos')}</button>
    <span class="chip">{open.length}</span>
  </div>
  <div class="w-body" role="list" {...pasteOnHover(() => pasteAsTodo(null))}>
    {#each roots as todo (todo.id)}
      {@render row(todo, 0)}
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
    padding: 7px 4px 7px calc(4px + var(--depth, 0) * 22px);
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
  .sub .title {
    font-weight: 500;
    font-size: 13px;
  }
  .sub .check {
    width: 16px;
    height: 16px;
  }
  .fold,
  .fold-space {
    width: 16px;
    height: 18px;
    flex: none;
    margin-right: -4px;
  }
  .fold {
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--faint);
    cursor: pointer;
    transition: transform 0.15s;
  }
  .fold:hover {
    color: var(--text);
    background: var(--surface-3);
  }
  .fold.open {
    transform: rotate(90deg);
  }
  .sub-count {
    font-size: 11px;
    font-weight: 650;
    color: var(--faint);
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
