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

  /** Empty for "Day N", which the D+N label already says. */
  function relative(d: DDay) {
    const n = ddayDays(d);
    if (n === 0) return t('dd.today');
    if (n < 0 && d.count_from_one) return '';
    return n > 0 ? t('dd.inDays', { n }) : t('dd.daysAgo', { n: -n });
  }

  const date = (d: DDay) =>
    [fmt(ddayTarget(d), { month: 'long', day: 'numeric', weekday: 'short' }), relative(d)].filter(Boolean).join(' · ');
</script>

{#if dday?.image}
  <!-- Small text at the bottom so the photo stays the main thing. -->
  <button class="card-dday photo" style:--c={hex(dday.color)} onclick={() => (ui.page = 'ddays')}>
    <img class="bg" src={imageUrl(dday.image, 'dday-card')} alt="" draggable="false" />
    <span class="title truncate">{dday.title}</span>
    <span class="date">{date(dday)}</span>
    <span class="label">{ddayLabel(dday)}</span>
  </button>
{:else if dday}
  <button class="card-dday" style:--c={hex(dday.color)} onclick={() => (ui.page = 'ddays')}>
    <span class="label">{ddayLabel(dday)}</span>
    <span class="title truncate">{dday.title}</span>
    <span class="date">{date(dday)}</span>
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
    gap: 0;
    padding: 14px 18px;
    color: #fff;
  }
  .bg {
    position: absolute;
    inset: 0;
    z-index: -2;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .card-dday.photo::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: linear-gradient(to top, rgba(10, 12, 24, 0.6), rgba(10, 12, 24, 0) 45%);
  }
  .photo .title {
    font-size: 13.5px;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .photo .date {
    font-size: 11.5px;
    opacity: 0.85;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .photo .label {
    margin-top: 2px;
    font-size: 26px;
    color: #fff;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.4);
  }
  .label {
    font-size: 40px;
    font-weight: 800;
    letter-spacing: -0.03em;
    line-height: 1.05;
    color: var(--c);
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
