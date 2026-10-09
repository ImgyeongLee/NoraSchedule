<script lang="ts">
  import { onMount } from 'svelte';
  import { MoonStar, Sun, Sunrise, Sunset } from '@lucide/svelte';
  import { api } from '../lib/api';
  import { covers, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load } from '../lib/state.svelte';

  let now = $state(new Date());
  let events = $state(0);
  let todos = $state(0);

  onMount(() => {
    const timer = setInterval(() => (now = new Date()), 10_000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    data.version;
    const d = today();
    load(api.eventsBetween(d, d), []).then((e) => (events = e.filter((x) => covers(x, d)).length));
    load(api.todos(), []).then((list) => (todos = list.filter((x) => !x.done && x.due !== null && x.due <= d).length));
  });

  const hour = $derived(now.getHours());
  const greeting = $derived(
    hour < 5 ? t('home.greetNight') : hour < 12 ? t('home.greetMorning') : hour < 18 ? t('home.greetAfternoon') : t('home.greetEvening'),
  );
  const TimeIcon = $derived(hour < 5 ? MoonStar : hour < 12 ? Sunrise : hour < 18 ? Sun : Sunset);
  const clock = $derived(`${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`);
</script>

<div class="widget greeting">
  <div class="text">
    <div class="time-icon"><TimeIcon size={20} /></div>
    <h2>{greeting}</h2>
    <p class="summary">{t('home.summary', { events, todos })}</p>
  </div>
  <div class="clock tabular">{clock}</div>
</div>

<style>
  .greeting {
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin: -18px;
    padding: 22px 26px;
    height: calc(100% + 36px);
    background:
      radial-gradient(circle at 100% 0%, color-mix(in srgb, var(--primary) 22%, transparent), transparent 60%),
      radial-gradient(circle at 0% 100%, color-mix(in srgb, var(--accent-2) 18%, transparent), transparent 55%);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .time-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
  }
  h2 {
    font-size: 22px;
    margin-top: 4px;
  }
  .summary {
    margin-top: 10px;
    display: inline-block;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--surface);
    font-size: 12.5px;
    font-weight: 600;
    box-shadow: var(--shadow-sm);
  }
  .clock {
    font-size: 54px;
    font-weight: 750;
    letter-spacing: -0.04em;
    color: var(--primary);
  }
  @container tile (max-width: 300px) {
    .clock {
      display: none;
    }
  }
</style>
