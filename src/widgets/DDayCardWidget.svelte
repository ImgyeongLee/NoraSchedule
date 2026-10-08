<script lang="ts">
  // One D-Day shown big, with its cover photo. Several of these can be on the Overview.
  import { CalendarHeart, Plus } from '@lucide/svelte';
  import { api, type DDay } from '../lib/api';
  import { hex } from '../lib/colors';
  import { byDdayTarget, ddayDays, ddayLabel, ddayTarget, ddayUpcoming, fmt } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { imageUrl } from '../lib/images';
  import { data, load, ui } from '../lib/state.svelte';

  let { ddayId = null }: { ddayId?: number | null } = $props();
  let ddays = $state<DDay[]>([]);

  $effect(() => {
    data.version;
    load(api.ddays(), []).then((d) => (ddays = d));
  });

  // The chosen D-Day, or the next upcoming one (also when the chosen one was deleted).
  const dday = $derived(
    ddays.find((d) => d.id === ddayId) ?? byDdayTarget(ddays.filter((d) => ddayUpcoming(d)))[0] ?? ddays[ddays.length - 1],
  );

  function relative(d: DDay) {
    const n = ddayDays(d);
    if (n === 0) return t('dd.today');
    if (n < 0 && d.count_from_one) return t('dd.dayN', { n: -n });
    return n > 0 ? t('dd.inDays', { n }) : t('dd.daysAgo', { n: -n });
  }
</script>

{#if dday}
  <button
    class="card-dday"
    class:photo={!!dday.image}
    style:--c={hex(dday.color)}
    style:background-image={dday.image ? `url('${imageUrl(dday.image)}')` : undefined}
    onclick={() => (ui.page = 'ddays')}
  >
    <span class="label">{ddayLabel(dday)}</span>
    <span class="title truncate">{dday.title}</span>
    <span class="date">{fmt(ddayTarget(dday), { month: 'long', day: 'numeric', weekday: 'short' })} · {relative(dday)}</span>
  </button>
{:else}
  <div class="widget">
    <div class="w-head"><span class="w-icon"><CalendarHeart size={15} /></span><span class="strong">{t('w.dday')}</span></div>
    <div class="w-empty">
      <span>
        {t('w.dday.none')}<br />
        <button class="btn small" onclick={() => (ui.page = 'ddays')}><Plus size={14} /> {t('w.dday.add')}</button>
      </span>
    </div>
  </div>
{/if}

<style>
  .card-dday {
    position: relative;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    gap: 2px;
    width: calc(100% + 36px);
    height: calc(100% + 36px);
    margin: -18px;
    padding: 20px 22px;
    border: none;
    text-align: left;
    cursor: pointer;
    background:
      radial-gradient(circle at 100% 0%, color-mix(in srgb, var(--c) 30%, transparent), transparent 65%),
      radial-gradient(circle at 0% 100%, color-mix(in srgb, var(--c) 12%, transparent), transparent 60%);
    background-size: cover;
    background-position: center;
  }
  .card-dday.photo {
    color: #fff;
  }
  .card-dday.photo::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: linear-gradient(to top, rgba(10, 12, 24, 0.78), rgba(10, 12, 24, 0.12) 70%);
  }
  .label {
    font-size: 40px;
    font-weight: 800;
    letter-spacing: -0.03em;
    line-height: 1.05;
    color: var(--c);
  }
  .photo .label {
    color: #fff;
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.35);
  }
  .title {
    max-width: 100%;
    font-size: 15px;
    font-weight: 700;
  }
  .date {
    font-size: 12.5px;
    opacity: 0.75;
  }
  .strong {
    font-weight: 700;
  }
</style>
