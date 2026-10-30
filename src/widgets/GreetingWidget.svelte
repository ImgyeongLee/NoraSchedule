<script lang="ts">
  import { onMount } from 'svelte';
  import { MoonStar, Pencil, RotateCcw, Sun, Sunrise, Sunset } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import { api } from '../lib/api';
  import { DAY_PARTS, greetingFor, greetings, loadGreetings, saveGreetings, type DayPart } from '../lib/greetings.svelte';
  import { covers, today } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load } from '../lib/state.svelte';

  let now = $state(new Date());
  let events = $state(0);
  let todos = $state(0);

  onMount(() => {
    loadGreetings();
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
  // greetingFor reads the user's lines, so this updates as soon as they are saved.
  const greeting = $derived(greetingFor(now));

  // ---- editing the lines for each part of the day
  let editing = $state<Record<DayPart, string> | null>(null);
  const openEditor = () => (editing = { ...greetings });
  async function save() {
    if (!editing) return;
    await saveGreetings(editing);
    editing = null;
  }
  const TimeIcon = $derived(hour < 5 ? MoonStar : hour < 12 ? Sunrise : hour < 18 ? Sun : Sunset);
  const clock = $derived(`${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`);
</script>

<div class="widget greeting">
  <button class="icon-btn edit" onclick={openEditor} title={t('greet.edit')} aria-label={t('greet.edit')}><Pencil size={14} /></button>
  <div class="text">
    <div class="time-icon"><TimeIcon size={20} /></div>
    <h2>{greeting}</h2>
    <p class="summary">{t('home.summary', { events, todos })}</p>
  </div>
  <div class="clock tabular">{clock}</div>
</div>

{#if editing}
  <Modal title={t('greet.edit')} onclose={() => (editing = null)} width={520}>
    <p class="muted small">{t('greet.editBody')}</p>
    {#each DAY_PARTS as p (p.id)}
      <div class="field">
        <label for="greet-{p.id}">{t(p.label)} <span class="faint small">{t('greet.hours', { from: p.from, to: p.to })}</span></label>
        <textarea id="greet-{p.id}" class="textarea" rows="2" placeholder={t(p.fallback)} bind:value={editing[p.id]}></textarea>
      </div>
    {/each}
    {#snippet footer()}
      <button class="btn ghost" onclick={() => editing && (editing = { night: '', morning: '', afternoon: '', evening: '' })}><RotateCcw size={15} /> {t('greet.reset')}</button>
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (editing = null)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={save}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .greeting {
    position: relative;
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
  .edit {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 28px;
    height: 28px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .greeting:hover .edit,
  .edit:focus-visible {
    opacity: 1;
  }
</style>
