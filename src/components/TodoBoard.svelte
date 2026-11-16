<script lang="ts">
  // Todos side by side, one column per group (a project, say), plus one for todos without a
  // group. Drag a todo to another column to move it there; its sub-todos come along.
  import { FolderPlus, Inbox, Pencil, Plus } from '@lucide/svelte';
  import TodoItem from '../pages/TodoItem.svelte';
  import { api, type Todo, type TodoGroup } from '../lib/api';
  import { hex } from '../lib/colors';
  import { pasteAsTodo, pasteOnHover } from '../lib/clipboard.svelte';
  import { t } from '../lib/i18n.svelte';
  import { mutate } from '../lib/state.svelte';

  let {
    todos,
    groups,
    children,
    showDone,
    onedit,
    oneditgroup,
    onnewgroup,
  }: {
    todos: Todo[];
    groups: TodoGroup[];
    children: Map<number, Todo[]>;
    showDone: boolean;
    onedit: (t: Todo) => void;
    oneditgroup: (g: TodoGroup) => void;
    onnewgroup: () => void;
  } = $props();

  const groupMap = $derived(new Map(groups.map((g) => [g.id, g])));
  type Column = { id: number | null; name: string; color: string | null; group?: TodoGroup };
  const columns = $derived<Column[]>([
    ...groups.map((g) => ({ id: g.id, name: g.name, color: hex(g.color), group: g })),
    { id: null, name: t('todo.ungrouped'), color: null },
  ]);
  const rootsOf = (id: number | null) => todos.filter((x) => x.parent_id === null && x.group_id === id);

  // ---- quick add, one field per column
  let drafts = $state<Record<string, string>>({});
  const keyOf = (id: number | null) => String(id ?? 'none');
  async function add(id: number | null) {
    const title = (drafts[keyOf(id)] ?? '').trim();
    if (!title) return;
    drafts[keyOf(id)] = '';
    await mutate(api.addTodo(id, null, title));
  }

  // ---- drag a todo to another column
  let dragging = $state<Todo | null>(null);
  let over = $state<string | null>(null);
  /** Where the pointer went down, so dragging never starts from a text field. */
  let pressed: EventTarget | null = null;

  function dragStart(e: DragEvent, todo: Todo) {
    if (pressed instanceof HTMLElement && pressed.closest('input, textarea')) {
      e.preventDefault();
      return;
    }
    e.dataTransfer?.setData('application/x-nora-todo', String(todo.id));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    dragging = todo;
  }
  function dragOver(e: DragEvent, id: number | null) {
    if (!dragging) return;
    e.preventDefault();
    over = keyOf(id);
  }
  async function drop(e: DragEvent, id: number | null) {
    e.preventDefault();
    const todo = dragging;
    dragging = null;
    over = null;
    if (!todo || todo.group_id === id) return;
    const name = id === null ? t('todo.ungrouped') : (groupMap.get(id)?.name ?? '');
    await mutate(api.updateTodo({ ...todo, group_id: id }), t('todo.movedTo', { name }));
  }
  function dragEnd() {
    dragging = null;
    over = null;
  }
</script>

<div class="board" onpointerdowncapture={(e) => (pressed = e.target)}>
  {#each columns as col (keyOf(col.id))}
    {@const roots = rootsOf(col.id)}
    {@const shown = roots.filter((x) => showDone || !x.done)}
    {@const done = roots.filter((x) => x.done).length}
    <section
      class="column card"
      class:over={over === keyOf(col.id) && dragging?.group_id !== col.id}
      style:--c={col.color ?? 'var(--faint)'}
      ondragover={(e) => dragOver(e, col.id)}
      ondragleave={(e) => {
        if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) over = null;
      }}
      ondrop={(e) => drop(e, col.id)}
      aria-label={col.name}
    >
      <header class="col-head">
        {#if col.color}<span class="dot"></span>{:else}<Inbox size={15} />{/if}
        <h3 class="truncate">{col.name}</h3>
        <span class="count tabular">{roots.length - done}</span>
        <span class="spacer"></span>
        {#if col.group}
          {@const g = col.group}
          <button class="icon-btn edit" onclick={() => oneditgroup(g)} title={t('todo.editGroup')}><Pencil size={14} /></button>
        {/if}
      </header>
      {#if roots.length}
        <div class="bar"><span style:width="{(done / roots.length) * 100}%"></span></div>
      {/if}
      <div class="add">
        <Plus size={15} />
        <input
          class="input"
          placeholder={t('todo.addPlaceholder')}
          value={drafts[keyOf(col.id)] ?? ''}
          oninput={(e) => (drafts[keyOf(col.id)] = e.currentTarget.value)}
          onkeydown={(e) => e.key === 'Enter' && !e.isComposing && add(col.id)}
        />
      </div>
      <div class="items" role="list" {...pasteOnHover(() => pasteAsTodo(col.id))}>
        {#each shown as todo (todo.id)}
          <div
            class="drag"
            class:dragging={dragging?.id === todo.id}
            role="presentation"
            draggable="true"
            ondragstart={(e) => dragStart(e, todo)}
            ondragend={dragEnd}
          >
            <TodoItem {todo} {children} groups={groupMap} {showDone} showGroup={false} compact {onedit} />
          </div>
        {:else}
          <p class="faint small none">{roots.length ? t('todo.boardAllDone') : t('todo.boardEmpty')}</p>
        {/each}
      </div>
    </section>
  {/each}
  <button class="column new" onclick={onnewgroup}><FolderPlus size={18} /> {t('todo.newGroup')}</button>
</div>

<style>
  .board {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    overflow-x: auto;
    padding-bottom: 12px;
  }
  .column {
    flex: 0 0 290px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    padding: 12px 8px 8px;
    border-top: 3px solid var(--c);
    transition:
      box-shadow 0.12s,
      background 0.12s;
  }
  .column.over {
    background: color-mix(in srgb, var(--c) 8%, var(--surface));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--c) 55%, transparent);
  }
  .col-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px;
    color: var(--muted);
  }
  .col-head h3 {
    margin: 0;
    font-size: 15px;
    color: var(--text);
  }
  .dot {
    width: 10px;
    height: 10px;
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }
  .count {
    font-size: 12px;
    font-weight: 700;
    color: var(--faint);
  }
  .edit {
    opacity: 0;
  }
  .column:hover .edit {
    opacity: 1;
  }
  .bar {
    height: 5px;
    margin: 0 6px;
    border-radius: 5px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 5px;
    background: var(--c);
    transition: width 0.3s ease;
  }
  .add {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--primary);
  }
  .add :global(svg) {
    position: absolute;
    left: 11px;
  }
  .add .input {
    height: 36px;
    padding-left: 32px;
    font-size: 13.5px;
  }
  .items {
    display: flex;
    flex-direction: column;
    min-height: 40px;
  }
  .drag {
    border-radius: 14px;
    cursor: grab;
  }
  .drag.dragging {
    opacity: 0.45;
  }
  .none {
    margin: 0;
    padding: 12px 6px;
    text-align: center;
  }
  .column.new {
    flex: 0 0 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 64px;
    border: 1.5px dashed var(--border-strong, var(--border));
    border-radius: var(--radius);
    background: none;
    color: var(--faint);
    font-weight: 650;
    cursor: pointer;
  }
  .column.new:hover {
    color: var(--primary);
    border-color: var(--primary);
  }
</style>
