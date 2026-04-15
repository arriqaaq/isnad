<script lang="ts">
  import { page } from '$app/state';
  import { getTurathBook, getTurathPages } from '$lib/api';
  import type { TurathBookDetail, TurathPage } from '$lib/types';
  import ReaderContent from '$lib/components/reader/ReaderContent.svelte';
  import ReaderHeader from '$lib/components/reader/ReaderHeader.svelte';
  import ReaderSidebar from '$lib/components/reader/ReaderSidebar.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let bookId = $derived(Number((page.params as Record<string, string>).bookId));
  let initialPage = $derived(Number(page.url.searchParams.get('page')) || 0);

  let book: TurathBookDetail | null = $state(null);
  let pages: Map<number, TurathPage> = $state(new Map());
  let loading = $state(true);
  let currentPageIndex = $state(0);
  let sidebarOpen = $state(false);
  let readerRef: ReturnType<typeof ReaderContent> | undefined = $state(undefined);

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
    loading = true;
    pages = new Map();
    fetchedRanges = new Set();

    getTurathBook(bookId)
      .then(async (b) => {
        book = b;

        // Fetch initial pages
        if (initialPage > 0) {
          // Fetch the chunk containing the target page
          const chunkStart = Math.max(0, initialPage - 5);
          await fetchPageChunk(chunkStart);
          currentPageIndex = initialPage;
          // Also fetch first chunk for context
          if (chunkStart > 0) fetchPageChunk(0);
        } else {
          await fetchPageChunk(0);
        }

        loading = false;

        // After render, scroll to target page
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
  });

  function handleNeedMore(startIndex: number) {
    // Align to chunk boundary
    const chunkStart = Math.floor(startIndex / PAGE_SIZE) * PAGE_SIZE;
    fetchPageChunk(chunkStart);
  }

  function handleSidebarNavigate(pageIndex: number) {
    // Ensure the target chunk is loaded
    const chunkStart = Math.floor(pageIndex / PAGE_SIZE) * PAGE_SIZE;
    fetchPageChunk(chunkStart).then(() => {
      if (readerRef) readerRef.scrollToPage(pageIndex);
    });
    currentPageIndex = pageIndex;
  }

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
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
      onToggleSidebar={toggleSidebar}
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

      <div class="sidebar-panel" class:open={sidebarOpen}>
        <ReaderSidebar
          headings={book.headings}
          {currentPageIndex}
          totalPages={book.total_pages}
          onNavigate={handleSidebarNavigate}
          onClose={() => { sidebarOpen = false; }}
        />
      </div>

      {#if sidebarOpen}
        <button class="sidebar-backdrop" onclick={() => { sidebarOpen = false; }} aria-label="Close sidebar"></button>
      {/if}
    </div>
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
  }

  .reader-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .reader-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .sidebar-panel {
    width: 320px;
    flex-shrink: 0;
    overflow: hidden;
    border-left: 1px solid var(--border);
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

  .sidebar-backdrop {
    display: none;
  }

  @media (max-width: 768px) {
    .sidebar-panel {
      display: none;
      position: fixed;
      top: 0;
      right: 0;
      width: 85%;
      max-width: 360px;
      height: 100vh;
      z-index: 50;
      background: var(--bg-primary);
      box-shadow: -4px 0 20px rgba(0, 0, 0, 0.15);
    }
    .sidebar-panel.open {
      display: block;
    }
    .sidebar-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 49;
      border: none;
      padding: 0;
      cursor: default;
    }
  }
</style>
