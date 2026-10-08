<script lang="ts">
  import { Target } from '@lucide/svelte';
  import { api, type DDay } from '../lib/api';
  import { hex } from '../lib/colors';
  import { ddayLabel, fmt, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';
  import { imageUrl } from '../lib/images';

  let ddays = $state<DDay[]>([]);

  $effect(() => {
    data.version;
    load(api.ddays(), []).then((x) => (ddays = x.filter((d) => d.date >= today())));
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
      <div
        class="hero"
        class:photo={!!next.image}
        style:--c={hex(next.color)}
        style:background-image={next.image ? `url('${imageUrl(next.image)}')` : undefined}
      >
        <span class="label">{ddayLabel(next.date)}</span>
        <span class="name truncate">{next.title}</span>
        <span class="faint small">{fmt(next.date, { month: 'long', day: 'numeric', weekday: 'short' })}</span>
      </div>
      {#each rest as d (d.id)}
        <div class="rest">
          <span class="pill" style:color={hex(d.color)} style:background="color-mix(in srgb, {hex(d.color)} 14%, transparent)">{ddayLabel(d.date)}</span>
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
    padding: 12px 14px;
    border-radius: 16px;
    background-size: cover;
    background-position: center;
    color: #fff;
  }
  .hero.photo::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: linear-gradient(to top, rgba(10, 12, 24, 0.75), rgba(10, 12, 24, 0.1));
  }
  .hero.photo .label,
  .hero.photo .faint {
    color: #fff;
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
