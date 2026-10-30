// New-version notice: compares this app's version with the latest release on GitHub.
// Checked at most once a day (and on demand in Settings → About); can be turned off.
// Nothing is downloaded or installed; the user opens the release page themselves.
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { api } from './api';
import { t } from './i18n.svelte';
import { UPDATES_URL } from './links';
import { toast } from './state.svelte';

const SETTING = 'updates';
const DAY_MS = 24 * 3600 * 1000;

export const updates = $state({
  /** Check by itself when the app starts (once a day). */
  auto: true,
  current: '',
  latest: null as string | null,
  /** The release page of `latest`. */
  url: null as string | null,
  status: 'idle' as 'idle' | 'checking' | 'latest' | 'newer' | 'failed',
  checkedAt: 0,
});

/** "0.10.0" > "0.9.2": compares number by number. */
export function isNewer(latest: string, current: string): boolean {
  const a = latest.split('.').map(Number);
  const b = current.split('.').map(Number);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const d = (a[i] ?? 0) - (b[i] ?? 0);
    if (d) return d > 0;
  }
  return false;
}

async function persist() {
  const { auto, latest, url, checkedAt } = updates;
  await api.setSetting(SETTING, JSON.stringify({ auto, latest, url, checkedAt })).catch(() => {});
}

export async function loadUpdates() {
  updates.current = await getVersion().catch(() => '');
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (saved && typeof saved === 'object') {
      updates.auto = saved.auto !== false;
      updates.checkedAt = Number(saved.checkedAt) || 0;
      if (typeof saved.latest === 'string') updates.latest = saved.latest;
      if (typeof saved.url === 'string') updates.url = saved.url;
    }
  } catch {
    /* defaults */
  }
  if (updates.latest && updates.current) updates.status = isNewer(updates.latest, updates.current) ? 'newer' : 'latest';
}

export const openReleasePage = () => openUrl(updates.url ?? UPDATES_URL).catch((e) => toast(String(e), 'error'));

/** Asks GitHub for the latest version. `manual`: from the Settings button (always checks, reports the result). */
export async function checkForUpdates(manual = false) {
  if (updates.status === 'checking') return;
  if (!manual && (!updates.auto || Date.now() - updates.checkedAt < DAY_MS)) return;
  updates.status = 'checking';
  try {
    if (!updates.current) updates.current = await getVersion();
    const latest = await api.latestRelease();
    updates.checkedAt = Date.now();
    updates.latest = latest?.version ?? null;
    updates.url = latest?.url ?? null;
    const newer = !!latest && isNewer(latest.version, updates.current);
    updates.status = newer ? 'newer' : 'latest';
    await persist();
    if (newer) {
      toast(t('upd.available', { v: latest!.version }), 'info', { label: t('upd.see'), run: openReleasePage });
    } else if (manual) {
      toast(t('upd.upToDate'), 'success');
    }
  } catch {
    // Offline or GitHub unreachable: try again next time, quietly unless asked.
    updates.status = 'failed';
    if (manual) toast(t('upd.failed'), 'error');
  }
}

export function setAutoCheck(on: boolean) {
  updates.auto = on;
  persist();
}
