// Shelf or plain list for the book pages (TRPG, Reading); remembered per page in the settings.
import { api } from './api';

export type BookView = 'shelf' | 'list';
export type BookViewPage = 'trpg' | 'reading';

export const bookViews = $state<Record<BookViewPage, BookView>>({ trpg: 'shelf', reading: 'shelf' });
const loaded = new Set<BookViewPage>();

const key = (page: BookViewPage) => `ui.bookView.${page}`;

export async function loadBookView(page: BookViewPage) {
  if (loaded.has(page)) return;
  loaded.add(page);
  const saved = await api.getSetting(key(page)).catch(() => null);
  if (saved === 'shelf' || saved === 'list') bookViews[page] = saved;
}

export function setBookView(page: BookViewPage, view: BookView) {
  bookViews[page] = view;
  api.setSetting(key(page), view).catch(() => {});
}
