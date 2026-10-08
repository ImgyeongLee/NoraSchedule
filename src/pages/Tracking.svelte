<script lang="ts">
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import { ChevronDown, ChevronLeft, ChevronRight, Clock, Hourglass, Laptop, Pause, Play, ShieldCheck, Trophy, X } from '@lucide/svelte';
  import StatCard from '../components/StatCard.svelte';
  import { api, type Activity, type ActivitySummary, type TrackerSettings, type TrackerStatus } from '../lib/api';
  import { colorForName } from '../lib/colors';
  import { addDays, dayStartTs, fmt, fmtDuration, fmtTs, today } from '../lib/dates';
  import { load, toast } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';
  import { trackState } from '../lib/tracker';

  let day = $state(today());
  let summary = $state<ActivitySummary | null>(null);
  let timeline = $state<Activity[]>([]);
  let status = $state<TrackerStatus | null>(null);
  let settings = $state<TrackerSettings | null>(null);
  let expanded = $state<string | null>(null);
  let newIgnored = $state('');

  async function refresh() {
    const d = day;
    const [s, t, st] = await Promise.all([
      load(api.activitySummary([d], 300), null),
      load(api.activityTimeline(d), []),
      api.trackerStatus().catch(() => null),
    ]);
    if (d !== day) return;
    summary = s;
    timeline = t;
    status = st;
  }

  $effect(() => {
    day;
    refresh();
  });

  onMount(() => {
    load(api.trackerSettings(), null).then((s) => (settings = s));
    const timer = setInterval(refresh, 3000);
    return () => clearInterval(timer);
  });

  async function saveSettings(next: TrackerSettings) {
    try {
      await api.setTrackerSettings(next);
      settings = await api.trackerSettings();
      refresh();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  const dayStart = $derived(dayStartTs(day));
  const pct = (ts: number) => Math.min(100, Math.max(0, ((ts - dayStart) / 86_400) * 100));
  const titlesFor = (app: string) => summary?.titles.filter((t) => t.app === app) ?? [];
  const maxApp = $derived(summary?.apps[0]?.secs ?? 1);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.tracking')}</h1>
      <p class="sub">{t('trk.subtitle')}</p>
    </div>
  </div>

  {#if status && settings}
    {@const s = trackState(status.state)}
    <div class="card live" style:--c={s.color}>
      <div class="live-dot" class:on={status.state === 'tracking'}></div>
      <div class="live-info">
        <div class="live-state">{s.label}</div>
        {#if status.state === 'tracking' || status.state === 'ignored'}
          <div class="live-app truncate">{status.app}</div>
          {#if status.title}<div class="muted truncate">{status.title}</div>{/if}
          {#if status.state === 'tracking'}<div class="faint small">{t('trk.focusedFor', { d: fmtDuration(status.segment_secs) })}</div>{/if}
        {:else}
          <div class="muted">{s.hint}</div>
        {/if}
      </div>
      <button class="btn {settings.paused ? 'primary' : ''}" onclick={() => settings && saveSettings({ ...settings, paused: !settings.paused })}>
        {#if settings.paused}<Play size={16} /> {t('trk.resume')}{:else}<Pause size={16} /> {t('trk.pause')}{/if}
      </button>
    </div>
  {/if}

  <div class="day-nav">
    <button class="icon-btn" onclick={() => (day = addDays(day, -1))} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
    <h2>{day === today() ? t('common.today') : fmt(day, { weekday: 'long', month: 'long', day: 'numeric' })}</h2>
    <button class="icon-btn" onclick={() => (day = addDays(day, 1))} disabled={day >= today()} aria-label={t('common.next')}><ChevronRight size={18} /></button>
    {#if day !== today()}<button class="btn small" onclick={() => (day = today())}>{t('common.today')}</button>{/if}
  </div>

  <div class="stats">
    <StatCard label={t('trk.total')} value={fmtDuration(summary?.total ?? 0)} icon={Clock} />
    <StatCard label={t('trk.appsUsed')} value={String(summary?.apps.length ?? 0)} icon={Laptop} tint="var(--long-break)" />
    <StatCard
      label={t('trk.mostUsed')}
      value={summary?.apps[0]?.app ?? '—'}
      sub={summary?.apps[0] ? fmtDuration(summary.apps[0].secs) : ''}
      icon={Trophy}
      tint="var(--warning)"
    />
  </div>

  <div class="card section">
    <h3>{t('trk.timeline')}</h3>
    <div class="timeline">
      {#each timeline as a, i (i)}
        <div
          class="seg"
          style:left="{pct(a.start)}%"
          style:width="max(2px, {pct(a.end) - pct(a.start)}%)"
          style:background={colorForName(a.app)}
          title="{a.app}{a.title ? ` — ${a.title}` : ''}&#10;{fmtTs(a.start)} – {fmtTs(a.end)}"
        ></div>
      {/each}
    </div>
    <div class="hours">
      {#each [0, 3, 6, 9, 12, 15, 18, 21, 24] as h (h)}<span>{String(h).padStart(2, '0')}</span>{/each}
    </div>
  </div>

  <div class="card section">
    <h3>{t('trk.byApp')}</h3>
    {#if !summary?.apps.length}
      <div class="empty">
        <span class="empty-icon"><Hourglass size={30} /></span>
        <p>{day === today() ? t('trk.noActivityToday') : t('trk.noActivityDay')}</p>
      </div>
    {/if}
    <div class="apps">
      {#each summary?.apps ?? [] as a (a.app)}
        {@const titles = titlesFor(a.app)}
        <div class="app">
          <button class="app-row" onclick={() => (expanded = expanded === a.app ? null : a.app)} disabled={!titles.length}>
            <span class="dot" style:background={colorForName(a.app)}></span>
            <span class="app-name truncate">{a.app}</span>
            <span class="bar"><span style:width="{(a.secs / maxApp) * 100}%" style:background={colorForName(a.app)}></span></span>
            <span class="tabular muted">{fmtDuration(a.secs)}</span>
            {#if titles.length}<span class="chev" class:open={expanded === a.app}><ChevronDown size={16} /></span>{:else}<span class="chev"></span>{/if}
          </button>
          {#if expanded === a.app}
            <div class="titles" transition:slide={{ duration: 160 }}>
              {#each titles.slice(0, 30) as t, i (i)}
                <div class="title-row">
                  <span class="truncate">{t.title}</span>
                  <span class="tabular faint">{fmtDuration(t.secs)}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  {#if settings}
    <div class="card section">
      <h3>{t('trk.settings')}</h3>
      <div class="setting">
        <div>
          <div class="s-title">{t('trk.idleTitle')}</div>
          <div class="muted small">{t('trk.idleBody')}</div>
        </div>
        <select
          class="select narrow"
          value={settings.idle_threshold_secs}
          onchange={(e) => settings && saveSettings({ ...settings, idle_threshold_secs: Number(e.currentTarget.value) })}
        >
          {#each [60, 180, 300, 600, 900, 1800, 0] as v (v)}
            <option value={v}>{v === 0 ? t('trk.never') : v === 60 ? t('trk.oneMinute') : t('trk.minutes', { n: v / 60 })}</option>
          {/each}
        </select>
      </div>
      <div class="setting col">
        <div>
          <div class="s-title">{t('trk.ignoredTitle')}</div>
          <div class="muted small">{t('trk.ignoredBody')}</div>
        </div>
        <div class="ignored">
          {#each settings.ignored_apps as app (app)}
            <span class="chip big-chip">
              {app}
              <button
                onclick={() => settings && saveSettings({ ...settings, ignored_apps: settings.ignored_apps.filter((a) => a !== app) })}
                aria-label={t('trk.stopIgnoring', { app })}><X size={13} /></button
              >
            </span>
          {/each}
          <input
            class="input narrow"
            placeholder={t('trk.ignorePlaceholder')}
            bind:value={newIgnored}
            onkeydown={(e) => {
              if (e.key === 'Enter' && newIgnored.trim() && settings) {
                saveSettings({ ...settings, ignored_apps: [...settings.ignored_apps, newIgnored.trim()] });
                newIgnored = '';
              }
            }}
          />
          {#if status?.app && !settings.ignored_apps.includes(status.app)}
            <button class="btn small" onclick={() => settings && status && saveSettings({ ...settings, ignored_apps: [...settings.ignored_apps, status.app] })}>
              {t('trk.ignoreApp', { app: status.app })}
            </button>
          {/if}
        </div>
      </div>
      <div class="tip">
        <ShieldCheck size={18} />
        <p class="small">{t('trk.privacy')}</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .live {
    display: flex;
    align-items: center;
    gap: 18px;
    margin-bottom: 22px;
    background:
      radial-gradient(circle at 0% 50%, color-mix(in srgb, var(--c) 14%, transparent), transparent 50%),
      var(--surface);
  }
  .live-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--c);
    flex: none;
    margin-left: 6px;
  }
  .live-dot.on {
    animation: live 2s infinite;
  }
  @keyframes live {
    0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--c) 55%, transparent); }
    70% { box-shadow: 0 0 0 12px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }
  .live-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .live-state {
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--c);
  }
  .live-app {
    font-size: 20px;
    font-weight: 700;
  }
  .day-nav {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 14px;
  }
  .day-nav h2 {
    min-width: 120px;
    text-align: center;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
    margin-bottom: 16px;
  }
  .section {
    margin-bottom: 16px;
  }
  .timeline {
    position: relative;
    height: 36px;
    margin-top: 14px;
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .seg {
    position: absolute;
    top: 0;
    bottom: 0;
  }
  .hours {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .apps {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 10px;
  }
  .app-row {
    display: grid;
    grid-template-columns: 10px minmax(120px, 1fr) 2fr 80px 20px;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    border: none;
    border-radius: 12px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .app-row:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .app-row:disabled {
    cursor: default;
  }
  .app-name {
    font-weight: 600;
  }
  .bar {
    height: 8px;
    border-radius: 8px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 8px;
  }
  .chev {
    color: var(--faint);
    display: grid;
    transition: transform 0.15s;
  }
  .chev.open {
    transform: rotate(180deg);
  }
  .titles {
    margin: 0 12px 8px 34px;
    padding: 8px 12px;
    border-radius: 12px;
    background: var(--surface-2);
  }
  .title-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 0;
    font-size: 13px;
  }
  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 0;
    border-bottom: 1px solid var(--border);
  }
  .setting.col {
    flex-direction: column;
    align-items: stretch;
  }
  .s-title {
    font-weight: 650;
  }
  .narrow {
    width: 200px;
    flex: none;
  }
  .ignored {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .big-chip {
    height: 30px;
    padding-right: 4px;
    color: var(--text);
  }
  .big-chip button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 50%;
    background: var(--surface-3);
    cursor: pointer;
  }
  .tip {
    display: flex;
    gap: 12px;
    margin-top: 16px;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .tip p {
    color: var(--text);
  }
  @container main (max-width: 720px) {
    .stats {
      grid-template-columns: minmax(0, 1fr);
    }
    .live {
      flex-wrap: wrap;
    }
    .app-row {
      grid-template-columns: 10px minmax(0, 1fr) 80px 20px;
    }
    .app-row .bar {
      display: none;
    }
    .setting {
      flex-direction: column;
      align-items: stretch;
    }
    .narrow {
      width: 100%;
    }
  }
</style>
