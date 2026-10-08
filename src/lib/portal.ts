// Moves an element to the end of <body>, so pop-ups are never trapped or clipped by
// the container they were opened from (Overview tiles, dialogs, scroll areas).
//
// Use it on an element *inside* the component's root, never on the root itself:
// Svelte removes a block's own nodes in place, and the action removes the moved one.
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}
