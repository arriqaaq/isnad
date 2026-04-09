<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let { editorEl }: { editorEl?: HTMLDivElement } = $props();

  // Active formatting states
  let isBold = $state(false);
  let isItalic = $state(false);
  let isUnderline = $state(false);
  let isStrikethrough = $state(false);
  let blockType = $state('Normal');
  let showBlockMenu = $state(false);
  let showInsertMenu = $state(false);

  const BLOCK_TYPES = [
    { label: 'Normal', tag: 'p' },
    { label: 'Heading 2', tag: 'h2' },
    { label: 'Heading 3', tag: 'h3' },
    { label: 'Blockquote', tag: 'blockquote' },
  ];

  const INSERT_ITEMS = [
    { label: 'Horizontal Rule', icon: '―', action: () => execFormat('insertHorizontalRule') },
    { label: 'Bulleted List', icon: '•', action: () => execFormat('insertUnorderedList') },
    { label: 'Numbered List', icon: '1.', action: () => execFormat('insertOrderedList') },
  ];

  function execFormat(command: string, value?: string) {
    editorEl?.focus();
    document.execCommand(command, false, value);
    updateState();
  }

  function setBlockType(tag: string) {
    editorEl?.focus();
    if (tag === 'p') {
      document.execCommand('formatBlock', false, 'p');
    } else {
      document.execCommand('formatBlock', false, tag);
    }
    showBlockMenu = false;
    updateState();
  }

  function updateState() {
    isBold = document.queryCommandState('bold');
    isItalic = document.queryCommandState('italic');
    isUnderline = document.queryCommandState('underline');
    isStrikethrough = document.queryCommandState('strikeThrough');

    const block = document.queryCommandValue('formatBlock');
    if (block === 'h2') blockType = 'Heading 2';
    else if (block === 'h3') blockType = 'Heading 3';
    else if (block === 'blockquote') blockType = 'Blockquote';
    else blockType = 'Normal';
  }

  function handleSelectionChange() {
    if (!editorEl) return;
    const sel = window.getSelection();
    if (sel && editorEl.contains(sel.anchorNode)) {
      updateState();
    }
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.block-menu') && !target.closest('.block-dropdown-btn')) {
      showBlockMenu = false;
    }
    if (!target.closest('.insert-menu') && !target.closest('.insert-dropdown-btn')) {
      showInsertMenu = false;
    }
  }

  onMount(() => {
    document.addEventListener('selectionchange', handleSelectionChange);
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('selectionchange', handleSelectionChange);
    document.removeEventListener('click', handleClickOutside);
  });
</script>

<div class="editor-toolbar-bar">
  <!-- Block type dropdown -->
  <div class="toolbar-group">
    <button
      class="block-dropdown-btn"
      onclick={() => { showBlockMenu = !showBlockMenu; showInsertMenu = false; }}
    >
      <span class="block-label">{blockType}</span>
      <span class="dropdown-arrow">&#9662;</span>
    </button>
    {#if showBlockMenu}
      <div class="block-menu">
        {#each BLOCK_TYPES as bt}
          <button
            class="menu-item"
            class:active={blockType === bt.label}
            onclick={() => setBlockType(bt.tag)}
          >
            {bt.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <span class="divider"></span>

  <!-- Inline formatting -->
  <div class="toolbar-group">
    <button class="fmt-btn" class:active={isBold} onclick={() => execFormat('bold')} title="Bold (Ctrl+B)">
      <strong>B</strong>
    </button>
    <button class="fmt-btn italic-btn" class:active={isItalic} onclick={() => execFormat('italic')} title="Italic (Ctrl+I)">
      <em>I</em>
    </button>
    <button class="fmt-btn" class:active={isUnderline} onclick={() => execFormat('underline')} title="Underline (Ctrl+U)">
      <span style="text-decoration: underline">U</span>
    </button>
    <button class="fmt-btn" class:active={isStrikethrough} onclick={() => execFormat('strikeThrough')} title="Strikethrough">
      <span style="text-decoration: line-through">S</span>
    </button>
  </div>

  <span class="divider"></span>

  <!-- Lists -->
  <div class="toolbar-group">
    <button class="fmt-btn" onclick={() => execFormat('insertUnorderedList')} title="Bullet List">
      <span class="list-icon">•≡</span>
    </button>
    <button class="fmt-btn" onclick={() => execFormat('insertOrderedList')} title="Numbered List">
      <span class="list-icon">1≡</span>
    </button>
  </div>

  <span class="divider"></span>

  <!-- Insert dropdown -->
  <div class="toolbar-group">
    <button
      class="insert-dropdown-btn"
      onclick={() => { showInsertMenu = !showInsertMenu; showBlockMenu = false; }}
    >
      Insert <span class="dropdown-arrow">&#9662;</span>
    </button>
    {#if showInsertMenu}
      <div class="insert-menu">
        {#each INSERT_ITEMS as item}
          <button class="menu-item" onclick={() => { item.action(); showInsertMenu = false; }}>
            <span class="menu-icon">{item.icon}</span>
            {item.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .editor-toolbar-bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 16px;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border-subtle);
    min-height: 40px;
    flex-wrap: wrap;
  }
  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 2px;
    position: relative;
  }
  .divider {
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 8px;
    flex-shrink: 0;
    opacity: 0.5;
  }

  /* Format buttons */
  .fmt-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 30px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.9rem;
    font-family: var(--font-serif);
    transition: all var(--transition);
  }
  .fmt-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .fmt-btn.active {
    background: var(--gold-accent-muted);
    color: var(--gold-accent);
  }
  .italic-btn {
    font-style: italic;
  }
  .list-icon {
    font-size: 0.75rem;
    font-family: var(--font-sans);
    letter-spacing: -1px;
  }

  /* Dropdown buttons */
  .block-dropdown-btn,
  .insert-dropdown-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 12px;
    height: 30px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.8rem;
    font-family: var(--font-serif);
    transition: all var(--transition);
    white-space: nowrap;
  }
  .block-dropdown-btn:hover,
  .insert-dropdown-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .block-label {
    font-weight: 500;
  }
  .dropdown-arrow {
    font-size: 0.6rem;
    color: var(--text-muted);
  }

  /* Dropdown menus */
  .block-menu,
  .insert-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    z-index: 100;
    min-width: 160px;
    overflow: hidden;
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 8px 14px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.8rem;
    font-family: var(--font-serif);
    cursor: pointer;
    transition: background var(--transition);
  }
  .menu-item:hover {
    background: var(--bg-hover);
  }
  .menu-item.active {
    color: var(--gold-accent);
    font-weight: 600;
  }
  .menu-icon {
    width: 18px;
    text-align: center;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
</style>
