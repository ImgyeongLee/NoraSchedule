<script lang="ts">
  import { Pencil, Plus, Repeat, Target } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import ImagePicker from '../components/ImagePicker.svelte';
  import DateField from '../components/DateField.svelte';
  import { imageUrl } from '../lib/images';
  import { api, type DDay } from '../lib/api';
  import { PALETTE, hex } from '../lib/colors';
  import { byDdayTarget, ddayDays, ddayLabel, ddayTarget, ddayUpcoming, fmt, today } from '../lib/dates';
  import { data, load, mutate } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

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

  function relative(d: DDay) {
    const n = ddayDays(d, todayStr);
    if (n === 0) return t('dd.today');
    if (n < 0 && d.count_from_one) return t('dd.dayN', { n: -n });
    if (n === 1) return t('dd.tomorrow');
    if (n === -1) return t('dd.yesterday');
    return n > 0 ? t('dd.inDays', { n }) : t('dd.daysAgo', { n: -n });
  }

  function create() {
    error = '';
    editing = { id: 0, title: '', date: todayStr, color: PALETTE[(ddays.length + 4) % PALETTE.length].value, image: null, yearly: false, count_from_one: false };
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
        <button
          class="dd-card"
          class:photo={!!d.image}
          style:--c={hex(d.color)}
          style:background-image={d.image ? `url('${imageUrl(d.image)}')` : undefined}
          onclick={() => { error = ''; editing = { ...d }; }}
        >
          <span class="edit"><Pencil size={14} /></span>
          <span class="countdown">{ddayLabel(d, todayStr)}</span>
          <span class="name truncate">{d.title}</span>
          <span class="date">
            {#if d.yearly}<Repeat size={12} />{/if}
            {fmt(ddayTarget(d, todayStr), { weekday: 'short', month: 'long', day: 'numeric', year: 'numeric' })}
          </span>
          <span class="rel">{relative(d)}{#if yearsOf(d)} · {t('dd.nthYear', { n: yearsOf(d) })}{/if}</span>
        </button>
      {/each}
    </div>
  {/if}
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
      {#if editing.date}<span class="preview" style:color={hex(editing.color)}>{ddayLabel(editing)} · {relative(editing)}</span>{/if}
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
    gap: 16px;
    margin-bottom: 28px;
  }
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
  /* Photo cards: image fills the card, a soft gradient keeps the text readable. */
  .dd-card.photo {
    min-height: 220px;
    justify-content: flex-end;
    border: none;
    background-size: cover;
    background-position: center;
    color: #fff;
    isolation: isolate;
  }
  .dd-card.photo::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: linear-gradient(to top, rgba(10, 12, 24, 0.78), rgba(10, 12, 24, 0.15) 65%, transparent);
  }
  .dd-card.photo .countdown {
    color: #fff;
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.35);
  }
  .dd-card.photo .date {
    color: rgba(255, 255, 255, 0.85);
  }
  .dd-card.photo .rel {
    color: #fff;
    background: rgba(255, 255, 255, 0.22);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
  }
</style>
