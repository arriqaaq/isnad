<script lang="ts">
  import { onMount, onDestroy, mount, unmount } from 'svelte';
  import { deserializeToHtml, serializeEditor, getAtMentionContext, getSlashCommandContext, replaceRangeWithAtom } from '$lib/editor';
  import EmbeddedRef from './EmbeddedRef.svelte';
  import MentionChip from './MentionChip.svelte';
  import MentionDropdown from './MentionDropdown.svelte';
  import SlashCommandPalette from './SlashCommandPalette.svelte';
  import EditorToolbar from './EditorToolbar.svelte';

  let { value = '', onchange, placeholder = 'Write your thoughts...' }: {
    value?: string;
    onchange: (text: string) => void;
    placeholder?: string;
  } = $props();

  let editorEl: HTMLDivElement | undefined = $state();
  let showDropdown = $state(false);
  let dropdownQuery = $state('');
  let dropdownPos = $state({ x: 0, y: 0 });
  let mentionRange: Range | null = null;
  let dropdownRef: { handleKeydown: (e: KeyboardEvent) => void } | undefined = $state();

  // Slash command palette state
  let showSlashPalette = $state(false);
  let slashQuery = $state('');
  let slashPos = $state({ x: 0, y: 0 });
  let slashRange: Range | null = null;
  let slashRef: { handleKeydown: (e: KeyboardEvent) => void } | undefined = $state();

  // Track mounted Svelte components for cleanup
  const atomComponents = new WeakMap<HTMLElement, ReturnType<typeof mount>>();

  // Track last value we serialized ourselves, to avoid re-rendering on own changes
  let lastEmittedValue = value;

  // Observer to clean up removed atoms
  let observer: MutationObserver | null = null;

  onMount(() => {
    if (!editorEl) return;

    // Initial render
    renderValue(value);

    // Watch for removed atoms to clean up Svelte components
    observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        for (const node of mutation.removedNodes) {
          if (node instanceof HTMLElement && node.classList.contains('ref-atom')) {
            const comp = atomComponents.get(node);
            if (comp) {
              try { unmount(comp); } catch { /* ignore */ }
              atomComponents.delete(node);
            }
          }
        }
      }
    });
    observer.observe(editorEl, { childList: true, subtree: true });
  });

  onDestroy(() => {
    observer?.disconnect();
  });

  // Re-render when value changes externally (e.g. switching to edit a different note)
  $effect(() => {
    if (value !== lastEmittedValue && editorEl) {
      renderValue(value);
      lastEmittedValue = value;
    }
  });

  function renderValue(text: string) {
    if (!editorEl) return;
    // Clean up existing atoms
    editorEl.querySelectorAll('.ref-atom').forEach(el => {
      const comp = atomComponents.get(el as HTMLElement);
      if (comp) { try { unmount(comp); } catch {} }
    });
    editorEl.innerHTML = text ? deserializeToHtml(text) : '';
    hydrateAtoms();
  }

  function hydrateAtoms() {
    if (!editorEl) return;
    editorEl.querySelectorAll('.ref-atom').forEach(el => {
      const htmlEl = el as HTMLElement;
      if (atomComponents.has(htmlEl)) return; // already hydrated
      const refType = htmlEl.dataset.refType as 'ayah' | 'hadith' | 'narrator';
      const refId = htmlEl.dataset.refId ?? '';
      const Component = refType === 'narrator' ? MentionChip : EmbeddedRef;
      const comp = mount(Component, { target: htmlEl, props: { refType, refId } });
      atomComponents.set(htmlEl, comp);
    });
  }

  function emitChange() {
    if (!editorEl) return;
    const text = serializeEditor(editorEl);
    lastEmittedValue = text;
    onchange(text);
  }

  function handleInput() {
    // Check for @ mention trigger
    if (!editorEl) return;
    const ctx = getAtMentionContext(editorEl);
    if (ctx) {
      mentionRange = ctx.range;
      dropdownQuery = ctx.query;

      // Position dropdown near cursor
      const rect = ctx.range.getBoundingClientRect();
      dropdownPos = { x: rect.left, y: rect.bottom + 4 };
      showDropdown = true;
    } else {
      showDropdown = false;
      mentionRange = null;
    }

    // Check for / slash command trigger
    if (!showDropdown) {
      const slashCtx = getSlashCommandContext(editorEl);
      if (slashCtx) {
        slashRange = slashCtx.range;
        slashQuery = slashCtx.query;
        const rect = slashCtx.range.getBoundingClientRect();
        slashPos = { x: rect.left, y: rect.bottom + 4 };
        showSlashPalette = true;
      } else {
        showSlashPalette = false;
        slashRange = null;
      }
    }

    // Debounced emit
    emitChange();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (showDropdown) {
      // Forward to dropdown for Enter/Escape/Arrow handling
      if (['Enter', 'Escape', 'ArrowDown', 'ArrowUp'].includes(e.key)) {
        dropdownRef?.handleKeydown(e);
        return;
      }
    }

    if (showSlashPalette) {
      if (['Enter', 'Escape', 'ArrowDown', 'ArrowUp'].includes(e.key)) {
        slashRef?.handleKeydown(e);
        return;
      }
    }

    // Keyboard shortcuts for formatting (Ctrl/Cmd + key)
    if (e.ctrlKey || e.metaKey) {
      if (e.key === 'b') { e.preventDefault(); document.execCommand('bold'); emitChange(); return; }
      if (e.key === 'i') { e.preventDefault(); document.execCommand('italic'); emitChange(); return; }
      if (e.key === 'u') { e.preventDefault(); document.execCommand('underline'); emitChange(); return; }
    }

    // Backspace: check if we're about to delete an atom
    if (e.key === 'Backspace') {
      const sel = window.getSelection();
      if (!sel || !sel.isCollapsed || sel.rangeCount === 0) return;
      const range = sel.getRangeAt(0);
      const node = range.startContainer;

      if (node.nodeType === Node.TEXT_NODE && range.startOffset === 0) {
        const prev = node.previousSibling;
        if (prev instanceof HTMLElement && prev.classList.contains('ref-atom')) {
          e.preventDefault();
          prev.remove();
          emitChange();
          return;
        }
      }
      // Also handle case where cursor is directly inside editor at offset pointing to an atom
      if (node === editorEl && range.startOffset > 0) {
        const prevChild = editorEl.childNodes[range.startOffset - 1];
        if (prevChild instanceof HTMLElement && prevChild.classList.contains('ref-atom')) {
          e.preventDefault();
          prevChild.remove();
          emitChange();
          return;
        }
      }
    }
  }

  function handlePaste(e: ClipboardEvent) {
    e.preventDefault();
    const text = e.clipboardData?.getData('text/plain') ?? '';
    document.execCommand('insertText', false, text);
    emitChange();
  }

  function handleMentionSelect(type: 'ayah' | 'hadith' | 'narrator', refId: string) {
    if (!mentionRange || !editorEl) return;

    // Create the atom element
    const atom = document.createElement('span');
    atom.className = 'ref-atom';
    atom.contentEditable = 'false';
    atom.dataset.refType = type;
    atom.dataset.refId = refId;

    // Mount the Svelte component
    const Component = type === 'narrator' ? MentionChip : EmbeddedRef;
    const comp = mount(Component, { target: atom, props: { refType: type, refId } });
    atomComponents.set(atom, comp);

    // Replace the @query text with the atom
    replaceRangeWithAtom(mentionRange, atom);

    showDropdown = false;
    mentionRange = null;
    editorEl.focus();
    emitChange();
  }

  function handleDropdownClose() {
    showDropdown = false;
    mentionRange = null;
  }

  function handleSlashSelect(command: string) {
    if (!slashRange || !editorEl) return;

    // Delete the /query text
    slashRange.deleteContents();

    // Execute the appropriate command
    switch (command) {
      case 'heading':
        document.execCommand('formatBlock', false, 'h2');
        break;
      case 'subheading':
        document.execCommand('formatBlock', false, 'h3');
        break;
      case 'quote':
        document.execCommand('formatBlock', false, 'blockquote');
        break;
      case 'bullet':
        document.execCommand('insertUnorderedList');
        break;
      case 'numbered':
        document.execCommand('insertOrderedList');
        break;
      case 'divider':
        document.execCommand('insertHorizontalRule');
        break;
      case 'ayah':
      case 'hadith':
      case 'narrator':
        // Insert @ to trigger the mention dropdown
        document.execCommand('insertText', false, '@');
        break;
    }

    showSlashPalette = false;
    slashRange = null;
    editorEl.focus();
    emitChange();
  }

  function handleSlashClose() {
    showSlashPalette = false;
    slashRange = null;
  }

  let isEmpty = $derived(!value || value.trim().length === 0);
</script>

<div class="rich-editor-wrapper">
  <EditorToolbar {editorEl} />
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="rich-editor"
    bind:this={editorEl}
    contenteditable="true"
    role="textbox"
    data-placeholder={placeholder}
    class:is-empty={isEmpty}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onpaste={handlePaste}
    onblur={emitChange}
  ></div>

  {#if showDropdown}
    <MentionDropdown
      bind:this={dropdownRef}
      query={dropdownQuery}
      position={dropdownPos}
      onselect={handleMentionSelect}
      onclose={handleDropdownClose}
    />
  {/if}

  {#if showSlashPalette}
    <SlashCommandPalette
      bind:this={slashRef}
      query={slashQuery}
      position={slashPos}
      onselect={handleSlashSelect}
      onclose={handleSlashClose}
    />
  {/if}
</div>

<style>
  .rich-editor-wrapper {
    position: relative;
  }
  .rich-editor {
    min-height: 180px;
    padding: 16px 20px;
    font-family: var(--font-serif);
    font-size: 1rem;
    line-height: 1.8;
    color: var(--text-primary);
    background: var(--note-editor-bg);
    outline: none;
    white-space: pre-wrap;
    word-break: break-word;
    overflow-wrap: break-word;
    border: none;
    border-radius: 0;
  }
  .rich-editor.is-empty:not(:focus)::before {
    content: attr(data-placeholder);
    font-family: var(--font-serif);
    font-style: italic;
    font-size: 0.95rem;
    color: var(--text-muted);
    pointer-events: none;
    opacity: 0.7;
  }

  /* Rich text content styles within the editor */
  .rich-editor :global(h2) {
    font-family: var(--font-serif);
    font-size: 1.4rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 20px 0 8px;
    line-height: 1.3;
  }
  .rich-editor :global(h3) {
    font-family: var(--font-serif);
    font-size: 1.15rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 16px 0 6px;
    line-height: 1.3;
  }
  .rich-editor :global(blockquote) {
    border-left: 3px solid var(--gold-accent);
    padding: 4px 16px;
    margin: 12px 0;
    color: var(--text-secondary);
    font-style: italic;
    background: var(--gold-accent-muted);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }
  .rich-editor :global(ul),
  .rich-editor :global(ol) {
    padding-left: 24px;
    margin: 8px 0;
  }
  .rich-editor :global(li) {
    margin: 4px 0;
  }
  .rich-editor :global(hr) {
    border: none;
    height: 1px;
    background: var(--border);
    margin: 20px 0;
  }
  .rich-editor :global(b),
  .rich-editor :global(strong) {
    font-weight: 700;
  }
  .rich-editor :global(i),
  .rich-editor :global(em) {
    font-style: italic;
  }

  /* Atom (embedded card) styling within editor */
  .rich-editor :global(.ref-atom) {
    display: block;
    margin: 8px 0;
    user-select: all;
    -webkit-user-select: all;
    cursor: default;
  }
  .rich-editor :global(.ref-atom[data-ref-type="narrator"]) {
    display: inline-flex;
    margin: 0 2px;
  }
</style>
