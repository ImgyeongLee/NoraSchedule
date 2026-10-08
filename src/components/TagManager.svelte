<script lang="ts">
  // Create, rename, recolor, categorize and delete event tags.
  import { Plus, Tag as TagIcon } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import ColorPicker from './ColorPicker.svelte';
  import ConfirmButton from './ConfirmButton.svelte';
  import { api, type Tag } from '../lib/api';
  import { DEFAULT_COLOR, hex } from '../lib/colors';
  import { mutate } from '../lib/state.svelte';
  import { categories, groupedTags, refreshTags } from '../lib/tags.svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    onclose,
    oncreated,
    startNew = false,
  }: {
    onclose: () => void;
    /** Called with the id of a newly created tag (e.g. to attach it to the event being edited). */
    oncreated?: (id: number) => void;
    /** Open with the "new tag" form instead of the first tag. */
    startNew?: boolean;
  } = $props();

  const blank = (): Tag => ({ id: 0, name: '', color: DEFAULT_COLOR, category: '' });
  const groups = $derived(groupedTags());
  // Start on the first tag, unless asked for a new one (or there are none yet).
  const initialForm = (): Tag => {
    const first = groupedTags()[0]?.tags[0];
    return !startNew && first ? { ...first } : blank();
  };
  let form = $state<Tag>(initialForm());
  let error = $state('');

  async function save() {
    if (!form.name.trim()) {
      error = t('tag.needName');
      return;
    }
    error = '';
    const id = await mutate(api.saveTag(form), form.id ? t('tag.saved') : t('tag.added'));
    if (id === undefined) return;
    const created = form.id === 0;
    await refreshTags();
    if (created && oncreated) {
      oncreated(id);
      onclose();
      return;
    }
    form = { ...form, id };
  }

  async function remove() {
    if (!form.id) return;
    const name = form.name;
    if ((await mutate(api.deleteTag(form.id), t('tag.deleted', { name }))) === undefined) return;
    await refreshTags();
    form = blank();
  }
</script>

<Modal title={t('tag.manage')} {onclose} width={620}>
  <div class="layout">
    <div class="list">
      <button class="btn small new" class:active={form.id === 0} onclick={() => (form = blank())}><Plus size={15} /> {t('tag.create')}</button>
      {#each groups as group (group.category)}
        <div class="faint small cat">{group.category || t('tag.noCategory')}</div>
        {#each group.tags as tag (tag.id)}
          <button class="row-btn" class:active={form.id === tag.id} onclick={() => { form = { ...tag }; error = ''; }}>
            <span class="dot" style:background={hex(tag.color)}></span>
            <span class="truncate">{tag.name}</span>
          </button>
        {/each}
      {/each}
    </div>

    <div class="editor">
      <div class="preview">
        <span class="chip-preview" style:--tc={hex(form.color)}><TagIcon size={13} /> {form.name || t('tag.namePlaceholder')}</span>
      </div>
      <div class="field">
        <label for="tag-name">{t('tag.name')}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="tag-name"
          class="input"
          placeholder={t('tag.namePlaceholder')}
          bind:value={form.name}
          autofocus
          onkeydown={(e) => e.key === 'Enter' && save()}
        />
      </div>
      <div class="field">
        <label for="tag-cat">{t('tag.category')}</label>
        <input id="tag-cat" class="input" list="tag-categories" placeholder={t('tag.categoryPlaceholder')} bind:value={form.category} />
        <datalist id="tag-categories">
          {#each categories() as c (c)}<option value={c}></option>{/each}
        </datalist>
      </div>
      <div class="field">
        <span class="label">{t('common.color')}</span>
        <ColorPicker bind:value={form.color} />
      </div>
      {#if error}<p class="error">{error}</p>{/if}
    </div>
  </div>

  {#snippet footer()}
    {#if form.id}<ConfirmButton onconfirm={remove} question={t('tag.deleteQ')} />{/if}
    <span class="spacer"></span>
    <button class="btn ghost" onclick={onclose}>{t('common.close')}</button>
    <button class="btn primary" onclick={save}>{form.id ? t('common.save') : t('tag.create')}</button>
  {/snippet}
</Modal>

<style>
  .layout {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
    gap: 18px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 360px;
    overflow-y: auto;
    padding-right: 6px;
    border-right: 1px solid var(--border);
  }
  .new {
    margin-bottom: 6px;
  }
  .new.active {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .cat {
    margin: 8px 0 2px 4px;
  }
  .row-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border: none;
    border-radius: var(--radius-xs);
    background: transparent;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .row-btn:hover {
    background: var(--surface-2);
  }
  .row-btn.active {
    background: var(--primary-soft);
    font-weight: 650;
  }
  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .chip-preview {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--tc);
    color: #fff;
    font-weight: 650;
  }
  .error {
    color: var(--danger);
    font-weight: 600;
  }
  @media (max-width: 620px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .list {
      border-right: none;
      max-height: 160px;
    }
  }
</style>
