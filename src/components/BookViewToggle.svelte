<script lang="ts">
  // Switches a book page between the shelf and a plain list.
  import { LibraryBig, List } from '@lucide/svelte';
  import { bookViews, loadBookView, setBookView, type BookViewPage } from '../lib/bookView.svelte';
  import { t } from '../lib/i18n.svelte';

  let { page }: { page: BookViewPage } = $props();
  $effect(() => {
    loadBookView(page);
  });
</script>

<div class="segmented icons" role="group" aria-label={t('books.view')}>
  <button class:active={bookViews[page] === 'shelf'} onclick={() => setBookView(page, 'shelf')} title={t('books.viewShelf')} aria-label={t('books.viewShelf')}>
    <LibraryBig size={16} />
  </button>
  <button class:active={bookViews[page] === 'list'} onclick={() => setBookView(page, 'list')} title={t('books.viewList')} aria-label={t('books.viewList')}>
    <List size={16} />
  </button>
</div>

<style>
  .icons button {
    display: grid;
    place-items: center;
    padding-inline: 10px;
  }
</style>
