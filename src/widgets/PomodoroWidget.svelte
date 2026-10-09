<script lang="ts">
  import { Pause, Play, RotateCcw, Timer } from '@lucide/svelte';
  import { PHASES, pomodoro } from '../lib/pomodoro.svelte';
  import { t } from '../lib/i18n.svelte';
  import { ui } from '../lib/state.svelte';

  const R = 44;
  const C = 2 * Math.PI * R;
</script>

<div class="widget" style:--c={PHASES[pomodoro.phase].color}>
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'pomodoro')}><span class="w-icon"><Timer size={15} /></span>{t('w.pomodoro')}</button>
  </div>
  <div class="body">
    <div class="ring">
      <svg viewBox="0 0 104 104" width="104" height="104">
        <circle cx="52" cy="52" r={R} class="track" />
        <circle cx="52" cy="52" r={R} class="progress" stroke-dasharray={C} stroke-dashoffset={C * (1 - pomodoro.progress)} transform="rotate(-90 52 52)" />
      </svg>
      <span class="time tabular">{pomodoro.text}</span>
    </div>
    <div class="side">
      <span class="phase">{PHASES[pomodoro.phase].name()}</span>
      <div class="buttons">
        <button class="btn primary play" onclick={() => pomodoro.toggle()}>
          {#if pomodoro.running}<Pause size={16} />{:else}<Play size={16} />{/if}
          {pomodoro.running ? t('common.pause') : pomodoro.started ? t('common.resume') : t('pomo.start')}
        </button>
        {#if pomodoro.started}
          <button class="icon-btn" onclick={() => pomodoro.reset()} title={t('pomo.reset')}><RotateCcw size={16} /></button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .body {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 0;
  }
  .ring {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
  }
  .ring svg {
    display: block;
  }
  .time {
    position: absolute;
    font-size: 19px;
    font-weight: 750;
  }
  .track {
    fill: none;
    stroke: var(--surface-2);
    stroke-width: 9;
  }
  .progress {
    fill: none;
    stroke: var(--c);
    stroke-width: 9;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.3s linear;
  }
  .side {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .phase {
    color: var(--c);
    font-weight: 700;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  /* Longer labels (e.g. Japanese) must stay inside a small tile. */
  .play {
    min-width: 0;
    max-width: 100%;
    padding-inline: 14px;
    background: var(--c);
    box-shadow: 0 4px 14px color-mix(in srgb, var(--c) 35%, transparent);
  }
  .play :global(svg) {
    flex: none;
  }
</style>
