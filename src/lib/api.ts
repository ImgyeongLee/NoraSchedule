// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
import { invoke } from '@tauri-apps/api/core';

/** Local date-time as `YYYY-MM-DDTHH:MM:SS` (no timezone). */
export type DateTime = string;
/** Local date as `YYYY-MM-DD`. */
export type DateStr = string;

export type Freq = 'daily' | 'weekly' | 'monthly' | 'yearly';

export interface Repeat {
  freq: Freq;
  interval: number;
  /** Weekly only: 0 = Sunday … 6 = Saturday. Empty = the first occurrence's weekday. */
  weekdays: number[];
  /** Last date an occurrence may start on. */
  until: DateStr | null;
  /** Total number of occurrences. */
  count: number | null;
}

/** For repeating events: change only this occurrence, or the whole series. */
/** What an uploaded image is for (see `Purpose` in src-tauri/src/images.rs). */
export type ImagePurpose = 'cover' | 'card' | 'header' | 'sticker';

export type Scope = 'one' | 'all';

export interface CalEvent {
  id: number;
  title: string;
  start: DateTime;
  /** Exclusive end for timed events; the last day (at 00:00) for all-day events. */
  end: DateTime;
  all_day: boolean;
  color: number;
  location: string;
  links: string[];
  memo: string;
  repeat: Repeat | null;
  /** Skipped occurrence dates of a series. */
  exdates: DateStr[];
  /** For an occurrence of a repeating event: the date it falls on. */
  occurrence: DateStr | null;
  /** Called off but kept on the calendar (shown struck through). */
  cancelled: boolean;
  /** Ids of the attached tags. */
  tags: number[];
  /** Remind this many minutes before the start; null for no reminder. */
  reminder: number | null;
}

/** A user-defined label for events, optionally grouped under a category. */
export interface Tag {
  id: number;
  name: string;
  color: number;
  /** Free-text group, e.g. "Work"; empty for none. */
  category: string;
}

/** Sent by the backend when an event's reminder time arrives. */
export interface Reminder {
  event: CalEvent;
  /** When the event starts (09:00 for all-day events). */
  starts_at: DateTime;
}

export interface DDay {
  id: number;
  title: string;
  date: DateStr;
  color: number;
  /** Cover image file name (see lib/images.ts), or null. */
  image: string | null;
  /** Counts toward the same month and day every year (birthdays, anniversaries). */
  yearly: boolean;
  /** For past dates, count the date itself as day 1 (D+1) instead of day 0. */
  count_from_one: boolean;
  /** Card shape on the D-Day page: landscape (`wide`) or portrait (`tall`). */
  shape: DDayShape;
}

export type DDayShape = 'normal' | 'wide' | 'tall';

export interface TodoGroup {
  id: number;
  name: string;
  color: number;
}

export interface Todo {
  id: number;
  group_id: number | null;
  parent_id: number | null;
  title: string;
  notes: string;
  done: boolean;
  due: DateStr | null;
  /** Optional deadline time `HH:MM`, only used together with `due`. */
  due_time: string | null;
  created_at: number;
  completed_at: number | null;
}

export interface BookmarkFolder {
  id: number;
  name: string;
  parent_id: number | null;
  color: number;
}

export interface Bookmark {
  id: number;
  folder_id: number | null;
  title: string;
  url: string;
  /** Preset icon id, see lib/bookmarks.ts. */
  kind: string;
  note: string;
  created_at: number;
}

export interface Expense {
  id: number;
  amount: number;
  /** Preset category id, see lib/expenses.svelte.ts. */
  category: string;
  date: DateStr;
  note: string;
}

/** Which TRPG list an entry belongs to. */
export type TrpgKind = 'rulebook' | 'scenario_book' | 'played' | 'wishlist';

export interface TrpgEntry {
  id: number;
  kind: TrpgKind;
  title: string;
  /** Scenario writer, or the book's author / publisher. */
  writer: string;
  /** Rule system, e.g. "CoC 7th". */
  system: string;
  links: string[];
  /** Cover image file name (books), see lib/images.ts. */
  image: string | null;
  /** When it was played (`played` entries). */
  date: DateStr | null;
  /** Roles played, comma-separated: 'gm', 'pl', 'HO1', 'PC2'… (see lib/trpgRoles.ts); '' when not recorded. */
  role: string;
  /** Kept from an earlier version (who played together); no longer shown. */
  pair: string;
  /** User-named group the entry is filed under, e.g. "CoC 타이만"; '' for none. */
  folder: string;
  memo: string;
  created_at: number;
}

export type BookStatus = 'want' | 'reading' | 'read';

export interface Book {
  id: number;
  title: string;
  author: string;
  publisher: string;
  status: BookStatus;
  /** Cover image file name, see lib/images.ts. */
  image: string | null;
  /** 0 when unknown. */
  total_pages: number;
  /** The page read up to. */
  current_page: number;
  /** Half stars: 0 (not rated) to 10 (five stars). */
  rating: number;
  started: DateStr | null;
  finished: DateStr | null;
  /** Markdown. */
  review: string;
  created_at: number;
}

export interface BackupManifest {
  format: number;
  app_version: string;
  exported_at: number;
  counts: { events: number; todos: number; memos: number; ddays: number; bookmarks: number; expenses: number; trpg?: number; books?: number; images: number };
}

export interface Memo {
  id: number;
  title: string;
  body: string;
  created_at: number;
  updated_at: number;
  group_id: number | null;
}

export interface MemoGroup {
  id: number;
  name: string;
  color: number;
}

export interface PomodoroSession {
  started_at: number;
  ended_at: number;
  label: string;
}

export interface Activity {
  app: string;
  title: string;
  start: number;
  end: number;
}

export interface ActivitySummary {
  total: number;
  per_day: number[];
  apps: { app: string; secs: number }[];
  titles: { app: string; title: string; secs: number }[];
}

export type TrackState = 'starting' | 'tracking' | 'idle' | 'paused' | 'untracked' | 'unavailable';

export interface TrackerStatus {
  state: TrackState;
  app: string;
  title: string;
  segment_secs: number;
  /** Apps focused recently (most recent first), offered as suggestions for the tracked list. */
  recent_apps: string[];
}

export interface TrackerSettings {
  paused: boolean;
  idle_threshold_secs: number;
  /** Apps whose time is recorded; every other app is skipped. */
  tracked_apps: string[];
}

export const api = {
  /** Paints the native title bar (Windows 11) in the theme's colors, as 0xRRGGBB. */
  setTitlebarColors: (caption: number, text: number, dark: boolean) =>
    invoke<void>('set_titlebar_colors', { caption, text, dark }),
  getSetting: (key: string) => invoke<string | null>('get_setting', { key }),

  exportData: (path: string) => invoke<BackupManifest>('export_data', { path }),
  inspectBackup: (path: string) => invoke<BackupManifest>('inspect_backup', { path }),
  /** Replaces all data with the backup and restarts the app (does not return on success). */
  importData: (path: string) => invoke<void>('import_data', { path }),
  /** Deletes all data and restarts the app (does not return on success). */
  resetAllData: () => invoke<void>('reset_all_data'),
  setSetting: (key: string, value: string) => invoke<void>('set_setting', { key, value }),

  eventsBetween: (from: DateStr, to: DateStr) => invoke<CalEvent[]>('events_between', { from, to }),
  saveEvent: (event: CalEvent, scope: Scope = 'all') => invoke<number>('save_event', { event, scope }),
  /** Returns the stored event as it was before deleting, for undo. */
  deleteEvent: (id: number, occurrence: DateStr | null = null, scope: Scope = 'all') =>
    invoke<CalEvent | null>('delete_event', { id, occurrence, scope }),
  restoreOccurrence: (id: number, day: DateStr) => invoke<void>('restore_occurrence', { id, day }),

  tags: () => invoke<Tag[]>('tags'),
  /** Creates (id 0) or updates a tag; returns its id. */
  saveTag: (tag: Tag) => invoke<number>('save_tag', { tag }),
  /** Deletes a tag and removes it from every event. */
  deleteTag: (id: number) => invoke<void>('delete_tag', { id }),

  ddays: () => invoke<DDay[]>('ddays'),
  saveDday: (dday: DDay) => invoke<void>('save_dday', { dday }),
  deleteDday: (id: number) => invoke<void>('delete_dday', { id }),
  /** Sends raw image bytes; Rust optimizes and stores them and returns the file name. */
  /** `purpose` sets how large the image is kept; stickers keep transparency. */
  importImage: (bytes: Uint8Array, purpose: ImagePurpose = 'cover') =>
    invoke<string>('import_image', bytes, { headers: { purpose } }),
  removeUnusedImages: () => invoke<void>('remove_unused_images'),

  todoGroups: () => invoke<TodoGroup[]>('todo_groups'),
  addTodoGroup: (name: string, color: number) => invoke<number>('add_todo_group', { name, color }),
  updateTodoGroup: (group: TodoGroup) => invoke<void>('update_todo_group', { group }),
  deleteTodoGroup: (id: number) => invoke<void>('delete_todo_group', { id }),
  todos: () => invoke<Todo[]>('todos'),
  addTodo: (groupId: number | null, parentId: number | null, title: string, due: DateStr | null = null, dueTime: string | null = null) =>
    invoke<number>('add_todo', { groupId, parentId, title, due, dueTime }),
  updateTodo: (todo: Todo) => invoke<void>('update_todo', { todo }),
  setTodoDone: (id: number, done: boolean) => invoke<void>('set_todo_done', { id, done }),
  deleteTodo: (id: number) => invoke<void>('delete_todo', { id }),

  bookmarkFolders: () => invoke<BookmarkFolder[]>('bookmark_folders'),
  saveBookmarkFolder: (folder: BookmarkFolder) => invoke<number>('save_bookmark_folder', { folder }),
  deleteBookmarkFolder: (id: number) => invoke<void>('delete_bookmark_folder', { id }),
  bookmarks: () => invoke<Bookmark[]>('bookmarks'),
  saveBookmark: (bookmark: Bookmark) => invoke<number>('save_bookmark', { bookmark }),
  /** Saves the order bookmarks are shown in; `ids` lists them first to last. */
  reorderBookmarks: (ids: number[]) => invoke<void>('reorder_bookmarks', { ids }),
  deleteBookmark: (id: number) => invoke<Bookmark | null>('delete_bookmark', { id }),

  expensesBetween: (from: DateStr, to: DateStr) => invoke<Expense[]>('expenses_between', { from, to }),
  saveExpense: (expense: Expense) => invoke<number>('save_expense', { expense }),
  deleteExpense: (id: number) => invoke<Expense | null>('delete_expense', { id }),

  trpgEntries: () => invoke<TrpgEntry[]>('trpg_entries'),
  saveTrpgEntry: (entry: TrpgEntry) => invoke<number>('save_trpg_entry', { entry }),
  deleteTrpgEntry: (id: number) => invoke<TrpgEntry | null>('delete_trpg_entry', { id }),

  books: () => invoke<Book[]>('books'),
  saveBook: (book: Book) => invoke<number>('save_book', { book }),
  deleteBook: (id: number) => invoke<Book | null>('delete_book', { id }),

  memos: () => invoke<Memo[]>('memos'),
  createMemo: (title: string, groupId: number | null = null) => invoke<number>('create_memo', { title, groupId }),
  setMemoGroup: (id: number, groupId: number | null) => invoke<void>('set_memo_group', { id, groupId }),
  memoGroups: () => invoke<MemoGroup[]>('memo_groups'),
  saveMemoGroup: (group: MemoGroup) => invoke<number>('save_memo_group', { group }),
  deleteMemoGroup: (id: number) => invoke<void>('delete_memo_group', { id }),
  updateMemo: (id: number, title: string, body: string) => invoke<void>('update_memo', { id, title, body }),
  deleteMemo: (id: number) => invoke<void>('delete_memo', { id }),

  addPomodoroSession: (startedAt: number, endedAt: number, label: string) =>
    invoke<void>('add_pomodoro_session', { startedAt, endedAt, label }),
  pomodoroSessions: (fromTs: number, toTs: number) =>
    invoke<PomodoroSession[]>('pomodoro_sessions', { fromTs, toTs }),

  activitySummary: (days: DateStr[], titleLimit = 50) =>
    invoke<ActivitySummary>('activity_summary', { days, titleLimit }),
  activityTimeline: (day: DateStr) => invoke<Activity[]>('activity_timeline', { day }),
  trackerStatus: () => invoke<TrackerStatus>('tracker_status'),
  trackerSettings: () => invoke<TrackerSettings>('tracker_settings'),
  setTrackerSettings: (settings: TrackerSettings) => invoke<void>('set_tracker_settings', { settings }),
};
