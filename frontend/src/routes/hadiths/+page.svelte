<script lang="ts">
  import { page } from '$app/state';
  import { getHadiths, getHadithSharhPages } from '$lib/api';
  import type { ApiHadith, PaginatedResponse, SharhPageRef } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let result: PaginatedResponse<ApiHadith> | null = $state(null);
  let loading = $state(true);
  let sharhMappings: Record<string, SharhPageRef> = $state({});
  let sharhTarget: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number } | null = $state(null);

  let currentPage = $derived(Number(page.url.searchParams.get('page')) || 1);
  let bookFilter = $derived(page.url.searchParams.get('book') ? Number(page.url.searchParams.get('book')) : undefined);

  async function load() {
    loading = true;
    try {
      result = await getHadiths({ book: bookFilter, page: currentPage });

      // Fetch sharh mappings for visible hadiths
      if (result && result.data.length > 0) {
        const numbers = result.data.map(h => h.hadith_number);
        const bookId = result.data[0]?.collection_id ?? 1;
        getHadithSharhPages(bookId, numbers)
          .then(res => { sharhMappings = res.mappings; })
          .catch(() => {});
      }
    } catch (e) {
      console.error('Failed to load hadiths:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void currentPage;
    void bookFilter;
    load();
  });

  function changePage(newPage: number) {
    const sp = new URLSearchParams();
    sp.set('page', String(newPage));
    if (bookFilter) sp.set('book', String(bookFilter));
    window.history.pushState({}, '', `/hadiths?${sp}`);
  }
</script>

<div class="hadith-list">
  <div class="list-header">
    <h1>Hadiths</h1>
    {#if bookFilter}
      <span class="filter-badge">Book {bookFilter}</span>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if result && result.data.length > 0}
    <div class="list">
      {#each result.data as hadith (hadith.id)}
        <HadithCard
          {hadith}
          sharhPage={sharhMappings[String(hadith.hadith_number)]}
          onopensharh={(info) => { sharhTarget = info; }}
        />
      {/each}
    </div>
    <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
  {:else}
    <div class="empty">No hadiths found.</div>
  {/if}
</div>

{#if sharhTarget}
  <BookViewerModal
    bookId={sharhTarget.bookId}
    pageIndex={sharhTarget.pageIndex}
    title={sharhTarget.bookName}
    subtitle="Hadith {sharhTarget.hadithNumber}"
    onclose={() => { sharhTarget = null; }}
  />
{/if}

<style>
  .hadith-list { padding: 24px; }
  .list-header { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; }
  .filter-badge { padding: 4px 12px; background: var(--accent-muted); color: var(--accent); border-radius: 20px; font-size: 0.8rem; font-weight: 500; }
  .list { display: flex; flex-direction: column; gap: 12px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
