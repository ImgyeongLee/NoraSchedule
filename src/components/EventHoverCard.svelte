<script lang="ts">
  import { Bell, MapPin, Repeat } from '@lucide/svelte';
  import { fade } from 'svelte/transition';
  import type { CalEvent } from '../lib/api';
  import { hex } from '../lib/colors';
  import { dateOf, eventSpan, fmt, fmtRange, timeOf } from '../lib/dates';
  import { hideHoverCard, hoverCard } from '../lib/hovercard.svelte';
  import { t } from '../lib/i18n.svelte';
  import { reminderLabel } from '../lib/reminders';
  import { eventColor, tagsOf } from '../lib/tags.svelte';

  const WIDTH = 290;
  const GAP = 8;
  let height = $state(0);
  let vw = $state(1000);
  let vh = $state(800);

  // Below the item if it fits, otherwise above; kept inside the window.
  const pos = $derived.by(() => {
    const a = hoverCard.anchor;
    const left = Math.min(Math.max(GAP, a.left), vw - WIDTH - GAP);
    const below = a.bottom + GAP;
    const top = below + height <= vh - GAP ? below : Math.max(GAP, a.top - GAP - height);
    return { left, top };
  });

  function when(e: CalEvent): string {
    const [a, b] = eventSpan(e);
    if (e.all_day) return a === b ? t('common.allDay') : fmtRange(a, b, { month: 'short', day: 'numeric' });
    if (a === b) return `${timeOf(e.start)} – ${timeOf(e.end)}`;
    const day = (d: string) => fmt(d, { month: 'short', day: 'numeric' });
    return `${day(a)} ${timeOf(e.start)} – ${day(dateOf(e.end))} ${timeOf(e.end)}`;
  }
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} onwheel={hideHoverCard} onkeydown={hideHoverCard} />

{#if hoverCard.visible}
  <div class="card hover" style:left="{pos.left}px" style:top="{pos.top}px" style:width="{WIDTH}px" bind:offsetHeight={height} transition:fade={{ duration: 100 }}>
    {#if hoverCard.heading}<div class="heading">{hoverCard.heading}</div>{/if}
    {#each hoverCard.events.slice(0, 6) as e (`${e.id}-${e.occurrence}`)}
      {@const tags = tagsOf(e)}
      <div class="ev" style:--c={hex(eventColor(e))}>
        <div class="title-row">
          <span class="swatch"></span>
          <span class="title" class:cancelled={e.cancelled}>{e.title}</span>
          {#if e.repeat}<Repeat size={12} />{/if}
        </div>
        <div class="meta">
          {when(e)}{#if e.cancelled} · <span class="cancel-label">{t('hover.cancelled')}</span>{/if}
        </div>
        {#if e.location}<div class="meta row"><MapPin size={12} /> <span class="truncate">{e.location}</span></div>{/if}
        {#if e.reminder !== null}<div class="meta row"><Bell size={12} /> {reminderLabel(e.reminder)}</div>{/if}
        {#if tags.length}
          <div class="tags">
            {#each tags as tag (tag.id)}<span class="tag" style:--tc={hex(tag.color)}>{tag.name}</span>{/each}
          </div>
        {/if}
        {#if e.memo.trim()}<p class="memo">{e.memo}</p>{/if}
      </div>
    {/each}
    {#if hoverCard.events.length > 6}
      <div class="meta">{t('cal.more', { n: hoverCard.events.length - 6 })}</div>
    {/if}
  </div>
{/if}

<style>
  .hover {
    position: fixed;
    z-index: 900;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    box-shadow: var(--shadow-lg);
    pointer-events: none;
  }
  .heading {
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .ev {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .ev + .ev {
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .swatch {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 3px;
    background: var(--c);
  }
  .title {
    font-weight: 700;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .title.cancelled {
    text-decoration: line-through;
    color: var(--muted);
  }
  .cancel-label {
    color: var(--danger);
    font-weight: 600;
  }
  .meta {
    font-size: 12.5px;
    color: var(--muted);
    padding-left: 18px;
    min-width: 0;
  }
  .meta.row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding-left: 18px;
  }
  .tag {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--tc);
    background: color-mix(in srgb, var(--tc) 14%, transparent);
  }
  .memo {
    margin: 4px 0 0 18px;
    padding: 8px 10px;
    border-radius: var(--radius-xs);
    background: var(--surface-2);
    font-size: 12.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 8;
    line-clamp: 8;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
