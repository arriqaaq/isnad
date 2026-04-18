<script lang="ts">
  import { onMount } from 'svelte';
  import { getBook, getBookPages } from '$lib/api';
  import type { BookPage, BookHeading } from '$lib/types';
  import ReaderPage from './ReaderPage.svelte';

  let { bookId, pageIndex, ayahRef, onclose }: {
    bookId: number;
    pageIndex: number;
    ayahRef: string;
    onclose: () => void;
  } = $props();

  let loading = $state(true);
  let pages: Map<number, BookPage> = $state(new Map());
  let headings: BookHeading[] = $state([]);
  let totalPages = $state(0);
  // svelte-ignore state_referenced_locally — intentional: one-shot init from prop
  let currentIndex = $state(pageIndex);
  let sidebarOpen = $state(false);

  // Panel position & size
  let panelX = $state(0);
  let panelY = $state(0);
  let panelW = $state(750);
  let panelH = $state(0);

  // Drag state
  let dragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let dragStartPanelX = 0;
  let dragStartPanelY = 0;

  // Resize state
  let resizing = $state(false);
  let resizeEdge = '';
  let resizeStartX = 0;
  let resizeStartY = 0;
  let resizeStartW = 0;
  let resizeStartH = 0;
  let resizeStartPanelX = 0;
  let resizeStartPanelY = 0;

  let currentPage = $derived(pages.get(currentIndex));

  // Find current heading based on page index
  let currentHeadingTitle = $derived.by(() => {
    let best = '';
    for (const h of headings) {
      if (h.page_index <= currentIndex) best = h.title;
      else break;
    }
    return best;
  });

  // Build sidebar tree (level 1 = parents, level 2+ = children)
  interface HeadingNode {
    heading: BookHeading;
    children: HeadingNode[];
  }
  let headingTree = $derived.by(() => {
    const nodes: HeadingNode[] = [];
    let parent: HeadingNode | null = null;
    for (const h of headings) {
      if (h.level === 1) {
        parent = { heading: h, children: [] };
        nodes.push(parent);
      } else if (parent) {
        parent.children.push({ heading: h, children: [] });
      } else {
        nodes.push({ heading: h, children: [] });
      }
    }
    return nodes;
  });

  let expandedSections: Set<number> = $state(new Set());

  onMount(() => {
    panelW = Math.min(750, window.innerWidth - 40);
    panelH = Math.min(window.innerHeight - 40, window.innerHeight * 0.9);
    panelX = Math.max(20, (window.innerWidth - panelW) / 2);
    panelY = Math.max(20, (window.innerHeight - panelH) / 2);

    // Fetch book metadata (for headings) and initial pages
    Promise.all([
      getBook(bookId),
      fetchAround(pageIndex),
    ]).then(([book]) => {
      headings = book.headings;
      totalPages = book.total_pages;
    }).catch(e => console.error('Failed to load tafsir:', e))
      .finally(() => { loading = false; });

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('keydown', handleKeydown);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('keydown', handleKeydown);
    };
  });

  async function fetchAround(center: number) {
    const start = Math.max(0, center - 2);
    try {
      const res = await getBookPages(bookId, start, 7);
      const next = new Map(pages);
      for (const p of res.pages) next.set(p.page_index, p);
      pages = next;
      if (res.total > 0) totalPages = res.total;
    } catch (e) {
      console.error('Failed to fetch tafsir pages:', e);
    }
  }

  function goTo(index: number) {
    if (index < 0 || (totalPages > 0 && index >= totalPages)) return;
    currentIndex = index;
    if (!pages.has(index)) fetchAround(index);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { onclose(); return; }
    if (e.key === 'ArrowRight') { goTo(currentIndex - 1); e.preventDefault(); }
    if (e.key === 'ArrowLeft') { goTo(currentIndex + 1); e.preventDefault(); }
  }

  function toggleSection(idx: number) {
    const next = new Set(expandedSections);
    if (next.has(idx)) next.delete(idx); else next.add(idx);
    expandedSections = next;
  }

  function sidebarNavigate(pi: number) {
    goTo(pi);
    sidebarOpen = false;
  }

  // Drag/resize handlers (same pattern as AyahDetailsModal)
  function startDrag(e: MouseEvent) {
    dragging = true; dragStartX = e.clientX; dragStartY = e.clientY;
    dragStartPanelX = panelX; dragStartPanelY = panelY; e.preventDefault();
  }
  function startResize(e: MouseEvent, edge: string) {
    resizing = true; resizeEdge = edge;
    resizeStartX = e.clientX; resizeStartY = e.clientY;
    resizeStartW = panelW; resizeStartH = panelH;
    resizeStartPanelX = panelX; resizeStartPanelY = panelY;
    e.preventDefault(); e.stopPropagation();
  }
  function handleMouseMove(e: MouseEvent) {
    if (dragging) {
      panelX = Math.max(0, Math.min(window.innerWidth - 100, dragStartPanelX + e.clientX - dragStartX));
      panelY = Math.max(0, Math.min(window.innerHeight - 50, dragStartPanelY + e.clientY - dragStartY));
    } else if (resizing) {
      const dx = e.clientX - resizeStartX; const dy = e.clientY - resizeStartY;
      if (resizeEdge.includes('w')) { const w = Math.max(450, resizeStartW - dx); panelX = resizeStartPanelX + (resizeStartW - w); panelW = w; }
      if (resizeEdge.includes('e')) panelW = Math.max(450, resizeStartW + dx);
      if (resizeEdge.includes('n')) { const h = Math.max(350, resizeStartH - dy); panelY = resizeStartPanelY + (resizeStartH - h); panelH = h; }
      if (resizeEdge.includes('s')) panelH = Math.max(350, resizeStartH + dy);
    }
  }
  function handleMouseUp() { dragging = false; resizing = false; }
  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('modal-backdrop')) onclose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdrop}>
  <div
    class="tafsir-panel"
    class:is-moving={dragging || resizing}
    style="left:{panelX}px; top:{panelY}px; width:{panelW}px; height:{panelH}px;"
  >
    <!-- Resize handles -->
    <div class="rh rh-n" onmousedown={(e) => startResize(e, 'n')}></div>
    <div class="rh rh-s" onmousedown={(e) => startResize(e, 's')}></div>
    <div class="rh rh-w" onmousedown={(e) => startResize(e, 'w')}></div>
    <div class="rh rh-e" onmousedown={(e) => startResize(e, 'e')}></div>
    <div class="rh rh-nw" onmousedown={(e) => startResize(e, 'nw')}></div>
    <div class="rh rh-ne" onmousedown={(e) => startResize(e, 'ne')}></div>
    <div class="rh rh-sw" onmousedown={(e) => startResize(e, 'sw')}></div>
    <div class="rh rh-se" onmousedown={(e) => startResize(e, 'se')}></div>

    <!-- Header -->
    <div class="panel-header" onmousedown={startDrag}>
      <div class="header-left">
        <button class="icon-btn sidebar-btn" onclick={(e) => { e.stopPropagation(); sidebarOpen = !sidebarOpen; }} title="Chapters">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="15" y2="12"/><line x1="3" y1="18" x2="18" y2="18"/></svg>
        </button>
        <span class="header-title" dir="rtl">Tafsir Ibn Kathir</span>
        <span class="header-ref">{ayahRef}</span>
      </div>
      <div class="header-right">
        <span class="drag-hint">&#x2807;</span>
        <button class="icon-btn close-btn" onclick={onclose}>&times;</button>
      </div>
    </div>

    <!-- Body: sidebar + content -->
    <div class="panel-body">
      <!-- Sidebar -->
      {#if sidebarOpen}
        <div class="panel-sidebar" dir="rtl">
          <div class="sidebar-scroll">
            {#each headingTree as node, ni}
              <div class="tree-node">
                {#if node.children.length > 0}
                  <button class="tree-parent" onclick={() => toggleSection(ni)}>
                    <span class="expand-icon" class:expanded={expandedSections.has(ni)}>&#9656;</span>
                    <span>{node.heading.title}</span>
                  </button>
                  {#if expandedSections.has(ni)}
                    <div class="tree-children">
                      {#each node.children as child}
                        <button
                          class="tree-child"
                          class:active={child.heading.page_index === currentIndex}
                          onclick={() => sidebarNavigate(child.heading.page_index)}
                        >{child.heading.title}</button>
                      {/each}
                    </div>
                  {/if}
                {:else}
                  <button class="tree-parent leaf" onclick={() => sidebarNavigate(node.heading.page_index)}>
                    <span>{node.heading.title}</span>
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Content -->
      <div class="panel-content">
        {#if loading}
          <div class="loading-state">Loading tafsir...</div>
        {:else if currentPage}
          <ReaderPage page={currentPage} />
        {:else}
          <div class="loading-state">Page not available</div>
        {/if}
      </div>
    </div>

    <!-- Footer navigation -->
    <div class="panel-footer">
      <button class="footer-nav" onclick={() => goTo(currentIndex + 1)} disabled={totalPages > 0 && currentIndex >= totalPages - 1}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg>
        Next
      </button>

      <div class="footer-center">
        <span class="footer-page-info">
          {#if currentPage}
            Vol {currentPage.vol} &middot; Page {currentPage.page_num}
          {/if}
        </span>
        <span class="footer-index">{currentIndex + 1} / {totalPages || '?'}</span>
      </div>

      <button class="footer-nav" onclick={() => goTo(currentIndex - 1)} disabled={currentIndex <= 0}>
        Prev
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>
      </button>
    </div>

    <!-- Full reader link -->
    <div class="panel-link-bar">
      <a href="/tafsir/{bookId}?page={currentIndex}" class="full-reader-link">Open full reader &#x2197;</a>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 200;
    animation: fadeIn 0.15s ease-out;
  }

  .tafsir-panel {
    position: fixed;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.25);
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease-out;
    min-width: 450px;
    min-height: 350px;
    overflow: hidden;
  }
  .tafsir-panel.is-moving { user-select: none; transition: none; }

  /* Header */
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    cursor: grab;
    background: var(--bg-surface);
  }
  .panel-header:active { cursor: grabbing; }

  .header-left, .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .header-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
  }
  .header-ref {
    font-size: 0.7rem;
    color: var(--accent);
    font-family: var(--font-mono);
    background: var(--accent-muted);
    padding: 1px 7px;
    border-radius: 10px;
  }
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
    font-size: 1.3rem;
    line-height: 1;
  }
  .icon-btn:hover { background: var(--bg-hover); border-color: var(--accent); color: var(--text-primary); }
  .close-btn { border: none; background: none; }
  .drag-hint { color: var(--text-muted); font-size: 0.9rem; opacity: 0.4; }

  /* Body: sidebar + content */
  .panel-body {
    flex: 1;
    display: flex;
    overflow: hidden;
    min-height: 0;
  }

  .panel-sidebar {
    width: 240px;
    flex-shrink: 0;
    border-left: 1px solid var(--border);
    background: var(--bg-surface);
    overflow: hidden;
    animation: slideRight 0.15s ease-out;
  }
  .sidebar-scroll {
    height: 100%;
    overflow-y: auto;
    padding: 8px 0;
  }

  .tree-node { margin-bottom: 1px; }
  .tree-parent {
    display: flex;
    align-items: flex-start;
    gap: 5px;
    width: 100%;
    padding: 6px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.78rem;
    font-weight: 600;
    line-height: 1.5;
    text-align: right;
    cursor: pointer;
    transition: background var(--transition);
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
  }
  .tree-parent:hover { background: var(--bg-hover); }
  .tree-parent.leaf { font-weight: 500; }
  .expand-icon { flex-shrink: 0; font-size: 0.65rem; transition: transform 0.15s; margin-top: 3px; }
  .expand-icon.expanded { transform: rotate(90deg); }
  .tree-children { padding-right: 16px; }
  .tree-child {
    display: block;
    width: 100%;
    padding: 4px 12px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 0.72rem;
    line-height: 1.5;
    text-align: right;
    cursor: pointer;
    transition: all var(--transition);
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
  }
  .tree-child:hover { background: var(--bg-hover); color: var(--text-primary); }
  .tree-child.active { color: var(--accent); background: var(--accent-muted); font-weight: 600; }

  /* Content */
  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px 20px;
    min-width: 0;
  }
  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  /* Footer navigation */
  .panel-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-surface);
    gap: 8px;
  }
  .footer-nav {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 14px;
    cursor: pointer;
    transition: all var(--transition);
    white-space: nowrap;
  }
  .footer-nav:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--accent);
    color: var(--accent);
  }
  .footer-nav:disabled { opacity: 0.3; cursor: default; }
  .footer-nav svg { flex-shrink: 0; }

  .footer-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    min-width: 0;
  }
  .footer-page-info {
    font-size: 0.75rem;
    color: var(--text-primary);
    font-weight: 500;
  }
  .footer-index {
    font-size: 0.65rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* Full reader link */
  .panel-link-bar {
    padding: 4px 12px 8px;
    text-align: center;
    flex-shrink: 0;
    background: var(--bg-surface);
  }
  .full-reader-link {
    font-size: 0.68rem;
    color: var(--text-muted);
    text-decoration: none;
    transition: color var(--transition);
  }
  .full-reader-link:hover { color: var(--accent); text-decoration: underline; }

  /* Resize handles */
  .rh { position: absolute; z-index: 10; }
  .rh-n { top: -3px; left: 12px; right: 12px; height: 6px; cursor: n-resize; }
  .rh-s { bottom: -3px; left: 12px; right: 12px; height: 6px; cursor: s-resize; }
  .rh-w { left: -3px; top: 12px; bottom: 12px; width: 6px; cursor: w-resize; }
  .rh-e { right: -3px; top: 12px; bottom: 12px; width: 6px; cursor: e-resize; }
  .rh-nw { top: -3px; left: -3px; width: 12px; height: 12px; cursor: nw-resize; }
  .rh-ne { top: -3px; right: -3px; width: 12px; height: 12px; cursor: ne-resize; }
  .rh-sw { bottom: -3px; left: -3px; width: 12px; height: 12px; cursor: sw-resize; }
  .rh-se { bottom: -3px; right: -3px; width: 12px; height: 12px; cursor: se-resize; }

  @keyframes slideUp {
    from { transform: translateY(16px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  @keyframes slideRight {
    from { transform: translateX(20px); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }

  @media (max-width: 640px) {
    .tafsir-panel {
      left: 0 !important;
      top: 0 !important;
      width: 100vw !important;
      height: 100vh !important;
      border-radius: 0;
    }
    .rh { display: none; }
    .panel-sidebar { width: 200px; }
  }
</style>
