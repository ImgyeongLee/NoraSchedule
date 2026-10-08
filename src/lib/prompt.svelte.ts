// "Just this one / All events" question for repeating events. Rendered by ScopeDialog in App.
import type { Scope } from './api';

export const scopePrompt = $state({
  open: false,
  action: 'delete' as 'delete' | 'save',
  resolve: null as ((scope: Scope | null) => void) | null,
});

/** Asks which part of a repeating series to change. Resolves to null if cancelled. */
export function askScope(action: 'delete' | 'save'): Promise<Scope | null> {
  return new Promise((resolve) => {
    scopePrompt.action = action;
    scopePrompt.resolve = resolve;
    scopePrompt.open = true;
  });
}

export function answerScope(scope: Scope | null) {
  scopePrompt.open = false;
  scopePrompt.resolve?.(scope);
  scopePrompt.resolve = null;
}

export const isRepeatingOccurrence = (e: { repeat: unknown; occurrence: string | null }) => !!e.repeat && !!e.occurrence;
