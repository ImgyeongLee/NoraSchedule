<script lang="ts">
  import { Coffee, Flame, Pause, Play, RotateCcw, SkipForward } from '@lucide/svelte';
  import { api, type PomodoroSession } from '../lib/api';
  import { dayStartTs, fmtDuration, fmtTs, today } from '../lib/dates';
  import { PHASES, pomodoro, type Phase } from '../lib/pomodoro.svelte';
  import { data, load } from '../lib/state.svelte';
  import { t, type Key } from '../lib/i18n.svelte';

  let sessions = $state<PomodoroSession[]>([]);
  let openTodos = $state<string[]>([]);

  $effect(() => {
    data.version;
    const start = dayStartTs(today());
    load(api.pomodoroSessions(start, start + 86_400), []).then((s) => (sessions = s));
    load(api.todos(), []).then((t) => (openTodos = t.filter((x) => !x.done).map((x) => x.title)));
  });

  const R = 120;
  const C = 2 * Math.PI * R;
  const focused = $derived(sessions.reduce((sum, s) => sum + s.ended_at - s.started_at, 0));
  const color = $derived(PHASES[pomodoro.phase].color);

  const settings: { key: 'focusMin' | 'shortMin' | 'longMin' | 'longEvery'; label: Key; unit: Key; max: number }[] = [
    { key: 'focusMin', label: 'pomo.focus', unit: 'pomo.min', max: 180 },
    { key: 'shortMin', label: 'pomo.short', unit: 'pomo.min', max: 60 },
    { key: 'longMin', label: 'pomo.long', unit: 'pomo.min', max: 90 },
    { key: 'longEvery', label: 'pomo.longAfter', unit: 'pomo.sessions', max: 12 },
  ];

  function nudge(key: (typeof settings)[number]['key'], delta: number, max: number) {
    pomodoro[key] = Math.min(max, Math.max(1, pomodoro[key] + delta));
    pomodoro.saveSettings();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.code === 'Space' && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) {
      e.preventDefault();
      pomodoro.toggle();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.pomodoro')}</h1>
      <p class="sub">{t('pomo.subtitle')}</p>
    </div>
  </div>

  <div class="layout">
    <div class="card timer" style:--c={color}>
      <div class="segmented">
        {#each Object.entries(PHASES) as [id, p] (id)}
          <button class:active={pomodoro.phase === id} onclick={() => pomodoro.setPhase(id as Phase)}>{p.name()}</button>
        {/each}
      </div>

      <div class="ring">
        <svg viewBox="0 0 280 280" width="280" height="280">
          <circle cx="140" cy="140" r={R} class="track" />
          <circle
            cx="140"
            cy="140"
            r={R}
            class="progress"
            stroke-dasharray={C}
            stroke-dashoffset={C * (1 - pomodoro.progress)}
            transform="rotate(-90 140 140)"
          />
        </svg>
        <div class="center">
          <div class="time tabular">{pomodoro.text}</div>
          <div class="phase">{#if pomodoro.phase === 'focus'}<Flame size={16} />{:else}<Coffee size={16} />{/if} {PHASES[pomodoro.phase].name()}</div>
        </div>
      </div>

      <div class="controls">
        <button class="icon-round" onclick={() => pomodoro.reset()} title={t('pomo.reset')}><RotateCcw size={20} /></button>
        <button class="btn primary big play" onclick={() => pomodoro.toggle()}>
          {#if pomodoro.running}<Pause size={20} /> {t('common.pause')}{:else}<Play size={20} /> {pomodoro.started ? t('common.resume') : t('pomo.start')}{/if}
        </button>
        <button class="icon-round" onclick={() => pomodoro.advance(false)} title={t('pomo.skip')}><SkipForward size={20} /></button>
      </div>

      <input class="input focus-input" list="todo-titles" placeholder={t('pomo.focusPlaceholder')} bind:value={pomodoro.label} />
      <datalist id="todo-titles">
        {#each openTodos as t, i (i)}<option value={t}></option>{/each}
      </datalist>
      <p class="faint small">
        {t('pomo.hintKey', { key: 'Space' })} · {t('pomo.hintSession', { n: (pomodoro.completed % pomodoro.longEvery) + 1, total: pomodoro.longEvery })}
      </p>
    </div>

    <div class="side">
      <div class="card stats">
        <div class="stat">
          <div class="icon focus"><Flame size={18} /></div>
          <div><div class="big">{sessions.length}</div><div class="muted small">{t('pomo.sessionsToday')}</div></div>
        </div>
        <div class="stat">
          <div class="icon rest"><Coffee size={18} /></div>
          <div><div class="big">{fmtDuration(focused)}</div><div class="muted small">{t('pomo.focusedToday')}</div></div>
        </div>
      </div>

      <div class="card">
        <h3>{t('pomo.settings')}</h3>
        <div class="settings">
          {#each settings as s (s.key)}
            <div class="setting">
              <span>{t(s.label)}</span>
              <div class="stepper">
                <button onclick={() => nudge(s.key, -1, s.max)} aria-label={t('pomo.decrease')}>−</button>
                <span class="tabular">{pomodoro[s.key]} <span class="faint small">{t(s.unit)}</span></span>
                <button onclick={() => nudge(s.key, 1, s.max)} aria-label={t('pomo.increase')}>+</button>
              </div>
            </div>
          {/each}
          <label class="setting">
            <span>{t('pomo.auto')}</span>
            <input type="checkbox" class="switch" bind:checked={pomodoro.autoStart} onchange={() => pomodoro.saveSettings()} />
          </label>
        </div>
      </div>

      <div class="card">
        <h3>{t('pomo.todaySessions')}</h3>
        {#if sessions.length === 0}
          <p class="muted small" style="margin-top: 8px">{t('pomo.noSessions')}</p>
        {/if}
        <div class="sessions">
          {#each [...sessions].reverse() as s, i (i)}
            <div class="session">
              <span class="session-icon"><Flame size={15} /></span>
              <span class="tabular muted">{fmtTs(s.started_at)} – {fmtTs(s.ended_at)}</span>
              <span class="truncate">{s.label || t('pomo.focus')}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 20px;
    align-items: start;
  }
  .timer {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 22px;
    padding: 32px;
    background:
      radial-gradient(circle at 50% 40%, color-mix(in srgb, var(--c) 10%, transparent), transparent 65%),
      var(--surface);
  }
  .ring {
    position: relative;
    width: 280px;
    height: 280px;
  }
  .track {
    fill: none;
    stroke: var(--surface-2);
    stroke-width: 16;
  }
  .progress {
    fill: none;
    stroke: var(--c);
    stroke-width: 16;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.3s linear, stroke 0.3s;
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .time {
    font-size: 62px;
    font-weight: 750;
    letter-spacing: -0.03em;
  }
  .phase {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--c);
    font-weight: 650;
  }
  .session-icon {
    display: grid;
    color: var(--focus);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .play {
    min-width: 150px;
    background: var(--c);
    box-shadow: 0 8px 22px color-mix(in srgb, var(--c) 40%, transparent);
  }
  .play:hover {
    background: color-mix(in srgb, var(--c) 88%, #000);
  }
  .icon-round {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .icon-round:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .focus-input {
    max-width: 380px;
    text-align: center;
    height: 44px;
    border-radius: var(--radius);
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .stats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .stat {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .icon {
    width: 40px;
    height: 40px;
    border-radius: 13px;
    display: grid;
    place-items: center;
  }
  .icon.focus {
    color: var(--focus);
    background: color-mix(in srgb, var(--focus) 14%, transparent);
  }
  .icon.rest {
    color: var(--break);
    background: color-mix(in srgb, var(--break) 14%, transparent);
  }
  .big {
    font-size: 22px;
    font-weight: 750;
  }
  .settings {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 12px;
  }
  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-weight: 550;
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px;
    border-radius: 12px;
    background: var(--surface-2);
  }
  .stepper button {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 9px;
    background: var(--surface);
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
  }
  .stepper > span {
    min-width: 92px;
    text-align: center;
    font-weight: 650;
  }
  .sessions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 10px;
    max-height: 220px;
    overflow-y: auto;
  }
  .session {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--surface-2);
  }
  @container main (max-width: 860px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @container main (max-width: 480px) {
    .timer {
      padding: 22px 14px;
    }
    .ring {
      transform: scale(0.85);
      margin: -20px 0;
    }
    .stats {
      grid-template-columns: 1fr;
    }
  }
</style>
