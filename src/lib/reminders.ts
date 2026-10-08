// Event reminders: the backend emits `reminder` when an event's reminder time arrives
// (see src-tauri/src/reminders.rs); this shows the notification and plays a chime.
import { listen } from '@tauri-apps/api/event';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import type { Reminder } from './api';
import { fmtDuration, timeOf } from './dates';
import { t } from './i18n.svelte';
import { toast } from './state.svelte';

/** Reminder choices in minutes; null means none. Anything else is a custom value. */
export const REMINDER_PRESETS: (number | null)[] = [null, 5, 10, 30, 60];
/** The longest custom reminder (matches MAX_REMINDER_DAYS in Rust). */
export const MAX_REMINDER_MINUTES = 7 * 24 * 60;

export const reminderLabel = (minutes: number | null) =>
  minutes === null ? t('remind.none') : t('remind.before', { d: fmtDuration(minutes * 60) });

let audio: AudioContext | null = null;

/** A short two-note chime, made with Web Audio so no sound file is needed. */
export function playChime() {
  try {
    audio ??= new AudioContext();
    if (audio.state === 'suspended') audio.resume();
    const now = audio.currentTime;
    [880, 1318.5].forEach((freq, i) => {
      const osc = audio!.createOscillator();
      const gain = audio!.createGain();
      osc.type = 'sine';
      osc.frequency.value = freq;
      const start = now + i * 0.18;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.25, start + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.001, start + 0.6);
      osc.connect(gain).connect(audio!.destination);
      osc.start(start);
      osc.stop(start + 0.65);
    });
  } catch {
    /* audio unavailable; the notification still shows */
  }
}

function notify({ event, starts_at }: Reminder) {
  const minutes = Math.max(0, Math.round((new Date(starts_at).getTime() - Date.now()) / 60_000));
  const when = event.all_day ? t('common.allDay') : timeOf(event.start);
  const body = minutes > 0 ? t('remind.startsIn', { d: fmtDuration(minutes * 60), time: when }) : t('remind.startsNow', { time: when });
  sendNotification({ title: event.title, body });
  toast(`${event.title} · ${body}`, 'info');
  playChime();
}

export async function initReminders() {
  if (!('__TAURI_INTERNALS__' in window)) return;
  try {
    if (!(await isPermissionGranted())) await requestPermission();
  } catch {
    /* notifications unavailable; toasts and the chime still work */
  }
  // Browsers keep audio locked until the first user gesture; unlock it then.
  window.addEventListener('pointerdown', () => { audio ??= new AudioContext(); audio.resume(); }, { once: true });
  await listen<Reminder>('reminder', (e) => notify(e.payload));
}
