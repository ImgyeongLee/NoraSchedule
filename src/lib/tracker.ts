import type { TrackState } from './api';
import { t } from './i18n.svelte';

export const TRACK_COLORS: Record<TrackState, string> = {
  starting: 'var(--faint)',
  tracking: 'var(--success)',
  idle: 'var(--warning)',
  paused: 'var(--faint)',
  untracked: 'var(--faint)',
  unavailable: 'var(--danger)',
};

export function trackState(state: TrackState) {
  return { label: t(`track.${state}`), hint: t(`track.${state}.hint`), color: TRACK_COLORS[state] };
}
