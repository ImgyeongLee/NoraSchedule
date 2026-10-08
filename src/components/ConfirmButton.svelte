<script lang="ts">
  // A delete button that asks "Are you sure?" inline instead of a scary dialog.
  import { Trash } from '@lucide/svelte';
  import { t } from '../lib/i18n.svelte';

  let { onconfirm, label, question }: { onconfirm: () => void; label?: string; question?: string } = $props();
  let asking = $state(false);
</script>

{#if asking}
  <span class="ask">
    <span class="muted small">{question ?? t('common.delete')}</span>
    <button class="btn small danger" onclick={onconfirm}>{t('common.yesDelete')}</button>
    <button class="btn small ghost" onclick={() => (asking = false)}>{t('common.keep')}</button>
  </span>
{:else}
  <button class="btn danger" onclick={() => (asking = true)}><Trash size={16} /> {label ?? t('common.delete')}</button>
{/if}

<style>
  .ask {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
