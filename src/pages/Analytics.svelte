<script lang="ts">
  import { CalendarDays, CircleCheck, Clock, Flame, ListTodo, Target, TriangleAlert, Trophy } from '@lucide/svelte';
  import StatCard from '../components/StatCard.svelte';
  import BarChart from '../components/BarChart.svelte';
  import BarList from '../components/BarList.svelte';
  import { api, type ActivitySummary, type CalEvent, type DDay, type PomodoroSession, type Todo, type TodoGroup } from '../lib/api';
  import { colorForName, hex } from '../lib/colors';
  import { addDays, byDdayTarget, dayStartTs, ddayLabel, ddayUpcoming, fmt, fmtDuration, fmtHours, range, today, tsToDate } from '../lib/dates';
  import { data, load } from '../lib/state.svelte';
  import { isOverdue } from '../lib/deadline';
  import { t } from '../lib/i18n.svelte';

  type Tab = 'all' | 'todos' | 'work';
  let tab = $state<Tab>('all');
  let span = $state(7);

  let summary = $state<ActivitySummary | null>(null);
  let todos = $state<Todo[]>([]);
  let groups = $state<TodoGroup[]>([]);
  let sessions = $state<PomodoroSession[]>([]);
  let upcoming = $state<CalEvent[]>([]);
  let ddays = $state<DDay[]>([]);

  const days = $derived(range(addDays(today(), -(span - 1)), span));

  $effect(() => {
    data.version;
    const d = days;
    const from = dayStartTs(d[0]);
    const to = dayStartTs(addDays(d[d.length - 1], 1));
    load(api.activitySummary(d, 30), null).then((s) => (summary = s));
    load(api.todos(), []).then((t) => (todos = t));
    load(api.todoGroups(), []).then((g) => (groups = g));
    load(api.pomodoroSessions(from, to), []).then((s) => (sessions = s));
    load(api.eventsBetween(today(), addDays(today(), 6)), []).then((e) => (upcoming = e));
    load(api.ddays(), []).then((x) => (ddays = x));
  });

  const label = (d: string) => (span <= 7 ? fmt(d, { weekday: 'short' }) : fmt(d, { month: 'numeric', day: 'numeric' }));
  const perDay = <T,>(items: T[], dayOf: (t: T) => string | null, value: (t: T) => number = () => 1) =>
    days.map((d) => ({ label: label(d), value: items.filter((t) => dayOf(t) === d).reduce((s, t) => s + value(t), 0) }));

  const workPerDay = $derived(days.map((d, i) => ({ label: label(d), value: summary?.per_day[i] ?? 0 })));
  const focusPerDay = $derived(perDay(sessions, (s) => tsToDate(s.started_at), (s) => s.ended_at - s.started_at));
  const donePerDay = $derived(perDay(todos, (t) => (t.done && t.completed_at ? tsToDate(t.completed_at) : null)));
  const createdPerDay = $derived(perDay(todos, (t) => tsToDate(t.created_at)));

  const todayStr = today();
  const firstTs = $derived(dayStartTs(days[0]));
  const doneTotal = $derived(todos.filter((t) => t.done).length);
  const openTotal = $derived(todos.length - doneTotal);
  const overdue = $derived(todos.filter(isOverdue).length);
  const doneInRange = $derived(todos.filter((t) => t.completed_at && t.completed_at >= firstTs).length);
  const rate = $derived(todos.length ? Math.round((doneTotal / todos.length) * 100) : 0);
  const focusTotal = $derived(sessions.reduce((s, x) => s + x.ended_at - x.started_at, 0));
  const nextDday = $derived(byDdayTarget(ddays.filter((d) => ddayUpcoming(d, todayStr)), todayStr)[0]);
  const activeDays = $derived(Math.max(1, summary?.per_day.filter((s) => s > 0).length ?? 1));
  const busiest = $derived.by(() => {
    if (!summary) return null;
    let best = -1;
    summary.per_day.forEach((s, i) => (best = best < 0 || s > summary!.per_day[best] ? i : best));
    return best >= 0 && summary.per_day[best] > 0 ? { day: days[best], secs: summary.per_day[best] } : null;
  });

  const groupProgress = $derived(
    [
      ...groups.map((g) => ({ name: g.name, color: hex(g.color), list: todos.filter((t) => t.group_id === g.id) })),
      { name: t('todo.ungrouped'), color: 'var(--faint)', list: todos.filter((t) => t.group_id === null) },
    ]
      .filter((g) => g.list.length)
      .map((g) => ({ ...g, done: g.list.filter((t) => t.done).length })),
  );
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.analytics')}</h1>
      <p class="sub">{t('an.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <div class="segmented">
      <button class:active={tab === 'all'} onclick={() => (tab = 'all')}>{t('an.overview')}</button>
      <button class:active={tab === 'todos'} onclick={() => (tab = 'todos')}>{t('an.todos')}</button>
      <button class:active={tab === 'work'} onclick={() => (tab = 'work')}>{t('an.work')}</button>
    </div>
    <div class="segmented">
      {#each [7, 30, 90] as n (n)}
        <button class:active={span === n} onclick={() => (span = n)}>{t('an.days', { n })}</button>
      {/each}
    </div>
  </div>

  {#if tab === 'all'}
    <div class="stats">
      <StatCard label={t('an.workedToday')} value={fmtDuration(summary?.per_day.at(-1) ?? 0)} sub={t('an.inDays', { d: fmtDuration(summary?.total ?? 0), n: span })} icon={Clock} />
      <StatCard label={t('an.focusSessions')} value={String(sessions.length)} sub={t('an.focused', { d: fmtDuration(focusTotal) })} icon={Flame} tint="var(--focus)" />
      <StatCard label={t('an.todosCompleted')} value={String(doneInRange)} sub={t('an.stillOpen', { n: openTotal })} icon={CircleCheck} tint="var(--success)" />
      <StatCard label={t('an.eventsWeek')} value={String(upcoming.length)} sub={t('an.next7')} icon={CalendarDays} tint="var(--long-break)" />
      <StatCard label={t('an.nextDday')} value={nextDday ? ddayLabel(nextDday) : '—'} sub={nextDday?.title ?? ''} icon={Target} tint="var(--warning)" />
    </div>
    <div class="grid-2">
      <div class="card"><h3>{t('an.workPerDay')}</h3><BarChart data={workPerDay} format={fmtHours} /></div>
      <div class="card"><h3>{t('an.focusPerDay')}</h3><BarChart data={focusPerDay} color="var(--focus)" format={fmtDuration} /></div>
      <div class="card"><h3>{t('an.donePerDay')}</h3><BarChart data={donePerDay} color="var(--success)" /></div>
      <div class="card">
        <h3>{t('an.topApps')}</h3>
        <div class="list-wrap">
          {#if summary?.apps.length}
            <BarList items={summary.apps.slice(0, 6).map((a) => ({ label: a.app, value: a.secs, color: colorForName(a.app) }))} format={fmtDuration} />
          {:else}<p class="muted">{t('an.noTracked')}</p>{/if}
        </div>
      </div>
    </div>
  {:else if tab === 'todos'}
    <div class="stats">
      <StatCard label={t('an.totalTodos')} value={String(todos.length)} sub={t('an.subTodos', { n: todos.filter((x) => x.parent_id !== null).length })} icon={ListTodo} />
      <StatCard label={t('an.rate')} value="{rate}%" sub={t('an.doneOpen', { done: doneTotal, open: openTotal })} icon={CircleCheck} tint="var(--success)" />
      <StatCard label={t('an.doneInDays', { n: span })} value={String(doneInRange)} sub={t('an.created', { n: createdPerDay.reduce((s, d) => s + d.value, 0) })} icon={Trophy} tint="var(--warning)" />
      <StatCard label={t('an.overdue')} value={String(overdue)} sub={overdue ? t('an.catchUp') : t('an.onTrack')} icon={TriangleAlert} tint="var(--danger)" />
    </div>
    <div class="grid-2">
      <div class="card"><h3>{t('an.completedPerDay')}</h3><BarChart data={donePerDay} color="var(--success)" /></div>
      <div class="card"><h3>{t('an.createdPerDay')}</h3><BarChart data={createdPerDay} color="var(--long-break)" /></div>
    </div>
    <div class="card spaced">
      <h3>{t('an.byGroup')}</h3>
      <div class="groups">
        {#each groupProgress as g (g.name)}
          <div class="g-row">
            <span class="dot" style:background={g.color}></span>
            <span class="g-name truncate">{g.name}</span>
            <span class="g-bar"><span style:width="{(g.done / g.list.length) * 100}%" style:background={g.color}></span></span>
            <span class="tabular muted">{g.done}/{g.list.length}</span>
          </div>
        {:else}
          <p class="muted">{t('an.addTodos')}</p>
        {/each}
      </div>
    </div>
  {:else}
    <div class="stats">
      <StatCard label={t('an.total')} value={fmtDuration(summary?.total ?? 0)} sub={t('an.lastDays', { n: span })} icon={Clock} />
      <StatCard label={t('an.dailyAvg')} value={fmtDuration((summary?.total ?? 0) / activeDays)} sub={t('an.overActive', { n: activeDays })} icon={CalendarDays} tint="var(--long-break)" />
      <StatCard label={t('an.busiest')} value={busiest ? fmt(busiest.day, { month: 'short', day: 'numeric' }) : '—'} sub={busiest ? fmtDuration(busiest.secs) : ''} icon={Trophy} tint="var(--warning)" />
      <StatCard label={t('an.mostUsed')} value={summary?.apps[0]?.app ?? '—'} sub={summary?.apps[0] ? fmtDuration(summary.apps[0].secs) : ''} icon={Flame} tint="var(--focus)" />
    </div>
    <div class="card spaced"><h3>{t('an.hoursPerDay')}</h3><BarChart data={workPerDay} format={fmtHours} height={220} /></div>
    <div class="grid-2">
      <div class="card">
        <h3>{t('an.applications')}</h3>
        <div class="list-wrap">
          <BarList items={(summary?.apps ?? []).slice(0, 12).map((a) => ({ label: a.app, value: a.secs, color: colorForName(a.app) }))} format={fmtDuration} />
        </div>
      </div>
      <div class="card">
        <h3>{t('an.windows')}</h3>
        <div class="list-wrap">
          {#if summary?.titles.length}
            <BarList items={summary.titles.slice(0, 12).map((t) => ({ label: t.title, sub: t.app, value: t.secs, color: colorForName(t.app) }))} format={fmtDuration} />
          {:else}<p class="muted">{t('an.noTitles')}</p>{/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(170px, 100%), 1fr));
    gap: 14px;
    margin-bottom: 16px;
  }
  .card h3 {
    margin-bottom: 14px;
  }
  .spaced {
    margin: 16px 0;
  }
  .list-wrap {
    margin-top: 4px;
  }
  .groups {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .g-row {
    display: grid;
    grid-template-columns: 10px minmax(100px, 200px) 1fr 60px;
    align-items: center;
    gap: 12px;
  }
  .g-name {
    font-weight: 600;
  }
  .g-bar {
    height: 10px;
    border-radius: 10px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .g-bar span {
    display: block;
    height: 100%;
    border-radius: 10px;
  }
  @container main (max-width: 560px) {
    .g-row {
      grid-template-columns: 10px minmax(0, 1fr) 50px;
    }
    .g-bar {
      display: none;
    }
  }
</style>
