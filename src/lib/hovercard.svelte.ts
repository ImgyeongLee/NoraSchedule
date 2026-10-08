// Hover card: a small popup with an event's details (time, place, tags, memo), or a
// whole day's events when hovering a day in the overview calendar. Rendered by
// EventHoverCard in App.
import type { CalEvent } from './api';
import { copyEvent, pointer } from './clipboard.svelte';

const DELAY_MS = 350;

export const hoverCard = $state({
  visible: false,
  events: [] as CalEvent[],
  /** Shown above the list, e.g. the day when previewing a date. */
  heading: '',
  anchor: { left: 0, top: 0, right: 0, bottom: 0 },
});

let timer: ReturnType<typeof setTimeout> | undefined;

export function showHoverCard(el: Element, events: CalEvent[], heading = '') {
  clearTimeout(timer);
  if (!events.length) return hideHoverCard();
  timer = setTimeout(() => {
    const r = el.getBoundingClientRect();
    hoverCard.anchor = { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
    hoverCard.events = events;
    hoverCard.heading = heading;
    hoverCard.visible = true;
  }, DELAY_MS);
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
