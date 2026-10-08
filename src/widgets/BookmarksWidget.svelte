<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Bookmark as BookmarkIcon } from '@lucide/svelte';
  import { api, type Bookmark } from '../lib/api';
  import { hostOf, kindOf } from '../lib/bookmarks';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';

  let links = $state<Bookmark[]>([]);

  $effect(() => {
    data.version;
    load(api.bookmarks(), []).then((b) => (links = b.slice(0, 8)));
  });
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'bookmarks')}><span class="w-icon"><BookmarkIcon size={15} /></span>{t('w.bookmarks')}</button>
  </div>
  <div class="w-body">
    {#each links as b (b.id)}
      {@const kind = kindOf(b.kind)}
      <button class="link" style:--c={kind.color} onclick={() => openUrl(b.url)} title={b.url}>
        <span class="icon"><kind.icon size={15} /></span>
        <span class="text">
          <span class="title truncate">{b.title}</span>
          <span class="host truncate">{hostOf(b.url)}</span>
        </span>
      </button>
    {:else}
      <div class="w-empty">{t('w.bookmarks.empty')}</div>
    {/each}
  </div>
</div>

<style>
  .link {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px;
    border: none;
    border-radius: 10px;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .link:hover {
    background: var(--surface-2);
  }
  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    font-weight: 600;
    font-size: 13px;
  }
  .host {
    font-size: 11.5px;
    color: var(--faint);
  }
</style>
