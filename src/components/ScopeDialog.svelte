<script lang="ts">
  import { CalendarDays, Repeat } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { t } from '../lib/i18n.svelte';
  import { answerScope, scopePrompt } from '../lib/prompt.svelte';
</script>

{#if scopePrompt.open}
  <Modal
    title={scopePrompt.action === 'delete' ? t('scope.deleteTitle') : t('scope.saveTitle')}
    onclose={() => answerScope(null)}
    width={440}
  >
    <p class="muted">{scopePrompt.action === 'delete' ? t('scope.deleteBody') : t('scope.saveBody')}</p>
    <div class="choices">
      <button class="choice" onclick={() => answerScope('one')}>
        <span class="icon"><CalendarDays size={20} /></span>
        {t('scope.one')}
      </button>
      <button class="choice" class:danger={scopePrompt.action === 'delete'} onclick={() => answerScope('all')}>
        <span class="icon"><Repeat size={20} /></span>
        {t('scope.all')}
      </button>
    </div>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => answerScope(null)}>{t('common.cancel')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .choice {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 18px 12px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-weight: 650;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .choice:hover {
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .choice.danger:hover {
    border-color: var(--danger);
    background: var(--danger-soft);
    color: var(--danger);
  }
  .icon {
    width: 44px;
    height: 44px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: var(--primary-soft);
  }
  .choice.danger:hover .icon {
    color: var(--danger);
    background: var(--surface);
  }
</style>
