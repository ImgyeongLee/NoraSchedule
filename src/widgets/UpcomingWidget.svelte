<script lang="ts">
  import { CalendarRange } from '@lucide/svelte';
  import UpcomingTable from '../components/UpcomingTable.svelte';
  import { dateOf } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { openCalendar, ui } from '../lib/state.svelte';
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'calendar')}><span class="w-icon"><CalendarRange size={15} /></span>{t('up.title')}</button>
  </div>
  <div class="w-body table-body">
    <UpcomingTable compact onopen={(e) => openCalendar(e.occurrence ?? dateOf(e.start))} />
  </div>
</div>

<style>
  .table-body {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
