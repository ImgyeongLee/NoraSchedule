// Right-click menu state. `ContextMenu.svelte` (mounted once in App) renders it.
import type { Component } from 'svelte';

export type MenuItem =
  | {
      label: string;
      icon?: Component<{ size?: number }>;
      shortcut?: string;
      danger?: boolean;
      disabled?: boolean;
      action: () => void;
    }
  | 'separator';

export const menu = $state({ open: false, x: 0, y: 0, items: [] as MenuItem[] });

const isMac = /mac/i.test(navigator.platform);
let openedAt = 0;

/**
 * True for clicks that are really "right-clicks": Control-click on macOS (which WebKit
 * also reports as a left click), or a click arriving right after the menu opened
 * (the tail end of the same right-click) while the menu is still showing.
 * Click handlers must ignore these so a right-click never also opens an editor.
 */
export function isSecondaryClick(e: MouseEvent): boolean {
  return e.button !== 0 || (isMac && e.ctrlKey) || Date.now() - openedAt < 500;
}

export function openMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  openedAt = Date.now();
  menu.x = e.clientX;
  menu.y = e.clientY;
  menu.items = items;
  menu.open = true;
}

export function closeMenu() {
  menu.open = false;
  // Once the menu is used or dismissed, the next click is a normal click again.
  openedAt = 0;
}
