<script lang="ts">
  import { Trophy } from '@lucide/svelte';
  import { api, type Todo } from '../lib/api';
  import { today, tsToDate } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';

  let todos = $state<Todo[]>([]);

  $effect(() => {
    data.version;
    load(api.todos(), []).then((x) => (todos = x));
  });

  const done = $derived(todos.filter((x) => x.done).length);
  const total = $derived(todos.length);
  const pct = $derived(total ? Math.round((done / total) * 100) : 0);
  const doneToday = $derived(todos.filter((x) => x.completed_at && tsToDate(x.completed_at) === today()).length);
  const R = 40;
  const C = 2 * Math.PI * R;
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'analytics')}><span class="w-icon"><Trophy size={15} /></span>{t('w.progress')}</button>
  </div>
  <div class="body">
    <div class="ring">
      <svg viewBox="0 0 96 96" width="96" height="96">
        <circle cx="48" cy="48" r={R} class="track" />
        <circle cx="48" cy="48" r={R} class="progress" stroke-dasharray={C} stroke-dashoffset={C * (1 - pct / 100)} transform="rotate(-90 48 48)" />
      </svg>
      <span class="pct">{pct}%</span>
    </div>
    <div class="info">
      <span class="strong">{t('w.progress.of', { done, total })}</span>
      <span class="muted small">{t('w.progress.today', { n: doneToday })}</span>
    </div>
  </div>
</div>

<style>
  .body {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .ring {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
  }
  .ring svg {
    display: block;
  }
  .pct {
    position: absolute;
    font-size: 20px;
    font-weight: 800;
  }
  .track {
    fill: none;
    stroke: var(--surface-2);
    stroke-width: 9;
  }
  .progress {
    fill: none;
    stroke: var(--success);
    stroke-width: 9;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.5s ease;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .strong {
    font-weight: 700;
  }
</style>
