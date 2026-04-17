<script lang="ts">
  import type { BookHeading } from '$lib/types';

  let { headings, currentPageIndex, totalPages, onNavigate, onClose }: {
    headings: BookHeading[];
    currentPageIndex: number;
    totalPages: number;
    onNavigate: (pageIndex: number) => void;
    onClose?: () => void;
  } = $props();

  let pageInput = $state('');
  let expandedSections: Set<number> = $state(new Set());

  // Build tree: level 1 headings are parents, level 2+ are children
  interface HeadingNode {
    heading: BookHeading;
    index: number;
    children: HeadingNode[];
  }

  let tree = $derived.by(() => {
    const nodes: HeadingNode[] = [];
    let currentParent: HeadingNode | null = null;

    for (let i = 0; i < headings.length; i++) {
      const h = headings[i];
      if (h.level === 1) {
        currentParent = { heading: h, index: i, children: [] };
        nodes.push(currentParent);
      } else if (currentParent) {
        currentParent.children.push({ heading: h, index: i, children: [] });
      } else {
        nodes.push({ heading: h, index: i, children: [] });
      }
    }
    return nodes;
  });

  // Find which heading is "current" based on scroll position
  let activeHeadingIndex = $derived.by(() => {
    let best = -1;
    for (let i = 0; i < headings.length; i++) {
      if (headings[i].page_index <= currentPageIndex) {
        best = i;
      } else {
        break;
      }
    }
    return best;
  });

  function toggleSection(index: number) {
    const next = new Set(expandedSections);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    expandedSections = next;
  }

  function handlePageJump() {
    const num = parseInt(pageInput, 10);
    if (num >= 1 && num <= totalPages) {
      onNavigate(num - 1);
    }
    pageInput = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') handlePageJump();
  }
</script>

<aside class="reader-sidebar" dir="rtl">
  {#if onClose}
    <button class="close-btn" onclick={onClose} aria-label="Close">&times;</button>
  {/if}

  <div class="page-navigator">
    <label class="nav-label" for="reader-page-nav">Go to page</label>
    <div class="nav-input-row">
      <input
        id="reader-page-nav"
        type="number"
        class="nav-input"
        placeholder="Page #"
        min="1"
        max={totalPages}
        bind:value={pageInput}
        onkeydown={handleKeydown}
      />
      <button class="nav-go" onclick={handlePageJump}>Go</button>
    </div>
    <span class="nav-total">{totalPages} pages</span>
  </div>

  <div class="heading-tree">
    {#each tree as node}
      <div class="tree-node">
        {#if node.children.length > 0}
          <button
            class="tree-parent"
            class:active={activeHeadingIndex === node.index || node.children.some(c => c.index === activeHeadingIndex)}
            onclick={() => toggleSection(node.index)}
          >
            <span class="expand-icon" class:expanded={expandedSections.has(node.index)}>&#9656;</span>
            <span class="heading-title">{node.heading.title}</span>
          </button>
          {#if expandedSections.has(node.index)}
            <div class="tree-children">
              {#each node.children as child}
                <button
                  class="tree-child"
                  class:active={child.index === activeHeadingIndex}
                  onclick={() => { onNavigate(child.heading.page_index); if (onClose) onClose(); }}
                >
                  {child.heading.title}
                </button>
              {/each}
            </div>
          {/if}
        {:else}
          <button
            class="tree-parent leaf"
            class:active={node.index === activeHeadingIndex}
            onclick={() => { onNavigate(node.heading.page_index); if (onClose) onClose(); }}
          >
            <span class="heading-title">{node.heading.title}</span>
          </button>
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .reader-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    padding: 16px 0;
    position: relative;
  }
  .close-btn {
    display: none;
    position: absolute;
    top: 8px;
    left: 8px;
    width: 32px;
    height: 32px;
    border: none;
    background: var(--bg-hover);
    border-radius: var(--radius-sm);
    font-size: 1.2rem;
    color: var(--text-muted);
    cursor: pointer;
    align-items: center;
    justify-content: center;
  }
  .close-btn:hover { background: var(--bg-active); }

  .page-navigator {
    padding: 0 16px 16px;
    border-bottom: 1px solid var(--border-subtle);
    direction: ltr;
    text-align: left;
  }
  .nav-label {
    font-size: 0.7rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 6px;
    display: block;
  }
  .nav-input-row {
    display: flex;
    gap: 6px;
  }
  .nav-input {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.8rem;
    font-family: var(--font-mono);
    outline: none;
  }
  .nav-input:focus { border-color: var(--accent); }
  .nav-go {
    padding: 6px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.8rem;
    cursor: pointer;
    transition: all var(--transition);
  }
  .nav-go:hover { background: var(--bg-hover); border-color: var(--accent); }
  .nav-total {
    font-size: 0.7rem;
    color: var(--text-muted);
    margin-top: 4px;
    display: block;
  }

  .heading-tree {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .tree-node {
    margin-bottom: 1px;
  }
  .tree-parent {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    width: 100%;
    padding: 8px 16px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1.6;
    text-align: right;
    cursor: pointer;
    transition: background var(--transition);
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
  }
  .tree-parent:hover { background: var(--bg-hover); }
  .tree-parent.active { color: var(--accent); background: var(--accent-muted); }
  .tree-parent.leaf { font-weight: 500; }
  .expand-icon {
    flex-shrink: 0;
    font-size: 0.7rem;
    transition: transform 0.2s ease;
    margin-top: 4px;
  }
  .expand-icon.expanded { transform: rotate(90deg); }

  .heading-title {
    flex: 1;
  }

  .tree-children {
    padding-right: 20px;
  }
  .tree-child {
    display: block;
    width: 100%;
    padding: 5px 16px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.6;
    text-align: right;
    cursor: pointer;
    transition: all var(--transition);
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
  }
  .tree-child:hover { background: var(--bg-hover); color: var(--text-primary); }
  .tree-child.active { color: var(--accent); background: var(--accent-muted); font-weight: 600; }

  @media (max-width: 768px) {
    .close-btn { display: flex; }
    .reader-sidebar { padding-top: 44px; }
  }
</style>
