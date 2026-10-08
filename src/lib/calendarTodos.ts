// Todos with a due date, as shown on the calendar (in their group's color).
import { api, type Todo } from './api';
import { DEFAULT_COLOR } from './colors';
import { t } from './i18n.svelte';
import type { CalTodo } from './lanes';
import { mutate } from './state.svelte';

export async function loadCalendarTodos(): Promise<CalTodo[]> {
  const [todos, groups] = await Promise.all([api.todos().catch(() => []), api.todoGroups().catch(() => [])]);
  const color = new Map(groups.map((g) => [g.id, g.color]));
  return todos
    .filter((todo) => todo.due)
    .map((todo) => ({ todo, color: (todo.group_id !== null && color.get(todo.group_id)) || DEFAULT_COLOR }));
}

export function setCalendarTodoDone(todo: Todo, done: boolean) {
  return mutate(api.setTodoDone(todo.id, done), done ? t('cal.todoDone', { name: todo.title }) : undefined);
}
