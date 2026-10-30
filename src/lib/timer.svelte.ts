// Simple countdown timer. Lives outside the page so it keeps running while you browse other
// pages, and uses wall-clock deadlines so it stays accurate in the background.
// When it ends it rings; "Done" puts it back to the time last set. In non-stop mode it rings
// and starts the same time again right away.
import { sendNotification } from '@tauri-apps/plugin-notification';
import { api } from './api';
import { t } from './i18n.svelte';
import { playChime } from './reminders';
import { toast } from './state.svelte';

/** Built-in presets, in seconds. */
export const BASE_PRESETS = [60, 180, 300, 600];
/** Longest timer: 24 hours. */
export const MAX_SECONDS = 24 * 3600;

const SETTING = 'timer';

class Timer {
  /** The time last set, in seconds; "Done" and non-stop go back to it. */
  seconds = $state(300);
  /** Deadline (ms) while running. */
  endsAt = $state<number | null>(null);
  /** Time left (ms) while paused or not started. */
  pausedRemaining = $state(300_000);
  /** Rang and waiting for "Done" (not in non-stop mode). */
  finished = $state(false);
  /** Rounds completed in a row in non-stop mode. */
  rounds = $state(0);
  nonstop = $state(false);
  sound = $state(true);
  /** The user's own presets, in seconds. */
  presets = $state<number[]>([]);
  now = $state(Date.now());
  private loaded = false;

  constructor() {
    setInterval(() => this.tick(), 200);
  }

  get running() {
    return this.endsAt !== null;
  }
  get remaining() {
    return this.endsAt !== null ? Math.max(0, this.endsAt - this.now) : this.pausedRemaining;
  }
  get progress() {
    return this.seconds > 0 ? this.remaining / (this.seconds * 1000) : 0;
  }
  /** "05:00", or "1:05:00" past an hour. */
  get text() {
    return fmtClock(Math.ceil(this.remaining / 1000));
  }
  /** Started and not back at the full time. */
  get started() {
    return this.running || this.finished || this.pausedRemaining !== this.seconds * 1000;
  }

  async load() {
    if (this.loaded) return;
    this.loaded = true;
    try {
      const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
      if (saved && typeof saved === 'object') {
        if (Array.isArray(saved.presets)) this.presets = saved.presets.filter((s: unknown) => Number.isInteger(s) && (s as number) > 0).slice(0, 12);
        this.nonstop = saved.nonstop === true;
        this.sound = saved.sound !== false;
        if (Number.isInteger(saved.seconds) && saved.seconds > 0 && !this.started) this.set(saved.seconds, false);
      }
    } catch {
      /* defaults */
    }
  }

  save() {
    const value = JSON.stringify({ presets: this.presets, nonstop: this.nonstop, sound: this.sound, seconds: this.seconds });
    api.setSetting(SETTING, value).catch(() => {});
  }

  /** Sets the time (stops a running timer). */
  set(seconds: number, persist = true) {
    this.seconds = Math.min(MAX_SECONDS, Math.max(1, Math.round(seconds)));
    this.endsAt = null;
    this.finished = false;
    this.rounds = 0;
    this.pausedRemaining = this.seconds * 1000;
    if (persist) this.save();
  }

  start() {
    if (this.finished) this.done();
    if (this.pausedRemaining <= 0) this.pausedRemaining = this.seconds * 1000;
    this.endsAt = Date.now() + this.pausedRemaining;
  }

  pause() {
    this.pausedRemaining = this.remaining;
    this.endsAt = null;
  }

  toggle() {
    if (this.running) this.pause();
    else this.start();
  }

  /** "Done" after it rang, or reset: back to the time last set. */
  done() {
    this.endsAt = null;
    this.finished = false;
    this.rounds = 0;
    this.pausedRemaining = this.seconds * 1000;
  }

  addPreset(seconds = this.seconds) {
    if (BASE_PRESETS.includes(seconds) || this.presets.includes(seconds)) return;
    this.presets = [...this.presets, seconds].sort((a, b) => a - b);
    this.save();
  }

  removePreset(seconds: number) {
    this.presets = this.presets.filter((s) => s !== seconds);
    this.save();
  }

  private tick() {
    this.now = Date.now();
    if (this.endsAt === null || this.now < this.endsAt) return;
    const body = t('timer.doneBody', { time: fmtLength(this.seconds) });
    if (this.sound) playChime(3);
    sendNotification({ title: t('timer.doneTitle'), body });
    if (this.nonstop) {
      // Start the same time again right away, counting from when it ended.
      this.rounds++;
      this.endsAt += this.seconds * 1000;
      if (this.endsAt <= this.now) this.endsAt = this.now + this.seconds * 1000;
      toast(`${body} · ${t('timer.roundN', { n: this.rounds + 1 })}`, 'success');
    } else {
      this.endsAt = null;
      this.pausedRemaining = 0;
      this.finished = true;
      toast(body, 'success');
    }
  }
}

export const timer = new Timer();

/** 300 → "05:00", 3900 → "1:05:00". */
export function fmtClock(total: number): string {
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = String(m).padStart(2, '0');
  const ss = String(s).padStart(2, '0');
  return h ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

/** 300 → "5 min", 90 → "1 min 30 s" (for presets and messages). */
export function fmtLength(total: number): string {
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return [h ? t('timer.h', { n: h }) : '', m ? t('timer.m', { n: m }) : '', s ? t('timer.s', { n: s }) : ''].filter(Boolean).join(' ');
}
