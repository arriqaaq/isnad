<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { searchQuran, searchByRoot } from '$lib/api';
  import type { QuranSearchResponse, RootSearchResponse } from '$lib/types';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let result: QuranSearchResponse | null = $state(null);
  let rootResult: RootSearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'text' | 'semantic' | 'hybrid' | 'root' = $state('text');
  let currentPage = $state(1);

  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as typeof searchType) || 'text');
  let urlPage = $derived(Number(page.url.searchParams.get('page')) || 1);

  $effect(() => {
    if (urlQuery) {
      query = urlQuery;
      searchType = urlType;
      currentPage = urlPage;
      doSearch();
    }
  });

  async function doSearch() {
    if (!query.trim()) return;
    loading = true;
    result = null;
    rootResult = null;
    try {
      if (searchType === 'root') {
        rootResult = await searchByRoot(query);
      } else {
        result = await searchQuran(query, searchType as 'text' | 'semantic' | 'hybrid', 20, currentPage);
      }
    } catch (e) {
      console.error('Quran search failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    currentPage = 1;
    pushUrl();
    doSearch();
  }

  function changePage(newPage: number) {
    currentPage = newPage;
    pushUrl();
    doSearch();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function pushUrl() {
    const sp = new URLSearchParams();
    sp.set('q', query);
    sp.set('type', searchType);
    if (currentPage > 1) sp.set('page', String(currentPage));
    window.history.pushState({}, '', `/quran/search?${sp}`);
  }
</script>

<div class="search-page">
  <h1>Quran Search</h1>

  <form class="search-form" onsubmit={handleSubmit}>
    <input type="text" placeholder="Search the Quran..." bind:value={query} class="search-input" />
    <div class="type-toggle">
      <button type="button" class="toggle-btn" class:active={searchType === 'text'} onclick={() => searchType = 'text'}>Text</button>
      <button type="button" class="toggle-btn" class:active={searchType === 'semantic'} onclick={() => searchType = 'semantic'}>Semantic</button>
      <button type="button" class="toggle-btn" class:active={searchType === 'hybrid'} onclick={() => searchType = 'hybrid'}>Hybrid</button>
      <button type="button" class="toggle-btn" class:active={searchType === 'root'} onclick={() => searchType = 'root'}>Root</button>
    </div>
    <button type="submit" class="search-btn">Search</button>
  </form>

  {#if loading}
    <LoadingSpinner />
  {:else if rootResult}
    {#if rootResult.occurrences.length > 0}
      <section class="results-section">
        <h2 dir="rtl">{rootResult.root} - {rootResult.occurrences.length} words in {rootResult.ayah_count} ayahs</h2>
        <a href="/quran/root/{encodeURIComponent(rootResult.root)}" class="view-all">View detailed root page</a>
      </section>
    {:else}
      <div class="empty">No words found for root "{query}".</div>
    {/if}
  {:else if result}
    {#if result.ayahs.length > 0}
      <section class="results-section">
        <h2>Results</h2>
        <div class="results-list">
          {#each result.ayahs as ayah}
            <a href="/quran/{ayah.surah_number}?ayah={ayah.ayah_number}" class="result-link">
              <AyahCard {ayah} showScore compact />
            </a>
          {/each}
        </div>
        <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
      </section>
    {:else}
      <div class="empty">No results found for "{result.query}".</div>
    {/if}
  {/if}
</div>

<style>
  .search-page { padding: 24px; }
  h1 { margin-bottom: 20px; }
  .search-form { display: flex; gap: 8px; margin-bottom: 24px; align-items: center; flex-wrap: wrap; }
  .search-input { flex: 1; min-width: 250px; max-width: 500px; }
  .type-toggle { display: flex; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .toggle-btn { padding: 8px 12px; font-size: 0.8rem; background: var(--bg-surface); color: var(--text-secondary); transition: all var(--transition); }
  .toggle-btn.active { background: var(--accent); color: var(--bg-primary); }
  .search-btn { padding: 8px 20px; background: var(--accent); color: var(--bg-primary); border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; transition: background var(--transition); }
  .search-btn:hover { background: var(--accent-hover); }
  .results-section { margin-bottom: 28px; }
  .results-section h2 { margin-bottom: 12px; }
  .results-list { display: flex; flex-direction: column; gap: 10px; }
  .result-link { color: var(--text-primary); }
  .result-link:hover { color: var(--text-primary); }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
  .view-all { font-size: 0.85rem; color: var(--accent); }
</style>
