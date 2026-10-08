<script lang="ts">
  import { NotebookPen, Plus } from '@lucide/svelte';
  import { api, type Memo } from '../lib/api';
  import { fmtTs } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load, mutate, ui } from '../lib/state.svelte';

  let memos = $state<Memo[]>([]);

  $effect(() => {
    data.version;
    load(api.memos(), []).then((m) => (memos = m.slice(0, 6)));
  });

  function open(id: number) {
    ui.memoFocus = id;
    ui.page = 'memos';
  }

  async function create() {
    const id = await mutate(api.createMemo(t('common.untitled')));
    if (id !== undefined) open(id);
  }
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'memos')}><span class="w-icon"><NotebookPen size={15} /></span>{t('w.memos')}</button>
    <span class="spacer"></span>
    <button class="icon-btn" onclick={create} title={t('w.memos.new')}><Plus size={17} /></button>
  </div>
  <div class="w-body">
    {#each memos as m (m.id)}
      <button class="memo" onclick={() => open(m.id)}>
        <span class="title truncate">{m.title || t('common.untitled')}</span>
        <span class="faint small">{fmtTs(m.updated_at, { month: 'short', day: 'numeric' })}</span>
      </button>
    {:else}
      <div class="w-empty">{t('w.memos.empty')}</div>
    {/each}
  </div>
</div>

<style>
  .memo {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 6px;
    border: none;
    border-radius: 10px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .memo:hover {
    background: var(--surface-2);
  }
  .title {
    flex: 1;
    font-weight: 600;
  }
</style>
