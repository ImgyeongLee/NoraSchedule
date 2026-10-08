<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Link, MapPin, Plus, Trash, X } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import RepeatEditor from '../components/RepeatEditor.svelte';
  import { deleteEventWithUndo } from '../lib/clipboard.svelte';
  import { askScope, isRepeatingOccurrence } from '../lib/prompt.svelte';
  import { api, type CalEvent } from '../lib/api';
  import { addMinutes, dateOf, timeOf, toDateTime } from '../lib/dates';
  import { mutate } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  let { event, onclose }: { event: CalEvent; onclose: () => void } = $props();

  function initialForm() {
    return {
      title: event.title,
      allDay: event.all_day,
      startDate: dateOf(event.start),
      startTime: timeOf(event.start),
      endDate: dateOf(event.end),
      endTime: timeOf(event.end),
      color: event.color,
      location: event.location,
      links: [...event.links],
      memo: event.memo,
      repeat: event.repeat ? { ...event.repeat, weekdays: [...event.repeat.weekdays] } : null,
    };
  }
  let form = $state(initialForm());
  let newLink = $state('');
  let error = $state('');
  const isNew = $derived(event.id === 0);
  const repeating = $derived(isRepeatingOccurrence(event));

  // Keep the end after the start when the start moves (like Google Calendar).
  function startChanged() {
    if (form.endDate < form.startDate) form.endDate = form.startDate;
    if (!form.allDay && toDateTime(form.endDate, form.endTime) <= toDateTime(form.startDate, form.startTime)) {
      const end = addMinutes(toDateTime(form.startDate, form.startTime), 60);
      form.endDate = dateOf(end);
      form.endTime = timeOf(end);
    }
  }

  function addLink() {
    const raw = newLink.trim();
    if (!raw) return;
    form.links.push(/^[a-z]+:/i.test(raw) ? raw : `https://${raw}`);
    newLink = '';
  }

  async function save() {
    addLink();
    if (!form.startDate || !form.endDate || (!form.allDay && (!form.startTime || !form.endTime))) {
      error = t('event.fillDateTime');
      return;
    }
    const start = toDateTime(form.startDate, form.allDay ? '00:00' : form.startTime);
    const end = toDateTime(form.endDate, form.allDay ? '00:00' : form.endTime);
    if (form.allDay ? end < start : end <= start) {
      error = t('event.endAfterStart');
      return;
    }
    // Editing one day of a repeating event: ask whether to change just it or the series.
    const scope = repeating ? await askScope('save') : 'all';
    if (!scope) return;
    const saved = await mutate(
      api.saveEvent(
        {
        id: event.id,
        title: form.title.trim() || t('event.untitled'),
        start,
        end,
        all_day: form.allDay,
        color: form.color,
        location: form.location.trim(),
        links: form.links,
        memo: form.memo,
        repeat: form.repeat && { ...form.repeat, interval: Math.max(1, Math.round(form.repeat.interval || 1)) },
        exdates: event.exdates,
        occurrence: event.occurrence,
        },
        scope,
      ),
      isNew ? t('event.added') : t('event.saved'),
    );
    if (saved !== undefined) onclose();
  }

  async function remove() {
    // Read the prop before closing: once this dialog unmounts, `event` is no longer reliable.
    const target = $state.snapshot(event) as CalEvent;
    onclose();
    await deleteEventWithUndo(target);
  }

  function onkeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') save();
  }
</script>

<Modal title={isNew ? t('event.new') : t('event.edit')} {onclose} width={540}>
  <!-- svelte-ignore a11y_autofocus -->
  <input class="input title" placeholder={t('event.titlePlaceholder')} bind:value={form.title} autofocus {onkeydown} />

  <div class="when">
    <label class="row allday">
      <input type="checkbox" class="switch" bind:checked={form.allDay} />
      <span>{t('common.allDay')}</span>
    </label>
    <div class="times">
      <div class="field">
        <label for="ev-start">{t('event.starts')}</label>
        <div class="row">
          <input id="ev-start" class="input" type="date" bind:value={form.startDate} onchange={startChanged} />
          {#if !form.allDay}<input class="input time" type="time" bind:value={form.startTime} onchange={startChanged} />{/if}
        </div>
      </div>
      <div class="field">
        <label for="ev-end">{t('event.ends')}</label>
        <div class="row">
          <input id="ev-end" class="input" type="date" bind:value={form.endDate} min={form.startDate} />
          {#if !form.allDay}<input class="input time" type="time" bind:value={form.endTime} />{/if}
        </div>
      </div>
    </div>
  </div>

  <div class="field repeat-box">
    <span class="label">{t('rep.label')}</span>
    <RepeatEditor bind:value={form.repeat} startDate={form.startDate} />
  </div>

  <div class="field">
    <span class="label">{t('common.color')}</span>
    <ColorPicker bind:value={form.color} />
  </div>

  <div class="field">
    <label for="ev-loc">{t('common.location')}</label>
    <div class="with-icon">
      <MapPin size={16} />
      <input id="ev-loc" class="input" placeholder={t('event.locationPlaceholder')} bind:value={form.location} />
    </div>
  </div>

  <div class="field">
    <label for="ev-link">{t('common.links')}</label>
    {#each form.links as link, i (i)}
      <div class="link">
        <Link size={14} />
        <button class="link-text truncate" onclick={() => openUrl(link)} title={t('event.openLink', { url: link })}>{link}</button>
        <button class="icon-btn" onclick={() => form.links.splice(i, 1)} aria-label={t('event.removeLink')}><X size={14} /></button>
      </div>
    {/each}
    <div class="row">
      <input
        id="ev-link"
        class="input"
        placeholder={t('event.linkPlaceholder')}
        bind:value={newLink}
        onkeydown={(e) => e.key === 'Enter' && addLink()}
      />
      <button class="btn" onclick={addLink} disabled={!newLink.trim()}><Plus size={16} /> {t('common.add')}</button>
    </div>
  </div>

  <div class="field">
    <label for="ev-memo">{t('common.memo')}</label>
    <textarea id="ev-memo" class="textarea" rows="3" placeholder={t('event.memoPlaceholder')} bind:value={form.memo}></textarea>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#snippet footer()}
    {#if repeating}
      <!-- The "just this one / all" dialog doubles as the confirmation. -->
      <button class="btn danger" onclick={remove}><Trash size={16} /> {t('common.delete')}</button>
    {:else if !isNew}
      <ConfirmButton onconfirm={remove} question={t('event.deleteQ')} />
    {/if}
    <span class="spacer"></span>
    <button class="btn ghost" onclick={onclose}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={save}>{isNew ? t('event.addButton') : t('common.save')}</button>
  {/snippet}
</Modal>

<style>
  .when {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .when .input {
    background: var(--surface);
  }
  .repeat-box {
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .allday {
    font-weight: 600;
    gap: 10px;
    cursor: pointer;
    width: fit-content;
  }
  .times {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 12px;
  }
  .times input[type='date'] {
    flex: 1;
    width: auto;
  }
  .times .row {
    gap: 6px;
  }
  .times .input {
    min-width: 0;
    padding: 0 8px;
  }
  .time {
    width: 96px;
    flex: none;
  }
  .with-icon {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--faint);
  }
  .with-icon :global(svg) {
    position: absolute;
    left: 12px;
  }
  .with-icon .input {
    padding-left: 36px;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 4px 12px;
    border-radius: var(--radius-sm);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .link-text {
    flex: 1;
    border: none;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font-weight: 550;
  }
  .link-text:hover {
    text-decoration: underline;
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
  @media (max-width: 620px) {
    .times {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
