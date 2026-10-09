<script lang="ts">
  import { Target } from '@lucide/svelte';
  import { api, type DDay } from '../lib/api';
  import { hex } from '../lib/colors';
  import { byDdayTarget, ddayLabel, ddayTarget, ddayUpcoming, fmt } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';
  import { imageUrl } from '../lib/images';

  let ddays = $state<DDay[]>([]);

  $effect(() => {
    data.version;
    load(api.ddays(), []).then((x) => (ddays = byDdayTarget(x.filter((d) => ddayUpcoming(d)))));
  });

  const next = $derived(ddays[0]);
  const rest = $derived(ddays.slice(1, 6));
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'ddays')}><span class="w-icon"><Target size={15} /></span>{t('w.ddays')}</button>
  </div>
  {#if next}
    <div class="w-body">
      <div class="hero" class:photo={!!next.image} style:--c={hex(next.color)}>
        {#if next.image}
          <!-- Small text at the bottom so the photo stays the main thing. -->
          <img class="bg" src={imageUrl(next.image, 'ddays-widget')} alt="" draggable="false" />
          <span class="name truncate">{next.title}</span>
          <span class="date">{fmt(ddayTarget(next), { month: 'long', day: 'numeric', weekday: 'short' })}</span>
          <span class="label">{ddayLabel(next)}</span>
        {:else}
          <span class="label">{ddayLabel(next)}</span>
          <span class="name truncate">{next.title}</span>
          <span class="faint small">{fmt(ddayTarget(next), { month: 'long', day: 'numeric', weekday: 'short' })}</span>
        {/if}
      </div>
      {#each rest as d (d.id)}
        <div class="rest">
          <span class="pill" style:color={hex(d.color)} style:background="color-mix(in srgb, {hex(d.color)} 14%, transparent)">{ddayLabel(d)}</span>
          <span class="truncate">{d.title}</span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="w-empty">{t('w.ddays.empty')}</div>
  {/if}
</div>

<style>
  .hero {
    display: flex;
    flex-direction: column;
    margin-bottom: 10px;
  }
  .hero.photo {
    position: relative;
    isolation: isolate;
    justify-content: flex-end;
    min-height: 104px;
    padding: 10px 12px;
    border-radius: 16px;
    overflow: hidden;
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
  .hero.photo::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: linear-gradient(to top, rgba(10, 12, 24, 0.6), rgba(10, 12, 24, 0) 55%);
  }
  .photo .name {
    font-size: 12.5px;
    font-weight: 700;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .photo .date {
    font-size: 11px;
    opacity: 0.85;
  }
  .photo .label {
    font-size: 20px;
    color: #fff;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.4);
  }
  .label {
    font-size: 34px;
    font-weight: 800;
    letter-spacing: -0.03em;
    color: var(--c);
    line-height: 1.1;
  }
  .name {
    font-weight: 650;
  }
  .rest {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-size: 13px;
  }
  .pill {
    flex: none;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11.5px;
    font-weight: 750;
  }
</style>
