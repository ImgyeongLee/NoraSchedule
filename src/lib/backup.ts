// Backup export/import and "delete all data" (Settings → Your data).
import { open, save } from '@tauri-apps/plugin-dialog';
import { api, type BackupManifest } from './api';
import { today } from './dates';
import { t } from './i18n.svelte';
import { toast } from './state.svelte';

const filters = () => [{ name: t('data.fileType'), extensions: ['nora', 'zip'] }];

/** Turns backend error codes into friendly messages. */
export function backupError(e: unknown): string {
  const msg = String(e);
  if (msg.includes('backup_too_new')) return t('data.tooNew');
  if (msg.includes('backup_invalid')) return t('data.invalid');
  return msg;
}

/** Asks where to save, then writes the backup. Returns false if cancelled or failed. */
export async function exportBackup(): Promise<boolean> {
  const path = await save({ defaultPath: `Nora-backup-${today()}.nora`, filters: filters() }).catch(() => null);
  if (!path) return false;
  try {
    await api.exportData(path);
    toast(t('data.exported', { name: path.split(/[\\/]/).pop() ?? path }), 'success');
    return true;
  } catch (e) {
    toast(backupError(e), 'error');
    return false;
  }
}

/** Lets the user pick a backup file and reads its summary. */
export async function pickBackup(): Promise<{ path: string; manifest: BackupManifest } | null> {
  const picked = await open({ multiple: false, directory: false, filters: filters() }).catch(() => null);
  const path = typeof picked === 'string' ? picked : null;
  if (!path) return null;
  try {
    return { path, manifest: await api.inspectBackup(path) };
  } catch (e) {
    toast(backupError(e), 'error');
    return null;
  }
}
