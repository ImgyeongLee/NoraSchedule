<script lang="ts">
  // Health page, "how I feel" tab: mood, energy and stress on 1–5 scales, tags for what the day
  // was like (tired, appetite down, happy… or your own words) and a note. Saved as you go.
  import { untrack } from 'svelte';
  import { Check, HeartPulse, Plus, X } from '@lucide/svelte';
  import { api, type Condition } from '../../lib/api';
  import { addDays, fmt, range, today, weekStart } from '../../lib/dates';
  import { t, type Key } from '../../lib/i18n.svelte';
  import { data, load, toast } from '../../lib/state.svelte';
  import {
    MOOD_ICONS, SCALES, TAG_GROUPS, emptyCondition, isPresetTag, scaleColor, scaleLabel, tagLabel, type Scale,
  } from '../../lib/wellness';

  let { day = $bindable() }: { day: string } = $props();

  let records = $state<Condition[]>([]);
  const week = $derived(range(weekStart(day), 7));
  $effect(() => {
    data.version;
    // Eight weeks back as well, for the tags you made up yourself.
    load(api.conditionsBetween(addDays(week[0], -56), week[6]), []).then((r) => (records = r));
  });

  const saved = $derived(records.find((r) => r.date === day) ?? null);
  let form = $state<Condition>(emptyCondition(''));
  let dirty = $state(false);
  let status = $state<'idle' | 'saving' | 'saved'>('idle');
  $effect(() => {
    const next = saved ? { ...saved, tags: [...saved.tags] } : emptyCondition(day);
    untrack(() => {
      if (form.date !== next.date) {
        // Another day: what was typed for the last one is saved first.
        if (dirty) save();
        form = next;
        dirty = false;
        status = 'idle';
      } else if (!dirty) {
        form = next;
      }
    });
  });

  // ---- saving as you go (typing waits a moment)
  let timer: ReturnType<typeof setTimeout> | undefined;
  function edit(patch: Partial<Condition>, delay = 0) {
    form = { ...form, ...patch, date: day };
    dirty = true;
    clearTimeout(timer);
    timer = setTimeout(save, delay);
  }
  async function save() {
    clearTimeout(timer);
    const sent = form;
    status = 'saving';
    try {
      await api.saveCondition(sent);
      if (form === sent) dirty = false;
      status = 'saved';
      data.version++;
    } catch (err) {
      status = 'idle';
      toast(String(err), 'error');
    }
  }
  $effect(() => () => {
    // Leaving the tab mid-typing still saves.
    if (dirty) untrack(save);
  });

  const toggleTag = (tag: string) => edit({ tags: form.tags.includes(tag) ? form.tags.filter((x) => x !== tag) : [...form.tags, tag] });
  let custom = $state('');
  function addCustom() {
    const tag = custom.trim();
    custom = '';
    if (tag && !form.tags.includes(tag)) edit({ tags: [...form.tags, tag] });
  }
  /** Your own tags, most used first. */
  const myTags = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const r of records) for (const tag of r.tags) if (!isPresetTag(tag)) counts.set(tag, (counts.get(tag) ?? 0) + 1);
    for (const tag of form.tags) if (!isPresetTag(tag) && !counts.has(tag)) counts.set(tag, 0);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([tag]) => tag);
  });

  const weekRecords = $derived(week.map((d) => ({ date: d, r: records.find((x) => x.date === d) })));
  const weekTags = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const { r } of weekRecords) for (const tag of r?.tags ?? []) counts.set(tag, (counts.get(tag) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 8);
  });
  const weekAvg = (scale: Scale) => {
    const list = weekRecords.map(({ r }) => r?.[scale] ?? 0).filter((n) => n > 0);
    return list.length ? list.reduce((s, n) => s + n, 0) / list.length : 0;
  };
  const LEVELS = [1, 2, 3, 4, 5];
</script>

<div class="condition">
  <div class="layout">
    <div class="card today">
      <div class="col-head">
        <h3><HeartPulse size={18} /> {day === today() ? t('wl.howToday') : t('wl.howThatDay')}</h3>
        <span class="faint small status">
          {#if status === 'saving'}{t('wl.saving')}{:else if status === 'saved' || (saved && !dirty)}<Check size={13} /> {t('wl.savedState')}{/if}
        </span>
      </div>

      {#each SCALES as scale (scale)}
        <div class="field">
          <span class="label">{t(`wl.${scale}` as Key)}</span>
          <div class="scale" role="group" aria-label={t(`wl.${scale}` as Key)}>
            {#each LEVELS as n (n)}
              {@const Icon = MOOD_ICONS[n - 1]}
              <button class="lv" class:on={form[scale] === n} style:--c={scaleColor(scale, n)} aria-pressed={form[scale] === n}
                onclick={() => edit({ [scale]: form[scale] === n ? 0 : n } as Partial<Condition>)}>
                {#if scale === 'mood'}<Icon size={20} />{:else}<span class="lv-n">{n}</span>{/if}
                <span class="lv-l">{scaleLabel(scale, n)}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}

      {#each TAG_GROUPS as g (g.id)}
        <div class="field">
          <span class="label">{t(`wl.tagGroup.${g.id}` as Key)}</span>
          <div class="tags">
            {#each g.tags as tag (tag)}
              <button class="tag" class:on={form.tags.includes(tag)} aria-pressed={form.tags.includes(tag)} onclick={() => toggleTag(tag)}>{tagLabel(tag)}</button>
            {/each}
          </div>
        </div>
      {/each}

      <div class="field">
        <span class="label">{t('wl.myTags')}</span>
        <div class="tags">
          {#each myTags as tag (tag)}
            <button class="tag mine" class:on={form.tags.includes(tag)} aria-pressed={form.tags.includes(tag)} onclick={() => toggleTag(tag)}>{tag}</button>
          {/each}
          <span class="custom">
            <input class="input" placeholder={t('wl.customTagPlaceholder')} bind:value={custom} maxlength="40"
              onkeydown={(e) => e.key === 'Enter' && !e.isComposing && addCustom()} />
            <button class="icon-btn" onclick={addCustom} disabled={!custom.trim()} aria-label={t('common.add')}><Plus size={16} /></button>
          </span>
        </div>
      </div>

      <div class="field">
        <label for="cd-note">{t('wl.conditionNote')}</label>
        <textarea id="cd-note" class="textarea" rows="4" placeholder={t('wl.conditionNotePlaceholder')} value={form.note}
          oninput={(e) => edit({ note: e.currentTarget.value }, 800)} onblur={() => dirty && save()}></textarea>
      </div>
    </div>

    <div class="card">
      <h3>{t('wl.thisWeek')}</h3>
      <div class="week">
        {#each weekRecords as { date, r } (date)}
          {@const Icon = r?.mood ? MOOD_ICONS[r.mood - 1] : null}
          <button class="wday" class:active={date === day} class:is-today={date === today()} onclick={() => (day = date)}>
            <span class="wd">{fmt(date, { weekday: 'short' })}</span>
            <span class="face" style:--c={scaleColor('mood', r?.mood ?? 0)}>
              {#if Icon}<Icon size={20} />{:else}<span class="dn">{Number(date.slice(8))}</span>{/if}
            </span>
            <span class="faint tabular small">{r?.tags.length ? t('wl.tagCount', { n: r.tags.length }) : '–'}</span>
          </button>
        {/each}
      </div>
      <div class="avgs">
        {#each SCALES as scale (scale)}
          {@const avg = weekAvg(scale)}
          <div class="avg">
            <span class="faint small">{t(`wl.${scale}` as Key)}</span>
            <span class="meter"><span style:width="{(avg / 5) * 100}%" style:background={scaleColor(scale, Math.round(avg))}></span></span>
            <b class="tabular">{avg ? avg.toFixed(1) : '–'}</b>
          </div>
        {/each}
      </div>
      {#if weekTags.length}
        <div class="label often">{t('wl.oftenThisWeek')}</div>
        <div class="tags">
          {#each weekTags as [tag, n] (tag)}
            <span class="tag static">{tagLabel(tag)} <b class="tabular">{n}</b></span>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .today {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .col-head h3,
  .card h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .scale {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 6px;
  }
  .lv {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    min-width: 0;
    padding: 8px 4px;
    border: 1.5px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }
  .lv:hover {
    border-color: var(--c);
  }
  .lv.on {
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, var(--surface));
    color: var(--c);
  }
  .lv-n {
    font-size: 16px;
    font-weight: 800;
    line-height: 20px;
  }
  .lv-l {
    font-size: 11px;
    font-weight: 650;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .tag {
    height: 30px;
    padding: 0 12px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 650;
    cursor: pointer;
  }
  .tag:hover {
    border-color: var(--primary);
  }
  .tag.on {
    border-color: var(--primary);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .tag.mine {
    border-style: dashed;
  }
  .tag.mine.on {
    border-style: solid;
  }
  .tag.static {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: default;
  }
  .tag.static:hover {
    border-color: var(--border);
  }
  .custom {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .custom .input {
    width: 170px;
    height: 30px;
    border-radius: var(--radius-pill);
    font-size: 12.5px;
  }
  .week {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 4px;
    margin-top: 12px;
  }
  .wday {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 8px 2px;
    border: none;
    border-radius: 12px;
    background: none;
    cursor: pointer;
  }
  .wday:hover {
    background: var(--surface-2);
  }
  .wday.active {
    background: var(--primary-soft);
  }
  .wd {
    font-size: 11.5px;
    font-weight: 650;
    color: var(--muted);
  }
  .face {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .dn {
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .wday.is-today .dn {
    color: var(--primary);
  }
  .avgs {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 14px;
  }
  .avg {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) 30px;
    align-items: center;
    gap: 10px;
  }
  .avg b {
    text-align: right;
  }
  .meter {
    height: 8px;
    border-radius: 8px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .meter span {
    display: block;
    height: 100%;
    border-radius: 8px;
  }
  .often {
    padding: 16px 0 8px;
  }
  @container main (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
