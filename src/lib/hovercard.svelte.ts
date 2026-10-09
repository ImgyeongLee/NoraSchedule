// Hover card: a small popup with an event's details (time, place, tags, memo), a whole
// day's events when hovering a day in the overview calendar, or a plain note (title,
// one line of details and a memo) for other items such as TRPG scenarios. Rendered by
// EventHoverCard in App.
import type { CalEvent } from './api';
import { copyEvent, pointer } from './clipboard.svelte';

const DELAY_MS = 350;

export const hoverCard = $state({
  visible: false,
  events: [] as CalEvent[],
  /** A plain note instead of events. */
  note: null as HoverNote | null,
  /** Shown above the list, e.g. the day when previewing a date. */
  heading: '',
  anchor: { left: 0, top: 0, right: 0, bottom: 0 },
});

export interface HoverNote {
  title: string;
  /** One line of details under the title; may be empty. */
  meta: string;
  memo: string;
}

let timer: ReturnType<typeof setTimeout> | undefined;

function showAfterDelay(el: Element, fill: () => void) {
  clearTimeout(timer);
  timer = setTimeout(() => {
    const r = el.getBoundingClientRect();
    hoverCard.anchor = { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
    fill();
    hoverCard.visible = true;
  }, DELAY_MS);
}

export function showHoverCard(el: Element, events: CalEvent[], heading = '') {
  if (!events.length) return hideHoverCard();
  showAfterDelay(el, () => {
    hoverCard.events = events;
    hoverCard.note = null;
    hoverCard.heading = heading;
  });
}

/** Shows a plain note card (only when there is a memo to read). */
export function showNoteCard(el: Element, note: HoverNote) {
  if (!note.memo.trim()) return hideHoverCard();
  showAfterDelay(el, () => {
    hoverCard.events = [];
    hoverCard.note = note;
    hoverCard.heading = '';
  });
}

export function hideHoverCard() {
  clearTimeout(timer);
  hoverCard.visible = false;
}

/** Spread onto an event element: shows its hover card, and ⌘C copies it while hovered. */
export function eventHover(e: CalEvent) {
  return {
    onmouseenter: (ev: MouseEvent) => {
      pointer.copy = () => copyEvent(e);
      showHoverCard(ev.currentTarget as Element, [e]);
    },
    onmouseleave: () => {
      pointer.copy = null;
      hideHoverCard();
    },
    onpointerdown: hideHoverCard,
  };
}
