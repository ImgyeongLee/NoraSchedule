<script lang="ts">
  // WYSIWYG editor for memos. Content goes in and comes out as Markdown, so memos
  // stay plain text on disk and can still be edited in the Markdown view.
  import { onDestroy, onMount } from 'svelte';
  import { Editor } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import { Markdown } from '@tiptap/markdown';
  import { TaskItem, TaskList } from '@tiptap/extension-list';
  import { Placeholder } from '@tiptap/extensions';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    Bold, Code, Heading1, Heading2, Heading3, Italic, Link, List, ListChecks, ListOrdered, Minus, Pilcrow, Quote, Redo2,
    SquareCode, Strikethrough, Underline, Undo2, Unlink,
  } from '@lucide/svelte';
  import { t, type Key } from '../lib/i18n.svelte';
  import { isMac } from '../lib/clipboard.svelte';

  let { value, onchange }: { value: string; onchange: (markdown: string) => void } = $props();

  let element: HTMLDivElement;
  let editor = $state<Editor | null>(null);
  /** Bumped on every editor transaction so toolbar states re-evaluate. */
  let version = $state(0);
  let linkOpen = $state(false);
  let linkUrl = $state('');

  onMount(() => {
    editor = new Editor({
      element,
      extensions: [
        StarterKit.configure({ link: { openOnClick: false, autolink: true } }),
        TaskList,
        TaskItem.configure({ nested: true }),
        Markdown,
        Placeholder.configure({ placeholder: () => t('memo.richPlaceholder') }),
      ],
      content: value,
      contentType: 'markdown',
      editorProps: {
        attributes: { class: 'prose rich-content' },
        // ⌘/Ctrl-click opens a link in the browser; a plain click just places the cursor.
        handleClick: (_view, _pos, event) => {
          const a = (event.target as HTMLElement).closest('a');
          if (a?.href && (event.metaKey || event.ctrlKey)) {
            openUrl(a.href);
            return true;
          }
          return false;
        },
      },
      onUpdate: ({ editor }) => onchange(editor.getMarkdown()),
      onTransaction: () => version++,
    });
  });

  onDestroy(() => editor?.destroy());

  function active(name: string, attrs?: Record<string, unknown>) {
    version; // re-run when the editor changes
    return editor?.isActive(name, attrs) ?? false;
  }

  const chain = () => editor!.chain().focus();

  function canDo(action: 'undo' | 'redo') {
    void version; // re-run when the editor changes
    return action === 'undo' ? (editor?.can().undo() ?? false) : (editor?.can().redo() ?? false);
  }

  function openLink() {
    linkUrl = editor?.getAttributes('link').href ?? '';
    linkOpen = !linkOpen;
  }

  function applyLink() {
    const url = linkUrl.trim();
    if (!url) chain().extendMarkRange('link').unsetLink().run();
    else chain().extendMarkRange('link').setLink({ href: /^[a-z]+:/i.test(url) ? url : `https://${url}` }).run();
    linkOpen = false;
  }

  type Tool = { icon: typeof Bold; label: Key; run: () => void; on?: () => boolean; disabled?: () => boolean } | 'sep';
  const tools: Tool[] = [
    { icon: Pilcrow, label: 'rt.paragraph', run: () => chain().setParagraph().run(), on: () => active('paragraph') },
    { icon: Heading1, label: 'rt.h1', run: () => chain().toggleHeading({ level: 1 }).run(), on: () => active('heading', { level: 1 }) },
    { icon: Heading2, label: 'rt.h2', run: () => chain().toggleHeading({ level: 2 }).run(), on: () => active('heading', { level: 2 }) },
    { icon: Heading3, label: 'rt.h3', run: () => chain().toggleHeading({ level: 3 }).run(), on: () => active('heading', { level: 3 }) },
    'sep',
    { icon: Bold, label: 'rt.bold', run: () => chain().toggleBold().run(), on: () => active('bold') },
    { icon: Italic, label: 'rt.italic', run: () => chain().toggleItalic().run(), on: () => active('italic') },
    { icon: Underline, label: 'rt.underline', run: () => chain().toggleUnderline().run(), on: () => active('underline') },
    { icon: Strikethrough, label: 'rt.strike', run: () => chain().toggleStrike().run(), on: () => active('strike') },
    { icon: Code, label: 'rt.code', run: () => chain().toggleCode().run(), on: () => active('code') },
    { icon: Link, label: 'rt.link', run: openLink, on: () => active('link') },
    'sep',
    { icon: List, label: 'rt.bullet', run: () => chain().toggleBulletList().run(), on: () => active('bulletList') },
    { icon: ListOrdered, label: 'rt.numbered', run: () => chain().toggleOrderedList().run(), on: () => active('orderedList') },
    { icon: ListChecks, label: 'rt.checklist', run: () => chain().toggleTaskList().run(), on: () => active('taskList') },
    'sep',
    { icon: Quote, label: 'rt.quote', run: () => chain().toggleBlockquote().run(), on: () => active('blockquote') },
    { icon: SquareCode, label: 'rt.codeBlock', run: () => chain().toggleCodeBlock().run(), on: () => active('codeBlock') },
    { icon: Minus, label: 'rt.divider', run: () => chain().setHorizontalRule().run() },
    'sep',
    { icon: Undo2, label: 'rt.undo', run: () => chain().undo().run(), disabled: () => !canDo('undo') },
    { icon: Redo2, label: 'rt.redo', run: () => chain().redo().run(), disabled: () => !canDo('redo') },
  ];
</script>

<div class="rich">
  <div class="toolbar" role="toolbar">
    {#each tools as tool, i (i)}
      {#if tool === 'sep'}
        <span class="sep"></span>
      {:else}
        <button
          type="button"
          class="tool"
          class:on={tool.on?.()}
          disabled={!editor || tool.disabled?.()}
          title={t(tool.label)}
          aria-label={t(tool.label)}
          onmousedown={(e) => e.preventDefault()}
          onclick={tool.run}
        >
          <tool.icon size={17} />
        </button>
      {/if}
    {/each}
  </div>

  {#if linkOpen}
    <div class="link-bar">
      <Link size={15} />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="input"
        placeholder={t('rt.linkPlaceholder')}
        bind:value={linkUrl}
        autofocus
        onkeydown={(e) => {
          if (e.key === 'Enter') applyLink();
          if (e.key === 'Escape') linkOpen = false;
        }}
      />
      <button type="button" class="btn small primary" onclick={applyLink}>{t('rt.apply')}</button>
      {#if active('link')}
        <button type="button" class="btn small ghost" onclick={() => { linkUrl = ''; applyLink(); }} title={t('rt.removeLink')}>
          <Unlink size={15} />
        </button>
      {/if}
    </div>
  {/if}

  <div class="surface" bind:this={element}></div>
  <p class="hint faint small">{t('rt.openLinkHint', { key: isMac ? '⌘' : 'Ctrl' })}</p>
</div>

<style>
  .rich {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .tool {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .tool:hover:not(:disabled) {
    background: var(--surface);
    color: var(--text);
  }
  .tool.on {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .tool:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .sep {
    width: 1px;
    height: 20px;
    margin: 0 5px;
    background: var(--border);
  }
  .link-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    color: var(--primary);
  }
  .link-bar .input {
    height: 34px;
  }
  .surface {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 24px;
    cursor: text;
  }
  .surface :global(.rich-content) {
    min-height: 100%;
  }
  .hint {
    padding: 6px 14px 8px;
    border-top: 1px solid var(--border);
  }
</style>
