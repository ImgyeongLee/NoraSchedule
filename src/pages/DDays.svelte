<script lang="ts">
  import { Pencil, Plus, Repeat, Target } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import ImagePicker from '../components/ImagePicker.svelte';
  import DateField from '../components/DateField.svelte';
  import { imageUrl } from '../lib/images';
  import { api, type DDay, type DDayShape } from '../lib/api';
  import { PALETTE, hex } from '../lib/colors';
  import { byDdayTarget, ddayDays, ddayLabel, ddayTarget, ddayUpcoming, fmt, today } from '../lib/dates';
  import { data, load, mutate } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  const SHAPES: { id: DDayShape; label: () => string }[] = [
    { id: 'normal', label: () => t('dd.shapeNormal') },
    { id: 'wide', label: () => t('dd.shapeWide') },
    { id: 'tall', label: () => t('dd.shapeTall') },
  ];

  let ddays = $state<DDay[]>([]);
  let editing = $state<DDay | null>(null);
  let error = $state('');

  $effect(() => {
    data.version;
    load(api.ddays(), []).then((d) => (ddays = d));
  });

  const todayStr = $derived(today());
  const upcoming = $derived(byDdayTarget(ddays.filter((d) => ddayUpcoming(d, todayStr)), todayStr));
  const past = $derived(ddays.filter((d) => !ddayUpcoming(d, todayStr)).reverse());

  /** "Tomorrow", "3 days ago"…; empty for "Day N", which the D+N label already says. */
  function relative(d: DDay) {
    const n = ddayDays(d, todayStr);
    if (n === 0) return t('dd.today');
    if (n < 0 && d.count_from_one) return '';
    if (n === 1) return t('dd.tomorrow');
    if (n === -1) return t('dd.yesterday');
    return n > 0 ? t('dd.inDays', { n }) : t('dd.daysAgo', { n: -n });
  }

  function create() {
    error = '';
    editing = { id: 0, title: '', date: todayStr, color: PALETTE[(ddays.length + 4) % PALETTE.length].value, image: null, yearly: false, count_from_one: false, shape: 'normal' };
  }

  /** Closing without saving: drop any photo that was uploaded but not kept. */
  function cancel() {
    editing = null;
    api.removeUnusedImages().catch(() => {});
  }

  /** For yearly D-Days: which anniversary the next one is (1st, 2nd…); 0 before the first. */
  const yearsOf = (d: DDay) => (d.yearly ? Number(ddayTarget(d, todayStr).slice(0, 4)) - Number(d.date.slice(0, 4)) : 0);

  async function save() {
    if (!editing) return;
    if (!editing.title.trim()) return (error = t('dd.needName'));
    if (!editing.date) return (error = t('dd.needDate'));
    const ok = await mutate(api.saveDday({ ...editing, title: editing.title.trim() }), editing.id ? t('dd.saved') : t('dd.added'));
    if (ok !== undefined) editing = null;
  }

  async function remove() {
    if (!editing) return;
    await mutate(api.deleteDday(editing.id), t('dd.deleted'));
    editing = null;
  }
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.ddays')}</h1>
      <p class="sub">{t('dd.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <button class="btn primary" onclick={create}><Plus size={17} /> {t('dd.new')}</button>
  </div>

  {#if ddays.length === 0}
    <div class="card empty">
      <span class="empty-icon"><Target size={30} /></span>
      <h2>{t('dd.emptyTitle')}</h2>
      <p>{t('dd.emptyBody')}</p>
      <button class="btn primary" onclick={create}><Plus size={17} /> {t('dd.addFirst')}</button>
    </div>
  {/if}

  {@render section(t('dd.comingUp'), upcoming)}
  {@render section(t('dd.past'), past)}
</div>

{#snippet section(heading: string, list: DDay[])}
  {#if list.length}
    <h3 class="section">{heading}</h3>
    <div class="cards">
      {#each list as d (d.id)}
        {@const rel = [relative(d), yearsOf(d) ? t('dd.nthYear', { n: yearsOf(d) }) : ''].filter(Boolean).join(' · ')}
        <button
          class="dd-card {d.shape}"
          class:photo={!!d.image}
          style:--c={hex(d.color)}
          onclick={() => { error = ''; editing = { ...d }; }}
        >
          <span class="edit"><Pencil size={14} /></span>
          {#if d.image}
            <!-- Small text at the bottom so the photo stays the main thing. -->
            <img class="bg" src={imageUrl(d.image, 'dday')} alt="" draggable="false" />
            <span class="name truncate">{d.title}</span>
            {@render date(d)}
            <span class="countdown">{ddayLabel(d, todayStr)}</span>
          {:else}
            <span class="countdown">{ddayLabel(d, todayStr)}</span>
            <span class="name truncate">{d.title}</span>
            {@render date(d)}
            {#if rel}<span class="rel">{rel}</span>{/if}
          {/if}
        </button>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet date(d: DDay)}
  <span class="date">
    {#if d.yearly}<Repeat size={12} />{/if}
    {fmt(ddayTarget(d, todayStr), { weekday: 'short', month: 'long', day: 'numeric', year: 'numeric' })}
  </span>
{/snippet}

{#if editing}
  <Modal title={editing.id ? t('dd.edit') : t('dd.new')} onclose={cancel} width={460}>
    <div class="field">
      <label for="dd-title">{t('common.name')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="dd-title" class="input title" placeholder={t('dd.namePlaceholder')} bind:value={editing.title} autofocus
        onkeydown={(e) => e.key === 'Enter' && save()} />
    </div>
    <div class="field">
      <label for="dd-date">{t('common.date')}</label>
      <DateField id="dd-date" value={editing.date} onchange={(v) => editing && v && (editing.date = v)} />
      {#if editing.date}<span class="preview" style:color={hex(editing.color)}>{[ddayLabel(editing), relative(editing)].filter(Boolean).join(' · ')}</span>{/if}
    </div>
    <div class="field options">
      <label class="row opt">
        <input type="checkbox" class="switch" bind:checked={editing.yearly} />
        <span><span class="opt-title">{t('dd.yearly')}</span><span class="muted small">{t('dd.yearlyBody')}</span></span>
      </label>
      <label class="row opt">
        <input type="checkbox" class="switch" bind:checked={editing.count_from_one} />
        <span><span class="opt-title">{t('dd.countFromOne')}</span><span class="muted small">{t('dd.countFromOneBody')}</span></span>
      </label>
    </div>
    <div class="field">
      <span class="label">{t('common.color')}</span>
      <ColorPicker bind:value={editing.color} />
    </div>
    <div class="field">
      <span class="label">{t('dd.cover')}</span>
      <ImagePicker bind:value={editing.image} />
    </div>
    <div class="field">
      <span class="label">{t('dd.shape')}</span>
      <div class="segmented shapes">
        {#each SHAPES as s (s.id)}
          <button type="button" class:active={editing.shape === s.id} onclick={() => editing && (editing.shape = s.id)}>
            <span class="shape-icon {s.id}"></span>{s.label()}
          </button>
        {/each}
      </div>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    {#snippet footer()}
      {#if editing?.id}<ConfirmButton onconfirm={remove} question={t('dd.deleteQ')} />{/if}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={cancel}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={save}>{editing?.id ? t('common.save') : t('dd.addButton')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .options {
    gap: 10px;
  }
  .opt {
    gap: 12px;
    cursor: pointer;
    align-items: center;
  }
  .opt > span {
    display: flex;
    flex-direction: column;
  }
  .opt-title {
    font-weight: 650;
  }
  .section {
    margin: 8px 0 12px;
    color: var(--muted);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(230px, 100%), 1fr));
    grid-auto-rows: 240px;
    grid-auto-flow: row dense;
    gap: 16px;
    margin-bottom: 28px;
  }
  /* Landscape cards take two columns, portrait cards two rows. */
  .dd-card.wide {
    grid-column: span 2;
  }
  .dd-card.tall {
    grid-row: span 2;
  }
  @container main (max-width: 560px) {
    .dd-card.wide {
      grid-column: span 1;
    }
  }
  .shapes {
    align-self: flex-start;
  }
  .shapes button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .shape-icon {
    border: 1.5px solid currentColor;
    border-radius: 3px;
  }
  .shape-icon.normal { width: 11px; height: 11px; }
  .shape-icon.wide { width: 16px; height: 10px; }
  .shape-icon.tall { width: 10px; height: 16px; }
  .dd-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 22px;
    border: 1px solid var(--border);
    border-radius: 24px;
    background:
      radial-gradient(circle at 100% 0%, color-mix(in srgb, var(--c) 22%, transparent), transparent 60%),
      var(--surface);
    box-shadow: var(--shadow-sm);
    text-align: left;
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.15s;
    min-width: 0;
  }
  .dd-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
  }
  .edit {
    position: absolute;
    top: 14px;
    right: 14px;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--muted);
    background: var(--surface-2);
    opacity: 0;
    transition: opacity 0.15s;
  }
  .dd-card:hover .edit {
    opacity: 1;
  }
  .countdown {
    font-size: 36px;
    font-weight: 800;
    letter-spacing: -0.03em;
    color: var(--c);
    line-height: 1.1;
  }
  .name {
    font-size: 16px;
    font-weight: 650;
    max-width: 100%;
    margin-top: 6px;
  }
  .date {
    color: var(--muted);
    font-size: 13px;
  }
  .rel {
    margin-top: 8px;
    padding: 3px 10px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 650;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .preview {
    font-weight: 700;
    font-size: 13px;
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
  .date {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  /* Photo cards: the image fills the card; small text and a short shade at the bottom keep it visible. */
  .dd-card.photo {
    justify-content: flex-end;
    gap: 0;
    padding: 16px 18px;
    border: none;
    overflow: hidden;
    color: #fff;
    isolation: isolate;
  }
  .bg {
    position: absolute;
    inset: 0;
    z-index: -2;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .dd-card.photo::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: linear-gradient(to top, rgba(10, 12, 24, 0.6), rgba(10, 12, 24, 0) 42%);
  }
  .photo .name {
    margin-top: 0;
    font-size: 13.5px;
    font-weight: 700;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .photo .date {
    font-size: 11.5px;
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.4);
  }
  .photo .countdown {
    margin-top: 2px;
    font-size: 24px;
    color: #fff;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.4);
  }
  .photo .edit {
    z-index: 1;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
  }
</style>
