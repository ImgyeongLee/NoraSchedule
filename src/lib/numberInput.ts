// Number fields as plain text fields. <input type="number"> (and inputmode="numeric"/"decimal")
// tells the system keyboard it is a number field, so a Korean/Japanese keyboard switches to
// English there and stays that way for the next field (a memo, a title...). These fields are
// type="text" instead, with `use:numeric` keeping what is typed to a number.

export type NumericOptions = { decimal?: boolean; max?: number };

/** Digits (and one decimal point when allowed), clamped to `max`. Full-width digits count too. */
export function cleanNumber(raw: string, { decimal = false, max }: NumericOptions = {}) {
  const half = raw.replace(/[０-９．]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 0xfee0));
  let out: string;
  if (decimal) {
    const [whole, ...rest] = half.replace(/[^\d.]/g, '').split('.');
    out = rest.length ? `${whole}.${rest.join('')}` : whole;
  } else {
    out = half.replace(/\D/g, '');
  }
  if (max !== undefined && out !== '' && Number(out) > max) out = String(max);
  return out;
}

/** What a number field's binding used to give: empty is null, anything else a number. */
export const toNum = (value: string) => (value === '' ? null : Number(value)) as number;

/**
 * Keeps a text field to a number while typing (it runs before the field's own input handlers,
 * so bindings only ever see the cleaned text), and keeps ↑/↓ stepping by one.
 */
export function numeric(node: HTMLInputElement, options: NumericOptions = {}) {
  let opts = options;
  node.autocomplete = 'off';
  node.spellcheck = false;

  function clean() {
    const before = node.value;
    const next = cleanNumber(before, opts);
    if (next === before) return;
    // Keep the caret where it was, minus whatever was dropped in front of it.
    const caret = node.selectionStart ?? before.length;
    const at = Math.min(next.length, cleanNumber(before.slice(0, caret), { decimal: opts.decimal }).length);
    node.value = next;
    try {
      node.setSelectionRange(at, at);
    } catch {
      /* not focused; fine */
    }
  }
  function oninput(e: Event) {
    // Leave a Korean/Japanese composition alone until it is finished.
    if (!(e as InputEvent).isComposing) clean();
  }
  function oncompositionend() {
    if (cleanNumber(node.value, opts) !== node.value) {
      clean();
      node.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }
  function onkeydown(e: KeyboardEvent) {
    if ((e.key !== 'ArrowUp' && e.key !== 'ArrowDown') || e.isComposing) return;
    e.preventDefault();
    const now = Number(node.value) || 0;
    let next = Math.max(0, e.key === 'ArrowUp' ? now + 1 : now - 1);
    if (opts.max !== undefined) next = Math.min(opts.max, next);
    node.value = String(opts.decimal ? Math.round(next * 100) / 100 : Math.round(next));
    node.dispatchEvent(new Event('input', { bubbles: true }));
  }

  // Capture: runs before the bubbling listeners that bindings and oninput use.
  node.addEventListener('input', oninput, true);
  node.addEventListener('compositionend', oncompositionend);
  node.addEventListener('keydown', onkeydown);
  return {
    update(next: NumericOptions = {}) {
      opts = next;
    },
    destroy() {
      node.removeEventListener('input', oninput, true);
      node.removeEventListener('compositionend', oncompositionend);
      node.removeEventListener('keydown', onkeydown);
    },
  };
}
