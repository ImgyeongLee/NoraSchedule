<script lang="ts">
  // A book cover: the uploaded image, or a cloth cover in a color picked from the title.
  // Size it with --cover-w / --cover-h on a parent; `small` drops the printed text.
  import { coverColor } from '../lib/colors';
  import { imageUrl } from '../lib/images';

  let { title, author = '', image = null, small = false }: { title: string; author?: string; image?: string | null; small?: boolean } =
    $props();
</script>

<span class="cover" class:photo={!!image} class:small style:--c={coverColor(title)}>
  {#if image}
    <img src={imageUrl(image, 'book-cover')} alt="" draggable="false" />
  {:else if !small}
    <span class="cover-title">{title}</span>
    {#if author}<span class="cover-writer truncate">{author}</span>{/if}
  {/if}
</span>

<style>
  .cover {
    position: relative;
    flex: none;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    width: var(--cover-w, 124px);
    height: var(--cover-h, 178px);
    padding: 16px 12px 12px 18px;
    border-radius: 3px 8px 8px 3px;
    overflow: hidden;
    color: #fff;
    text-align: left;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 45%),
      linear-gradient(to bottom, color-mix(in srgb, var(--c) 88%, #fff), color-mix(in srgb, var(--c) 82%, #000));
    box-shadow: 0 6px 14px rgba(0, 0, 0, 0.22), 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: transform 0.18s;
  }
  .cover.small {
    padding: 0;
    border-radius: 2px 5px 5px 2px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
  }
  /* The spine: a darker band and a crease on the left edge. */
  .cover::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(to right, rgba(0, 0, 0, 0.28) 0, rgba(0, 0, 0, 0.12) 7%, rgba(255, 255, 255, 0.22) 8%, transparent 11%);
    pointer-events: none;
  }
  .cover.photo {
    padding: 0;
    background: var(--surface-3);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover-title {
    font-size: 14px;
    font-weight: 800;
    line-height: 1.25;
    word-break: keep-all;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
  }
  .cover-writer {
    font-size: 11px;
    font-weight: 600;
    opacity: 0.85;
  }
</style>
