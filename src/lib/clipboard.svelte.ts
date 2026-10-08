// Copy & paste for events and todos (⌘C/⌘V or Ctrl+C/Ctrl+V, and the right-click menu).
//
// Items are copied to an in-app clipboard (and as plain text to the system clipboard, so
// they can be pasted into other apps). Whatever the mouse is over decides what is copied
// and where a paste lands; each page also registers a fallback paste target.
import { CircleCheck, ClipboardPaste, Copy, CopyPlus, Pencil, RotateCcw, Trash } from '@lucide/svelte';
import { api, type CalEvent, type Todo } from './api';
import { DEFAULT_COLOR } from './colors';
import { addDays, addMinutes, dateOf, diffDays, pad, timeOf, toDateTime } from './dates';
import { t } from './i18n.svelte';
import type { MenuItem } from './menu.svelte';
import { data, toast } from './state.svelte';
import { askScope, isRepeatingOccurrence } from './prompt.svelte';

export interface TodoTree {
  todo: Todo;
  children: TodoTree[];
}

export type Clip = { kind: 'event'; event: CalEvent } | { kind: 'todo'; tree: TodoTree };

export const clipboard = $state({ item: null as Clip | null });

/** What ⌘C / ⌘V act on right now; set while the mouse is over an item or a drop zone. */
export const pointer: { copy: (() => void) | null; paste: (() => void) | null } = { copy: null, paste: null };
/** Where ⌘V pastes when the mouse is not over a drop zone (e.g. the selected calendar day). */
export const pageTarget: { paste: (() => void) | null } = { paste: null };

/** Spread onto an element: ⌘C copies via `copy` while the mouse is over it. */
export function copyOnHover(copy: () => void) {
  return { onmouseenter: () => (pointer.copy = copy), onmouseleave: () => (pointer.copy = null) };
}

/** Spread onto an element: ⌘V pastes via `paste` while the mouse is over it. */
export function pasteOnHover(paste: () => void) {
  return { onmouseenter: () => (pointer.paste = paste), onmouseleave: () => (pointer.paste = null) };
}

export const isMac = /mac/i.test(navigator.platform);
export const shortcut = (key: string) => (isMac ? `⌘${key}` : `Ctrl+${key}`);

const fmtMinutes = (m: number) => `${pad(Math.floor(m / 60))}:${pad(m % 60)}`;
const toMinutes = (time: string) => Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5));

function writeSystemClipboard(text: string) {
  navigator.clipboard?.writeText(text).catch(() => {});
}

// ---- copying -----------------------------------------------------------------

export function copyEvent(e: CalEvent) {
  clipboard.item = { kind: 'event', event: JSON.parse(JSON.stringify(e)) };
  const when = e.all_day ? dateOf(e.start) : `${dateOf(e.start)} ${timeOf(e.start)}–${timeOf(e.end)}`;
  writeSystemClipboard([`${e.title} (${when})`, e.location, ...e.links, e.memo].filter(Boolean).join('\n'));
  toast(t('clip.copied', { name: e.title }));
}

export function todoTree(todo: Todo, children: Map<number, Todo[]>): TodoTree {
  return { todo: { ...todo }, children: (children.get(todo.id) ?? []).map((c) => todoTree(c, children)) };
}

export function todoChildrenMap(todos: Todo[]): Map<number, Todo[]> {
  const map = new Map<number, Todo[]>();
  for (const x of todos) if (x.parent_id !== null) map.set(x.parent_id, [...(map.get(x.parent_id) ?? []), x]);
  return map;
}

const countDescendants = (tree: TodoTree): number => tree.children.reduce((n, c) => n + 1 + countDescendants(c), 0);

function treeText(tree: TodoTree, depth = 0): string[] {
  const line = `${'  '.repeat(depth)}- [${tree.todo.done ? 'x' : ' '}] ${tree.todo.title}`;
  return [line, ...tree.children.flatMap((c) => treeText(c, depth + 1))];
}

export function copyTodo(tree: TodoTree) {
  clipboard.item = { kind: 'todo', tree: JSON.parse(JSON.stringify(tree)) };
  writeSystemClipboard(treeText(tree).join('\n'));
  const n = countDescendants(tree);
  toast(n ? t('clip.copiedWithSubs', { name: tree.todo.title, n }) : t('clip.copied', { name: tree.todo.title }));
}

// ---- pasting -----------------------------------------------------------------

async function run(work: () => Promise<unknown>, message: string) {
  try {
    await work();
    data.version++;
    toast(message, 'success');
  } catch (e) {
    toast(String(e), 'error');
  }
}

/** A one-off copy of `src` (repeat rules are not copied). */
const single = (src: CalEvent): CalEvent => ({ ...src, id: 0, repeat: null, exdates: [], occurrence: null, cancelled: false });

/** Copy of `src` moved to `day` (and to `minutes` past midnight, if given), keeping its length. */
function eventOn(src: CalEvent, day: string, minutes?: number): CalEvent {
  if (src.all_day) {
    const span = diffDays(dateOf(src.end), dateOf(src.start));
    return { ...single(src), start: toDateTime(day, '00:00'), end: toDateTime(addDays(day, span), '00:00') };
  }
  const length = Math.round((new Date(src.end).getTime() - new Date(src.start).getTime()) / 60_000);
  const start = toDateTime(day, minutes !== undefined ? fmtMinutes(minutes) : timeOf(src.start));
  return { ...single(src), start, end: addMinutes(start, length) };
}

/** Pastes the clipboard into the calendar. A copied todo becomes an event. */
export function pasteAsEvent(day: string, minutes?: number) {
  const item = clipboard.item;
  if (!item) return toast(t('clip.nothing'));
  let event: CalEvent;
  if (item.kind === 'event') {
    event = eventOn(item.event, day, minutes);
  } else {
    const todo = item.tree.todo;
    const m = minutes ?? (todo.due_time ? toMinutes(todo.due_time) : undefined);
    const start = toDateTime(day, m !== undefined ? fmtMinutes(m) : '00:00');
    event = {
      id: 0, title: todo.title, start, end: m !== undefined ? addMinutes(start, 60) : start, all_day: m === undefined,
      color: DEFAULT_COLOR, location: '', links: [], memo: todo.notes, repeat: null, exdates: [], occurrence: null, cancelled: false, tags: [], reminder: null,
    };
  }
  run(() => api.saveEvent(event), t('clip.pasted', { name: event.title }));
}

/** Re-creates a todo tree. `keepDone` restores completion (used by undo); copies start open. */
async function createTree(tree: TodoTree, groupId: number | null, parentId: number | null, keepDone: boolean): Promise<void> {
  const src = tree.todo;
  const id = await api.addTodo(groupId, parentId, src.title, src.due, src.due_time);
  if (src.notes) await api.updateTodo({ ...src, id, group_id: groupId, parent_id: parentId });
  // Mark done before children exist, so completing it does not cascade onto them.
  if (keepDone && src.done) await api.setTodoDone(id, true);
  for (const child of tree.children) await createTree(child, groupId, id, keepDone);
}

/** Pastes the clipboard into a todo list. A copied event becomes a todo due at its start. */
export function pasteAsTodo(groupId: number | null, parentId: number | null = null) {
  const item = clipboard.item;
  if (!item) return toast(t('clip.nothing'));
  if (item.kind === 'todo') {
    run(() => createTree(item.tree, groupId, parentId, false), t('clip.pasted', { name: item.tree.todo.title }));
  } else {
    const e = item.event;
    const tree: TodoTree = {
      todo: {
        id: 0, group_id: groupId, parent_id: parentId, title: e.title, notes: e.memo, done: false,
        due: dateOf(e.start), due_time: e.all_day ? null : timeOf(e.start), created_at: 0, completed_at: null,
      },
      children: [],
    };
    run(() => createTree(tree, groupId, parentId, false), t('clip.pasted', { name: e.title }));
  }
}

export function duplicateEvent(e: CalEvent) {
  run(() => api.saveEvent(single(e)), t('clip.duplicated', { name: e.title }));
}

export function duplicateTodo(tree: TodoTree) {
  run(() => createTree(tree, tree.todo.group_id, tree.todo.parent_id, false), t('clip.duplicated', { name: tree.todo.title }));
}

// ---- deleting with undo -----------------------------------------------------

/** Deletes an event; for a repeating event first asks "just this one / all". Offers undo. */
export async function deleteEventWithUndo(e: CalEvent) {
  const scope = isRepeatingOccurrence(e) ? await askScope('delete') : 'all';
  if (!scope) return;
  pointer.copy = null; // the hovered element is about to disappear
  try {
    const stored = await api.deleteEvent(e.id, e.occurrence, scope);
    data.version++;
    const day = e.occurrence;
    const undo =
      scope === 'one' && day
        ? () => run(() => api.restoreOccurrence(e.id, day), t('event.saved'))
        : () => stored && run(() => api.saveEvent({ ...stored, id: 0, occurrence: null }), t('event.saved'));
    toast(t('clip.deleted', { name: e.title }), 'info', { label: t('common.undo'), run: undo });
  } catch (err) {
    toast(String(err), 'error');
  }
}

/** Marks an event as cancelled (kept on the calendar, struck through) or restores it. Offers undo. */
export async function setEventCancelled(e: CalEvent, cancelled: boolean) {
  const scope = isRepeatingOccurrence(e) ? await askScope('save') : 'all';
  if (!scope) return;
  try {
    const id = await api.saveEvent({ ...e, cancelled }, scope);
    data.version++;
    const day = e.occurrence;
    // Changing one day of a series detaches it into its own event; undo removes that copy again.
    const undo =
      scope === 'one' && day
        ? () => run(async () => { await api.deleteEvent(id, null, 'all'); await api.restoreOccurrence(e.id, day); }, t('event.saved'))
        : () => run(() => api.saveEvent({ ...e, cancelled: !cancelled }, scope), t('event.saved'));
    toast(t(cancelled ? 'clip.cancelled' : 'clip.restored', { name: e.title }), 'info', { label: t('common.undo'), run: undo });
  } catch (err) {
    toast(String(err), 'error');
  }
}

export async function deleteTodoWithUndo(tree: TodoTree) {
  const snapshot: TodoTree = JSON.parse(JSON.stringify(tree));
  pointer.copy = null;
  try {
    await api.deleteTodo(tree.todo.id);
    data.version++;
    toast(t('clip.deleted', { name: tree.todo.title }), 'info', {
      label: t('common.undo'),
      run: () => run(() => createTree(snapshot, snapshot.todo.group_id, snapshot.todo.parent_id, true), t('todo.saved')),
    });
  } catch (err) {
    toast(String(err), 'error');
  }
}

// ---- menus ------------------------------------------------------------------

export function eventMenu(e: CalEvent, onEdit: () => void): MenuItem[] {
  return [
    { label: t('menu.edit'), icon: Pencil, action: onEdit },
    { label: t('menu.copy'), icon: Copy, shortcut: shortcut('C'), action: () => copyEvent(e) },
    { label: t('menu.duplicate'), icon: CopyPlus, action: () => duplicateEvent(e) },
    'separator',
    e.cancelled
      ? { label: t('menu.uncancelEvent'), icon: RotateCcw, action: () => setEventCancelled(e, false) }
      : { label: t('menu.cancelEvent'), icon: CircleCheck, action: () => setEventCancelled(e, true) },
    { label: t('menu.delete'), icon: Trash, danger: true, action: () => deleteEventWithUndo(e) },
  ];
}

/** Menu for an empty spot in the calendar: a day, or a time slot when `minutes` is given. */
export function slotMenu(day: string, minutes: number | undefined, onNew: () => void, extra: MenuItem[] = []): MenuItem[] {
  const time = minutes !== undefined ? fmtMinutes(minutes) : '';
  return [
    { label: minutes !== undefined ? t('menu.newEventAt', { time }) : t('menu.newEventHere'), icon: Pencil, action: onNew },
    {
      label: minutes !== undefined ? t('menu.pasteAt', { time }) : t('menu.pasteHere'),
      icon: ClipboardPaste,
      shortcut: shortcut('V'),
      disabled: !clipboard.item,
      action: () => pasteAsEvent(day, minutes),
    },
    ...extra,
  ];
}
