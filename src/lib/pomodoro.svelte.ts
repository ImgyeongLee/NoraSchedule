// Pomodoro timer. Lives outside the page so it keeps running while you browse
// other pages. Uses wall-clock deadlines, so it stays accurate in the background.
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { api } from './api';
import { data, toast } from './state.svelte';
import { t } from './i18n.svelte';

export type Phase = 'focus' | 'short' | 'long';

export const PHASES: Record<Phase, { name: () => string; color: string }> = {
  focus: { name: () => t('pomo.focus'), color: 'var(--focus)' },
  short: { name: () => t('pomo.short'), color: 'var(--break)' },
  long: { name: () => t('pomo.long'), color: 'var(--long-break)' },
};

const SETTINGS = ['focusMin', 'shortMin', 'longMin', 'longEvery', 'autoStart'] as const;

class Pomodoro {
  focusMin = $state(25);
  shortMin = $state(5);
  longMin = $state(15);
  longEvery = $state(4);
  autoStart = $state(false);

  phase = $state<Phase>('focus');
  label = $state('');
  completed = $state(0);
  /** Deadline (ms) while running. */
  endsAt = $state<number | null>(null);
  /** Remaining time (ms) while paused or not started. */
  pausedRemaining = $state(25 * 60_000);
  /** Whether the current phase has been started (it may be paused now). */
  started = $state(false);
  now = $state(Date.now());
  private focusStartedTs: number | null = null;

  constructor() {
    setInterval(() => this.tick(), 250);
  }

  get running() {
    return this.endsAt !== null;
  }
  get phaseMs() {
    const min = this.phase === 'focus' ? this.focusMin : this.phase === 'short' ? this.shortMin : this.longMin;
    return min * 60_000;
  }
  get remaining() {
    return this.endsAt !== null ? Math.max(0, this.endsAt - this.now) : this.pausedRemaining;
  }
  get progress() {
    return this.phaseMs > 0 ? this.remaining / this.phaseMs : 0;
  }
  get text() {
    const secs = Math.ceil(this.remaining / 1000);
    return `${String(Math.floor(secs / 60)).padStart(2, '0')}:${String(secs % 60).padStart(2, '0')}`;
  }

  async load() {
    for (const key of SETTINGS) {
      const v = await api.getSetting(`pomodoro.${key}`).catch(() => null);
      if (v === null) continue;
      if (key === 'autoStart') this.autoStart = v === 'true';
      else if (Number(v) > 0) this[key] = Number(v);
    }
    if (!this.started) this.pausedRemaining = this.phaseMs;
  }

  async saveSettings() {
    for (const key of SETTINGS) await api.setSetting(`pomodoro.${key}`, String(this[key])).catch(() => {});
    if (!this.started) this.pausedRemaining = this.phaseMs;
  }

  start() {
    if (this.phase === 'focus' && this.focusStartedTs === null) this.focusStartedTs = Math.floor(Date.now() / 1000);
    this.started = true;
    this.endsAt = Date.now() + this.pausedRemaining;
    // Ask once, early, so the end-of-session notification can be shown.
    isPermissionGranted()
      .then((ok) => {
        if (!ok) requestPermission();
      })
      .catch(() => {});
  }

  pause() {
    this.pausedRemaining = this.remaining;
    this.endsAt = null;
  }

  toggle() {
    if (this.running) this.pause();
    else this.start();
  }

  setPhase(phase: Phase) {
    this.phase = phase;
    this.endsAt = null;
    this.started = false;
    this.focusStartedTs = null;
    this.pausedRemaining = this.phaseMs;
  }

  reset() {
    this.setPhase(this.phase);
  }

  async advance(finished: boolean) {
    let next: Phase = 'focus';
    if (this.phase === 'focus') {
      if (finished) {
        const end = Math.floor(Date.now() / 1000);
        await api.addPomodoroSession(this.focusStartedTs ?? end - this.focusMin * 60, end, this.label).catch(() => {});
        data.version++;
        this.completed++;
      }
      next = this.completed > 0 && this.completed % Math.max(1, this.longEvery) === 0 ? 'long' : 'short';
    }
    this.setPhase(next);
    if (finished && this.autoStart) this.start();
  }

  private tick() {
    this.now = Date.now();
    if (this.endsAt === null || this.now < this.endsAt) return;
    const wasFocus = this.phase === 'focus';
    this.endsAt = null;
    this.advance(true);
    const body = wasFocus ? t('pomo.doneFocus') : t('pomo.doneBreak');
    toast(body, 'success');
    sendNotification({ title: 'Nora Schedule', body });
  }
}

export const pomodoro = new Pomodoro();
