<script lang="ts">
  import { onMount } from 'svelte';
  import { CircleCheck, Pause, Play, Plus, Repeat, RotateCcw, Volume2, X } from '@lucide/svelte';
  import { BASE_PRESETS, MAX_SECONDS, fmtLength, timer } from '../lib/timer.svelte';
  import { playChime } from '../lib/reminders';
  import { t } from '../lib/i18n.svelte';

  onMount(() => {
    timer.load();
  });

  const R = 120;
  const C = 2 * Math.PI * R;

  // Typing a time: minutes and seconds.
  let minutes = $state('');
  let seconds = $state('');
  function applyTyped() {
    const total = (Number(minutes) || 0) * 60 + (Number(seconds) || 0);
    if (total <= 0) return;
    timer.set(Math.min(MAX_SECONDS, total));
    minutes = '';
    seconds = '';
  }

  const presets = $derived([...BASE_PRESETS.map((s) => ({ s, custom: false })), ...timer.presets.map((s) => ({ s, custom: true }))]);
  const isPreset = $derived(BASE_PRESETS.includes(timer.seconds) || timer.presets.includes(timer.seconds));

  function onkeydown(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el.isContentEditable) return;
    if (e.key === ' ') {
      e.preventDefault();
      if (timer.finished) timer.done();
      else timer.toggle();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.timer')}</h1>
      <p class="sub">{t('timer.subtitle')}</p>
    </div>
  </div>

  <div class="layout">
    <div class="card main" class:ringing={timer.finished}>
      <div class="ring">
        <svg viewBox="0 0 280 280" width="280" height="280">
          <circle cx="140" cy="140" r={R} class="track" />
          <circle cx="140" cy="140" r={R} class="progress" stroke-dasharray={C} stroke-dashoffset={C * (1 - timer.progress)} transform="rotate(-90 140 140)" />
        </svg>
        <div class="center">
          <div class="time tabular">{timer.text}</div>
          <div class="sub-label muted small">
            {#if timer.finished}{t('timer.finished')}
            {:else if timer.nonstop && timer.rounds > 0}<Repeat size={13} /> {t('timer.roundN', { n: timer.rounds + 1 })}
            {:else}{fmtLength(timer.seconds)}{/if}
          </div>
        </div>
      </div>

      <div class="controls">
        {#if timer.finished}
          <button class="btn primary big done-btn" onclick={() => timer.done()}><CircleCheck size={20} /> {t('timer.done')}</button>
        {:else}
          <button class="icon-round" onclick={() => timer.done()} disabled={!timer.started} title={t('pomo.reset')} aria-label={t('pomo.reset')}><RotateCcw size={20} /></button>
          <button class="btn primary big play" onclick={() => timer.toggle()}>
            {#if timer.running}<Pause size={20} /> {t('common.pause')}{:else}<Play size={20} /> {timer.started ? t('common.resume') : t('pomo.start')}{/if}
          </button>
          <span class="icon-round placeholder"></span>
        {/if}
      </div>
      <p class="faint small key-hint">{t('timer.keyHint')}</p>
    </div>

    <div class="side">
      <div class="card">
        <h3>{t('timer.presets')}</h3>
        <div class="presets">
          {#each presets as p (p.s)}
            <span class="preset-wrap">
              <button class="preset" class:on={timer.seconds === p.s} onclick={() => timer.set(p.s)}>{fmtLength(p.s)}</button>
              {#if p.custom}
                <button class="preset-del" onclick={() => timer.removePreset(p.s)} title={t('timer.removePreset')} aria-label={t('timer.removePreset')}><X size={11} /></button>
              {/if}
            </span>
          {/each}
        </div>
        <div class="custom">
          <span class="faint small">{t('timer.typeTime')}</span>
          <div class="time-input">
            <input class="input tabular" type="number" inputmode="numeric" min="0" max="1440" placeholder="0" bind:value={minutes}
              onkeydown={(e) => e.key === 'Enter' && applyTyped()} aria-label={t('timer.minutes')} />
            <span class="unit">{t('timer.minUnit')}</span>
            <input class="input tabular" type="number" inputmode="numeric" min="0" max="59" placeholder="0" bind:value={seconds}
              onkeydown={(e) => e.key === 'Enter' && applyTyped()} aria-label={t('timer.seconds')} />
            <span class="unit">{t('timer.secUnit')}</span>
            <button class="btn" onclick={applyTyped} disabled={!(Number(minutes) > 0 || Number(seconds) > 0)}>{t('timer.set')}</button>
          </div>
          {#if !isPreset}
            <button class="btn small save-preset" onclick={() => timer.addPreset()}><Plus size={14} /> {t('timer.savePreset', { time: fmtLength(timer.seconds) })}</button>
          {/if}
        </div>
      </div>

      <div class="card">
        <h3>{t('pomo.settings')}</h3>
        <label class="setting">
          <span class="setting-text">
            <span class="strong">{t('timer.nonstop')}</span>
            <span class="muted small">{t('timer.nonstopBody')}</span>
          </span>
          <input type="checkbox" class="switch" bind:checked={timer.nonstop} onchange={() => timer.save()} />
        </label>
        <label class="setting">
          <span class="setting-text">
            <span class="strong">{t('timer.sound')}</span>
            <span class="muted small">{t('timer.soundBody')}</span>
          </span>
          <span class="sound-opt">
            <button type="button" class="icon-btn" onclick={() => playChime(3)} title={t('pomo.soundTest')} aria-label={t('pomo.soundTest')}><Volume2 size={16} /></button>
            <input type="checkbox" class="switch" bind:checked={timer.sound} onchange={() => timer.save()} />
          </span>
        </label>
      </div>
    </div>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .main {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    padding: 32px 20px 24px;
  }
  .ring {
    position: relative;
    display: grid;
    place-items: center;
  }
  .ring svg {
    display: block;
    max-width: 100%;
    height: auto;
  }
  .track {
    fill: none;
    stroke: var(--surface-2);
    stroke-width: 14;
  }
  .progress {
    fill: none;
    stroke: var(--primary);
    stroke-width: 14;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.2s linear;
  }
  .ringing .progress,
  .ringing .track {
    stroke: var(--success);
    animation: blink 1s ease-in-out infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0.45;
    }
  }
  .center {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .time {
    font-size: 52px;
    font-weight: 800;
    letter-spacing: -0.02em;
  }
  .sub-label {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .icon-round {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }
  .icon-round:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .icon-round.placeholder {
    visibility: hidden;
  }
  .play,
  .done-btn {
    min-width: 150px;
  }
  .btn.primary.done-btn {
    background: var(--success);
    box-shadow: 0 4px 14px color-mix(in srgb, var(--success) 35%, transparent);
    animation: pop 1s ease-in-out infinite;
  }
  @keyframes pop {
    50% {
      transform: scale(1.04);
    }
  }
  .key-hint {
    margin: 0;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .side h3 {
    margin-bottom: 12px;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .preset-wrap {
    position: relative;
  }
  .preset {
    height: 38px;
    padding: 0 16px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    font-weight: 650;
    cursor: pointer;
  }
  .preset:hover {
    border-color: color-mix(in srgb, var(--primary) 50%, var(--border));
  }
  .preset.on {
    border-color: var(--primary);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .preset-del {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 50%;
    background: var(--surface-3);
    color: var(--muted);
    cursor: pointer;
    opacity: 0;
  }
  .preset-wrap:hover .preset-del {
    opacity: 1;
  }
  .custom {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 16px;
  }
  .time-input {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .time-input .input {
    width: 72px;
    text-align: center;
  }
  .time-input .btn {
    height: 40px;
    margin-left: 4px;
  }
  .unit {
    color: var(--muted);
    font-weight: 600;
  }
  .save-preset {
    align-self: flex-start;
  }
  input[type='number']::-webkit-inner-spin-button,
  input[type='number']::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .setting {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    cursor: pointer;
  }
  .setting + .setting {
    border-top: 1px solid var(--border);
  }
  .setting-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .strong {
    font-weight: 650;
  }
  .sound-opt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  @container main (max-width: 860px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
