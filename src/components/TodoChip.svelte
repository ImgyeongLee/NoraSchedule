<script lang="ts">
  // A todo on the calendar: click to tick it off, double-click to open the Todos page.
  import { CircleCheck, Circle } from '@lucide/svelte';
  import type { CalTodo } from '../lib/lanes';
  import { hex } from '../lib/colors';
  import { setCalendarTodoDone } from '../lib/calendarTodos';
  import { t } from '../lib/i18n.svelte';
  import { ui } from '../lib/state.svelte';

  let { item, compact = false, style = '' }: { item: CalTodo; compact?: boolean; style?: string } = $props();
  const todo = $derived(item.todo);
</script>

<button
  class="todo-chip"
  class:done={todo.done}
  class:compact
  {style}
  style:--c={hex(item.color)}
  title={[todo.title, todo.notes, t(todo.done ? 'cal.todoUndoHint' : 'cal.todoDoneHint')].filter(Boolean).join('\n')}
  onclick={(e) => { e.stopPropagation(); setCalendarTodoDone(todo, !todo.done); }}
  ondblclick={(e) => { e.stopPropagation(); ui.page = 'todos'; }}
  onpointerdown={(e) => e.stopPropagation()}
>
  <span class="box">{#if todo.done}<CircleCheck size={13} />{:else}<Circle size={13} />{/if}</span>
  {#if todo.due_time}<span class="time">{todo.due_time}</span>{/if}
  <span class="truncate">{todo.title}</span>
</button>

<style>
  .todo-chip {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 21px;
    margin: 0 5px;
    padding: 0 7px;
    border: 1.5px solid color-mix(in srgb, var(--c) 55%, transparent);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    pointer-events: auto;
  }
  .todo-chip:hover {
    background: color-mix(in srgb, var(--c) 10%, var(--surface));
  }
  .box {
    display: grid;
    flex: none;
    color: var(--c);
  }
  .time {
    flex: none;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .done {
    opacity: 0.55;
  }
  .done .truncate {
    text-decoration: line-through;
  }
  .compact {
    margin: 0 3px;
  }
</style>
