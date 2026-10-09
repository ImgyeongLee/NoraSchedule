// TRPG roles: a played session can record several at once — GM and/or PL, handout
// (HO1…) and player-character (PC1…) slots, or anything typed in. Stored comma-separated.

/** Offered as one-click choices in the editor. */
export const ROLE_PRESETS = ['gm', 'pl', 'HO1', 'HO2', 'HO3', 'HO4', 'HO5', 'PC1', 'PC2', 'PC3', 'PC4', 'PC5'];

export const rolesOf = (role: string): string[] =>
  role
    .split(',')
    .map((r) => r.trim())
    .filter(Boolean);

/** Joins roles back into the stored form, dropping empties and duplicates. */
export function joinRoles(roles: string[]): string {
  const out: string[] = [];
  for (const r of roles.map((x) => x.trim().replace(/,/g, ' ')).filter(Boolean)) {
    if (!out.some((o) => o.toLowerCase() === r.toLowerCase())) out.push(r);
  }
  return out.join(',');
}

/** "gm" → "GM"; HO/PC and custom roles are shown as typed. */
export const roleLabel = (r: string) => (r === 'gm' || r === 'pl' ? r.toUpperCase() : r);
