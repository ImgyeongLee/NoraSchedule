// Event tags: the shared list, plus the calendar's "show only these tags" filter.
import { api, type CalEvent, type Tag } from './api';
import { load } from './state.svelte';

const FILTER_KEY = 'nora.tagFilter';

function savedFilter(): number[] {
  try {
    const ids = JSON.parse(localStorage.getItem(FILTER_KEY) ?? '[]');
    return Array.isArray(ids) ? ids.filter((n) => typeof n === 'number') : [];
  } catch {
    return [];
  }
}

export const tagStore = $state({
  list: [] as Tag[],
  /** Tag ids the calendar is filtered to; empty shows every event. */
  filter: savedFilter(),
});

export async function refreshTags() {
  tagStore.list = await load(api.tags(), []);
  // Forget filters for tags that no longer exist.
  setTagFilter(tagStore.filter.filter((id) => tagStore.list.some((t) => t.id === id)));
}

export function setTagFilter(ids: number[]) {
  tagStore.filter = ids;
  try {
    localStorage.setItem(FILTER_KEY, JSON.stringify(ids));
  } catch {
    /* not persisted; fine */
  }
}

export function toggleTagFilter(id: number) {
  setTagFilter(tagStore.filter.includes(id) ? tagStore.filter.filter((x) => x !== id) : [...tagStore.filter, id]);
}

/** The event's tags in the order they were picked, skipping ids that were deleted. */
export const tagsOf = (e: CalEvent): Tag[] =>
  e.tags.flatMap((id) => tagStore.list.find((t) => t.id === id) ?? []);

/** The color an event is drawn in: its first-picked tag's color, or its own when it has no tags. */
export const eventColor = (e: CalEvent): number => tagsOf(e)[0]?.color ?? e.color;

/** Whether the event passes the calendar's tag filter. */
export const matchesTagFilter = (e: CalEvent) => !tagStore.filter.length || e.tags.some((id) => tagStore.filter.includes(id));

/** Tags grouped by category (alphabetical, uncategorized last). */
export function groupedTags(list: Tag[] = tagStore.list): { category: string; tags: Tag[] }[] {
  const groups = new Map<string, Tag[]>();
  for (const t of list) groups.set(t.category, [...(groups.get(t.category) ?? []), t]);
  return [...groups.entries()]
    .sort(([a], [b]) => (a === '' ? 1 : b === '' ? -1 : a.localeCompare(b)))
    .map(([category, tags]) => ({ category, tags }));
}

export const categories = () => [...new Set(tagStore.list.map((t) => t.category).filter(Boolean))].sort();
