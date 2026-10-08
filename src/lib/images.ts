// User images (D-Day covers, Overview header and cards, stickers). Rust does the optimizing and storage (src-tauri/src/images.rs);
// this file handles picking files and building URLs for stored images.
import { convertFileSrc } from '@tauri-apps/api/core';
import { api, type ImagePurpose } from './api';
import { t } from './i18n.svelte';

/** Dev browser preview keeps imported images as blob URLs (see lib/mock.ts). */
type PreviewWindow = Window & { __noraPreviewImages?: Map<string, string> };

/**
 * URL of a stored image. `variant` gives the same file a separate address: WebKit shares
 * one animation between all <img>s with the same URL, so an animated GIF placed on a page
 * would stall when its thumbnail elsewhere disappears. The server ignores the query.
 */
export function imageUrl(name: string, variant?: string): string {
  const preview = (window as PreviewWindow).__noraPreviewImages?.get(name);
  if (preview) return preview;
  const url = convertFileSrc(name, 'noraimg');
  return variant ? `${url}?${encodeURIComponent(variant)}` : url;
}

/** Re-encodes an image the webview can display but Rust can't decode (e.g. HEIC) as PNG. */
async function reencodeInWebview(file: File): Promise<Uint8Array> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, 2560 / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
  if (!blob) throw new Error('image_unsupported');
  return new Uint8Array(await blob.arrayBuffer());
}

/** Imports a picked/dropped file and returns the stored file name. Throws a translated message. */
export async function importImageFile(file: File, purpose: ImagePurpose = 'cover'): Promise<string> {
  try {
    try {
      return await api.importImage(new Uint8Array(await file.arrayBuffer()), purpose);
    } catch (e) {
      if (String(e) !== 'image_unsupported') throw e;
      return await api.importImage(await reencodeInWebview(file), purpose);
    }
  } catch (e) {
    const code = String(e);
    if (code.includes('image_too_large')) throw new Error(t('img.tooLarge'));
    if (code.includes('image_unsupported') || code.includes('InvalidStateError')) throw new Error(t('img.unsupported'));
    throw e instanceof Error ? e : new Error(code);
  }
}
