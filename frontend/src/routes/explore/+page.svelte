<script lang="ts">
  import { page } from '$app/state';
  import { searchUnified } from '$lib/api';
  import type { UnifiedSearchResponse, UnifiedSearchItem } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';

  let result: UnifiedSearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'hybrid' | 'semantic' = $state('semantic');
  let currentPage = $state(1);

  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as 'hybrid' | 'semantic') || 'semantic');
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
    try {
      result = await searchUnified(query, searchType, 20, currentPage);
    } catch (e) {
      console.error('Unified search failed:', e);
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
  }

  function pushUrl() {
    const sp = new URLSearchParams();
    sp.set('q', query);
    sp.set('type', searchType);
    if (currentPage > 1) sp.set('page', String(currentPage));
    window.history.pushState({}, '', `/explore?${sp}`);
  }

  function isQuran(item: UnifiedSearchItem): item is Extract<UnifiedSearchItem, { source: 'quran' }> {
    return item.source === 'quran';
  }
</script>

<div class="explore-page">
  <div class="explore-header">
    <h1>Quran & Sunnah</h1>
    <form class="search-form" onsubmit={handleSubmit}>
      <div class="search-bar">
        <span class="search-icon">&#x2315;</span>
        <input type="text" placeholder="Search Quran & Sunnah..." bind:value={query} class="search-input" />
      </div>
      <div class="search-controls">
        <div class="type-toggle">
          <button type="button" class="toggle-btn" class:active={searchType === 'hybrid'} onclick={() => searchType = 'hybrid'}>Hybrid</button>
          <button type="button" class="toggle-btn" class:active={searchType === 'semantic'} onclick={() => searchType = 'semantic'}>Semantic</button>
        </div>
        <button type="submit" class="search-btn">Search</button>
      </div>
    </form>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    {#if result.results.length > 0}
      <div class="results-summary">
        <span class="summary-count">{result.results.length} results</span>
        <span class="summary-breakdown">
          <span class="quran-count">{result.quran_count} from Quran</span>
          <span class="summary-dot">&middot;</span>
          <span class="hadith-count">{result.hadith_count} from Hadith</span>
        </span>
      </div>

      <div class="results-list">
        {#each result.results as item}
          {#if isQuran(item)}
            <div class="result-item">
              <div class="source-tag quran-tag">Quran</div>
              <a href="/quran/{item.surah_number}?ayah={item.ayah_number}" class="result-link">
                <AyahCard ayah={{
                  id: item.id,
                  surah_number: item.surah_number,
                  ayah_number: item.ayah_number,
                  text_ar: item.text_ar,
                  text_en: item.text_en,
                  tafsir_en: item.tafsir_en,

                }} compact />
              </a>
            </div>
          {:else}
            <div class="result-item">
              <div class="source-tag hadith-tag">Hadith</div>
              <a href="/hadiths/{item.id}" class="result-card">
                <div class="result-header">
                  <Badge text="Book {item.book_id}" />
                  <span class="hadith-num mono">#{item.hadith_number}</span>
                  {#if item.score}<span class="score mono">{item.score.toFixed(3)}</span>{/if}
                </div>
                {#if item.narrator_text}<p class="narrator">{item.narrator_text}</p>{/if}
                <p class="text">{$language === 'en' && item.text_en ? truncate(stripHtml(item.text_en), 200) : truncate(item.text_ar || stripHtml(item.text_en ?? ''), 200)}</p>
              </a>
            </div>
          {/if}
        {/each}
      </div>

      <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
    {:else}
      <div class="empty">No results found for "{result.query}".</div>
    {/if}
  {:else}
    <div class="empty-state">
      <div class="empty-icon">&#x2726;</div>
      <h2>Search across Quran & Sunnah</h2>
      <p>Find wisdom from the Quran and Prophetic tradition in a single search.</p>
    </div>
  {/if}
</div>

<style>
  .explore-page { padding: 24px; }

  .explore-header { margin-bottom: 24px; }
  .explore-header h1 {
    font-size: 1.4rem;
    margin-bottom: 16px;
    color: var(--text-primary);
  }

  .search-form { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .search-bar {
    flex: 1;
    min-width: 250px;
    max-width: 500px;
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 20px;
    padding: 0 14px;
    transition: border-color var(--transition);
  }
  .search-bar:focus-within { border-color: var(--accent); }
  .search-icon { color: var(--text-muted); font-size: 1rem; margin-right: 8px; }
  .search-input { flex: 1; border: none; background: transparent; padding: 10px 0; font-size: 0.9rem; }
  .search-controls { display: flex; gap: 8px; align-items: center; }
  .type-toggle { display: flex; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .toggle-btn { padding: 8px 14px; font-size: 0.8rem; background: var(--bg-surface); color: var(--text-secondary); transition: all var(--transition); border: none; cursor: pointer; }
  .toggle-btn.active { background: var(--accent); color: white; }
  .search-btn { padding: 8px 20px; background: var(--accent); color: white; border: none; border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; cursor: pointer; transition: background var(--transition); }
  .search-btn:hover { background: var(--accent-hover); }

  .results-summary {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .summary-count { font-weight: 600; color: var(--text-primary); }
  .summary-breakdown { display: flex; gap: 6px; align-items: center; }
  .quran-count { color: var(--success); }
  .hadith-count { color: var(--accent); }
  .summary-dot { color: var(--text-muted); }

  .results-list { display: flex; flex-direction: column; gap: 12px; }

  .result-item { position: relative; }
  .source-tag {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 8px;
    border-radius: 10px;
  }
  .quran-tag { background: rgba(63, 185, 80, 0.12); color: var(--success); }
  .hadith-tag { background: var(--accent-muted); color: var(--accent); }

  .result-link { display: block; color: var(--text-primary); }
  .result-link:hover { color: var(--text-primary); }

  .result-card {
    display: block;
    padding: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    transition: all var(--transition);
  }
  .result-card:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .result-header { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
  .hadith-num { color: var(--text-muted); font-size: 0.8rem; }
  .score { margin-left: auto; color: var(--success); font-size: 0.8rem; }
  .narrator { color: var(--accent); font-size: 0.85rem; margin-bottom: 4px; }
  .text { color: var(--text-secondary); font-size: 0.85rem; line-height: 1.5; }

  .empty { text-align: center; color: var(--text-muted); padding: 40px; }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 80px 24px;
    color: var(--text-secondary);
    gap: 12px;
  }
  .empty-icon { font-size: 2.5rem; color: var(--accent); }
  .empty-state h2 { color: var(--text-primary); font-size: 1.2rem; }
  .empty-state p { max-width: 400px; line-height: 1.6; font-size: 0.9rem; }

  @media (max-width: 640px) {
    .search-bar { max-width: 100%; }
    .source-tag { top: 4px; right: 4px; }
  }
</style>
