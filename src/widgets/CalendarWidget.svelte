<script lang="ts">
  import { CalendarDays } from '@lucide/svelte';
  import MiniCalendar from '../components/MiniCalendar.svelte';
  import { api, type CalEvent } from '../lib/api';
  import { addDays, diffDays, eventSpan, monthStart, range, today, weekStart } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, openCalendar } from '../lib/state.svelte';

  let events = $state<CalEvent[]>([]);

  $effect(() => {
    data.version;
    const start = weekStart(monthStart(today()));
    load(api.eventsBetween(start, addDays(start, 41)), []).then((e) => (events = e));
  });

  const marks = $derived(
    new Set(events.flatMap((e) => { const [a, b] = eventSpan(e); return range(a, diffDays(b, a) + 1); })),
  );
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => openCalendar(today())}><span class="w-icon"><CalendarDays size={15} /></span>{t('w.calendar')}</button>
  </div>
  <div class="w-body">
    <MiniCalendar selected={today()} onpick={openCalendar} {marks} />
  </div>
</div>
