// Bookmark link types: preset icons and colors, detected from the URL.
import type { Component } from 'svelte';
import {
  CodeXml, FileSpreadsheet, FileText, Globe, HardDrive, Newspaper, NotebookText, PenLine, PenTool, Play, Presentation,
} from '@lucide/svelte';
import type { Key } from './i18n.svelte';

export type KindId =
  | 'gdoc' | 'gsheet' | 'gslides' | 'gdrive' | 'blog' | 'article' | 'video' | 'code' | 'notes' | 'design' | 'other';

export const KINDS: { id: KindId; label: Key; icon: Component<{ size?: number }>; color: string }[] = [
  { id: 'gdoc', label: 'bm.kind.gdoc', icon: FileText, color: '#4285f4' },
  { id: 'gsheet', label: 'bm.kind.gsheet', icon: FileSpreadsheet, color: '#0f9d58' },
  { id: 'gslides', label: 'bm.kind.gslides', icon: Presentation, color: '#f4b400' },
  { id: 'gdrive', label: 'bm.kind.gdrive', icon: HardDrive, color: '#1fa463' },
  { id: 'blog', label: 'bm.kind.blog', icon: PenLine, color: '#ff7a45' },
  { id: 'article', label: 'bm.kind.article', icon: Newspaper, color: '#6c7a89' },
  { id: 'video', label: 'bm.kind.video', icon: Play, color: '#ef4444' },
  { id: 'code', label: 'bm.kind.code', icon: CodeXml, color: '#24292f' },
  { id: 'notes', label: 'bm.kind.notes', icon: NotebookText, color: '#8b5cf6' },
  { id: 'design', label: 'bm.kind.design', icon: PenTool, color: '#ec4899' },
  { id: 'other', label: 'bm.kind.other', icon: Globe, color: '#0ea5e9' },
];

export const kindOf = (id: string) => KINDS.find((k) => k.id === id) ?? KINDS[KINDS.length - 1];

const RULES: [RegExp, KindId][] = [
  [/docs\.google\.com\/document/, 'gdoc'],
  [/docs\.google\.com\/spreadsheets/, 'gsheet'],
  [/docs\.google\.com\/presentation/, 'gslides'],
  [/drive\.google\.com|docs\.google\.com\/(drive|file)/, 'gdrive'],
  [/youtube\.com|youtu\.be|vimeo\.com|twitch\.tv|tv\.naver\.com/, 'video'],
  [/github\.com|gitlab\.com|bitbucket\.org|stackoverflow\.com/, 'code'],
  [/notion\.(so|site)|evernote\.com|obsidian\.md/, 'notes'],
  [/figma\.com|dribbble\.com|behance\.net|canva\.com/, 'design'],
  [/medium\.com|velog\.io|tistory\.com|brunch\.co\.kr|blog\.naver\.com|substack\.com|wordpress\.com|blogspot\.|\/blog\//, 'blog'],
  [/news|article|nytimes|bbc\.|cnn\.|reuters|bloomberg|theguardian|chosun|joongang|hani\.co|wikipedia\.org/, 'article'],
];

export function detectKind(url: string): KindId {
  const u = url.toLowerCase();
  return RULES.find(([re]) => re.test(u))?.[1] ?? 'other';
}

/** Adds https:// when the scheme is missing. */
export function normalizeUrl(raw: string): string {
  const url = raw.trim();
  return /^[a-z][a-z0-9+.-]*:/i.test(url) ? url : `https://${url}`;
}

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

/** A readable default title from a URL, e.g. "example.com/guides/setup". */
export function titleFromUrl(url: string): string {
  try {
    const u = new URL(url);
    const path = decodeURIComponent(u.pathname).replace(/\/$/, '');
    return (u.hostname.replace(/^www\./, '') + (path.length > 1 ? path : '')).slice(0, 80);
  } catch {
    return url.slice(0, 80);
  }
}
