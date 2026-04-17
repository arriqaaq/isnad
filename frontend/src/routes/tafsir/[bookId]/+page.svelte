<script lang="ts">
  import { page } from '$app/state';
  import { getTurathBook, getTurathPages } from '$lib/api';
  import type { TurathBookDetail, TurathPage } from '$lib/types';
  import ReaderContent from '$lib/components/reader/ReaderContent.svelte';
  import ReaderHeader from '$lib/components/reader/ReaderHeader.svelte';
  import ReaderSidebar from '$lib/components/reader/ReaderSidebar.svelte';
  import SidebarTabs from '$lib/components/reader/SidebarTabs.svelte';
  import BookChat from '$lib/components/reader/BookChat.svelte';
  import ResizeHandle from '$lib/components/layout/ResizeHandle.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import { loadTurathConfig, getBookConfig } from '$lib/stores/turath';
  import type { TurathBooksConfig } from '$lib/types';

  let turathConfig: TurathBooksConfig | null = $state(null);
  let bookConfig = $derived(
    turathConfig ? getBookConfig(turathConfig, bookId) : undefined
  );
  let chatDefaultQuestions = $derived(bookConfig?.default_questions ?? []);

  let bookId = $derived(Number((page.params as Record<string, string>).bookId));
  let initialPage = $derived(Number(page.url.searchParams.get('page')) || 0);

  let book: TurathBookDetail | null = $state(null);
  let pages: Map<number, TurathPage> = $state(new Map());
  let loading = $state(true);
  let currentPageIndex = $state(0);
  let readerRef: ReturnType<typeof ReaderContent> | undefined = $state(undefined);

  // Right sidebar state
  let rightCollapsed = $state(false);
  let rightWidth = $state(300);
  const RIGHT_MIN = 220;
  const RIGHT_MAX = 700;
  const RIGHT_COLLAPSED_W = 40;

  // Mobile drawer state
  let mobileDrawerOpen = $state(false);

  // Track which page ranges are already fetched or in-flight
  let fetchedRanges: Set<string> = new Set();
  let PAGE_SIZE = 20;

  async function fetchPageChunk(start: number) {
    const rangeKey = `${start}-${start + PAGE_SIZE}`;
    if (fetchedRanges.has(rangeKey)) return;
    fetchedRanges.add(rangeKey);

    try {
      const res = await getTurathPages(bookId, start, PAGE_SIZE);
      const next = new Map(pages);
      for (const p of res.pages) {
        next.set(p.page_index, p);
      }
      pages = next;
    } catch (e) {
      console.error('Failed to fetch pages:', e);
      fetchedRanges.delete(rangeKey);
    }
  }

  $effect(() => {
    loadTurathConfig().then((c) => { turathConfig = c; });
  });

  $effect(() => {
    loading = true;
    pages = new Map();
    fetchedRanges = new Set();

    getTurathBook(bookId)
      .then(async (b) => {
        book = b;

        if (initialPage > 0) {
          const chunkStart = Math.max(0, initialPage - 5);
          await fetchPageChunk(chunkStart);
          currentPageIndex = initialPage;
          if (chunkStart > 0) fetchPageChunk(0);
        } else {
          await fetchPageChunk(0);
        }

        loading = false;

        if (initialPage > 0) {
          requestAnimationFrame(() => {
            setTimeout(() => {
              if (readerRef) readerRef.scrollToPage(initialPage);
            }, 200);
          });
        }
      })
      .catch((e) => {
        console.error('Failed to load book:', e);
        loading = false;
      });

    // Restore sidebar state
    if (typeof localStorage !== 'undefined') {
      try {
        const saved = JSON.parse(localStorage.getItem('tafsir_right_sidebar') ?? '{}');
        if (typeof saved.collapsed === 'boolean') rightCollapsed = saved.collapsed;
        if (typeof saved.width === 'number') rightWidth = Math.max(RIGHT_MIN, Math.min(saved.width, RIGHT_MAX));
      } catch { /* ignore */ }
    }
  });

  function saveRightState() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('tafsir_right_sidebar', JSON.stringify({ collapsed: rightCollapsed, width: rightWidth }));
    }
  }

  function handleNeedMore(startIndex: number) {
    const chunkStart = Math.floor(startIndex / PAGE_SIZE) * PAGE_SIZE;
    fetchPageChunk(chunkStart);
  }

  function handleSidebarNavigate(pageIndex: number) {
    const chunkStart = Math.floor(pageIndex / PAGE_SIZE) * PAGE_SIZE;
    fetchPageChunk(chunkStart).then(() => {
      if (readerRef) readerRef.scrollToPage(pageIndex);
    });
    currentPageIndex = pageIndex;
  }

  function handleRightDrag(deltaX: number) {
    rightWidth = Math.max(RIGHT_MIN, Math.min(rightWidth - deltaX, RIGHT_MAX));
    saveRightState();
  }

  function toggleRight() {
    rightCollapsed = !rightCollapsed;
    saveRightState();
  }
</script>

<svelte:head>
  <title>{book ? book.name_ar : 'Tafsir'} - Ilm</title>
</svelte:head>

{#if loading}
  <div class="loading-container">
    <LoadingSpinner />
  </div>
{:else if book}
  <div class="tafsir-reader">
    <ReaderHeader
      {book}
      currentPage={currentPageIndex}
      totalPages={book.total_pages}
      onToggleSidebar={toggleRight}
    />

    <div class="reader-body">
      <div class="reader-main">
        <ReaderContent
          bind:this={readerRef}
          {pages}
          bind:currentPageIndex
          totalPages={book.total_pages}
          onNeedMore={handleNeedMore}
        />
      </div>

      <!-- Desktop right sidebar -->
      <div class="right-sidebar-area">
        <ResizeHandle ondrag={handleRightDrag} />
        <div
          class="right-sidebar"
          class:right-collapsed={rightCollapsed}
          style="width: {rightCollapsed ? RIGHT_COLLAPSED_W : rightWidth}px"
        >
          {#if rightCollapsed}
            <button class="expand-btn" onclick={toggleRight} title="Expand panel">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 18 9 12 15 6"/></svg>
            </button>
          {:else}
            <SidebarTabs>
              {#snippet content()}
                <ReaderSidebar
                  headings={book.headings}
                  {currentPageIndex}
                  totalPages={book.total_pages}
                  onNavigate={handleSidebarNavigate}
                  onClose={toggleRight}
                />
              {/snippet}
              {#snippet chat()}
                <BookChat
                  {bookId}
                  bookName={book.name_en}
                  {currentPageIndex}
                  onNavigate={handleSidebarNavigate}
                  defaultQuestions={chatDefaultQuestions}
                />
              {/snippet}
            </SidebarTabs>
          {/if}
        </div>
      </div>
    </div>

    <!-- Mobile floating button -->
    <button class="mobile-sidebar-btn" onclick={() => { mobileDrawerOpen = true; }} aria-label="Open panel">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="15" y2="12"/><line x1="3" y1="18" x2="18" y2="18"/></svg>
    </button>

    <!-- Mobile drawer -->
    {#if mobileDrawerOpen}
      <button class="mobile-backdrop" onclick={() => { mobileDrawerOpen = false; }} aria-label="Close sidebar"></button>
      <div class="mobile-drawer">
        <SidebarTabs>
          {#snippet content()}
            <ReaderSidebar
              headings={book.headings}
              {currentPageIndex}
              totalPages={book.total_pages}
              onNavigate={(idx) => { mobileDrawerOpen = false; handleSidebarNavigate(idx); }}
              onClose={() => { mobileDrawerOpen = false; }}
            />
          {/snippet}
          {#snippet chat()}
            <BookChat
              {bookId}
              bookName={book.name_en}
              {currentPageIndex}
              onNavigate={(idx) => { mobileDrawerOpen = false; handleSidebarNavigate(idx); }}
            />
          {/snippet}
        </SidebarTabs>
      </div>
    {/if}
  </div>
{:else}
  <div class="error-container">
    <p>Book not found.</p>
    <a href="/quran">Back to Quran</a>
  </div>
{/if}

<style>
  .tafsir-reader {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    position: relative;
  }

  .reader-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .reader-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .right-sidebar-area {
    display: flex;
    flex-shrink: 0;
  }

  .right-sidebar {
    height: 100%;
    overflow: hidden;
    transition: width 200ms ease;
    border-left: none;
  }

  .right-sidebar.right-collapsed {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .expand-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition);
  }
  .expand-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }

  .loading-container, .error-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    color: var(--text-muted);
  }
  .error-container a {
    color: var(--accent);
  }

  .mobile-sidebar-btn { display: none; }
  .mobile-backdrop { display: none; }
  .mobile-drawer { display: none; }

  @media (max-width: 768px) {
    .right-sidebar-area {
      display: none;
    }

    .mobile-sidebar-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      position: fixed;
      bottom: 20px;
      right: 16px;
      width: 44px;
      height: 44px;
      border-radius: 50%;
      border: 1px solid var(--border);
      background: var(--bg-surface);
      color: var(--text-secondary);
      box-shadow: 0 2px 8px rgba(0,0,0,0.12);
      cursor: pointer;
      z-index: 30;
      transition: all var(--transition);
    }
    .mobile-sidebar-btn:hover {
      border-color: var(--accent);
      color: var(--accent);
    }

    .mobile-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 49;
      border: none;
      padding: 0;
      cursor: default;
    }

    .mobile-drawer {
      display: block;
      position: fixed;
      top: 0;
      right: 0;
      width: 85%;
      max-width: 360px;
      height: 100vh;
      z-index: 50;
      background: var(--bg-primary);
      box-shadow: -4px 0 20px rgba(0, 0, 0, 0.15);
      overflow-y: auto;
    }
  }
</style>
