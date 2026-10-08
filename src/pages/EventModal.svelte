<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Ban, Bell, Link, MapPin, Plus, RotateCcw, Trash, X } from '@lucide/svelte';
  import TagManager from '../components/TagManager.svelte';
  import { hex } from '../lib/colors';
  import { MAX_REMINDER_MINUTES, REMINDER_PRESETS, reminderLabel } from '../lib/reminders';
  import { tagStore } from '../lib/tags.svelte';
  import Modal from '../components/Modal.svelte';
  import ColorPicker from '../components/ColorPicker.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import RepeatEditor from '../components/RepeatEditor.svelte';
  import { deleteEventWithUndo, setEventCancelled } from '../lib/clipboard.svelte';
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
      tags: [...event.tags],
      reminder: event.reminder,
    };
  }
  let form = $state(initialForm());
  let newLink = $state('');
  let error = $state('');
  const isNew = $derived(event.id === 0);
  const repeating = $derived(isRepeatingOccurrence(event));
  let creatingTag = $state(false);

  function toggleTag(id: number) {
    form.tags = form.tags.includes(id) ? form.tags.filter((x) => x !== id) : [...form.tags, id];
  }

  // Reminder: one of the presets, or 'custom' with its own number of minutes.
  const isCustomReminder = () => !REMINDER_PRESETS.includes(event.reminder);
  let customReminder = $state(isCustomReminder());
  const reminderChoice = $derived(customReminder ? 'custom' : String(form.reminder));
  function pickReminder(value: string) {
    customReminder = value === 'custom';
    if (value === 'custom') form.reminder = form.reminder ?? 15;
    else form.reminder = value === 'null' ? null : Number(value);
  }

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
        cancelled: event.cancelled,
        tags: form.tags,
        reminder: form.reminder === null ? null : Math.min(MAX_REMINDER_MINUTES, Math.max(1, Math.round(form.reminder))),
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

  async function toggleCancelled() {
    const target = $state.snapshot(event) as CalEvent;
    onclose();
    await setEventCancelled(target, !target.cancelled);
  }

  function onkeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') save();
  }
</script>

<Modal title={isNew ? t('event.new') : t('event.edit')} {onclose} width={540}>
  {#if event.cancelled}
    <p class="cancelled-note"><Ban size={15} /> {t('event.cancelledNote')}</p>
  {/if}
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
    <label for="ev-remind">{t('remind.label')}</label>
    <div class="row remind">
      <Bell size={16} />
      <select id="ev-remind" class="select" value={reminderChoice} onchange={(e) => pickReminder(e.currentTarget.value)}>
        {#each REMINDER_PRESETS as m (m)}<option value={String(m)}>{reminderLabel(m)}</option>{/each}
        <option value="custom">{t('remind.custom')}</option>
      </select>
      {#if customReminder}
        <input class="input minutes" type="number" min="1" max={MAX_REMINDER_MINUTES} bind:value={form.reminder} />
        <span class="muted">{t('remind.minutesBefore')}</span>
      {/if}
    </div>
  </div>

  <div class="field">
    <span class="label">{t('tag.title')}</span>
    <div class="tag-pick">
      {#each tagStore.list as tag (tag.id)}
        <button
          type="button"
          class="tag-chip"
          class:on={form.tags.includes(tag.id)}
          style:--tc={hex(tag.color)}
          onclick={() => toggleTag(tag.id)}
          title={tag.category || undefined}
        ><span class="tag-dot"></span>{tag.name}</button>
      {/each}
      <button type="button" class="btn small ghost" onclick={() => (creatingTag = true)}><Plus size={14} /> {t('tag.create')}</button>
    </div>
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
    {#if !isNew}
      <button class="btn" onclick={toggleCancelled}>
        {#if event.cancelled}<RotateCcw size={16} /> {t('event.uncancel')}{:else}<Ban size={16} /> {t('event.cancelEvent')}{/if}
      </button>
    {/if}
    <span class="spacer"></span>
    <button class="btn ghost" onclick={onclose}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={save}>{isNew ? t('event.addButton') : t('common.save')}</button>
  {/snippet}
</Modal>

{#if creatingTag}
  <TagManager
    startNew
    onclose={() => (creatingTag = false)}
    oncreated={(id) => {
      form.tags = [...form.tags, id];
      const tag = tagStore.list.find((x) => x.id === id);
      // A new event takes the color of its first tag.
      if (tag && isNew && form.tags.length === 1) form.color = tag.color;
    }}
  />
{/if}

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
  .remind {
    gap: 8px;
    color: var(--faint);
  }
  .remind .select {
    width: auto;
    flex: none;
  }
  .minutes {
    width: 90px;
    flex: none;
  }
  .tag-pick {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px;
    border: 1.5px solid color-mix(in srgb, var(--tc) 40%, transparent);
    border-radius: 999px;
    background: transparent;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .tag-chip.on {
    background: var(--tc);
    border-color: var(--tc);
    color: #fff;
  }
  .tag-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--tc);
  }
  .tag-chip.on .tag-dot {
    background: #fff;
  }
  .cancelled-note {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    color: var(--muted);
    font-weight: 600;
  }
  @media (max-width: 620px) {
    .times {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
