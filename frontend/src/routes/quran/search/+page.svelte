<script lang="ts">
  import { page } from '$app/state';
  import { searchQuran } from '$lib/api';
  import type { QuranSearchResponse } from '$lib/types';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let result: QuranSearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'text' | 'semantic' | 'hybrid' | 'tafsir' = $state('text');

  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as typeof searchType) || 'text');

  $effect(() => {
    if (urlQuery) {
      query = urlQuery;
      searchType = urlType;
      doSearch();
    }
  });

  async function doSearch() {
    if (!query.trim()) return;
    loading = true;
    try {
      result = await searchQuran(query, searchType);
    } catch (e) {
      console.error('Quran search failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    window.history.pushState({}, '', `/quran/search?q=${encodeURIComponent(query)}&type=${searchType}`);
    doSearch();
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
      <button type="button" class="toggle-btn" class:active={searchType === 'tafsir'} onclick={() => searchType = 'tafsir'}>Tafsir</button>
    </div>
    <button type="submit" class="search-btn">Search</button>
  </form>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    {#if result.ayahs.length > 0}
      <section class="results-section">
        <h2>Results ({result.ayahs.length})</h2>
        <div class="results-list">
          {#each result.ayahs as ayah}
            <a href="/quran/{ayah.surah_number}#{ayah.surah_number}:{ayah.ayah_number}" class="result-link">
              <AyahCard {ayah} showScore compact />
            </a>
          {/each}
        </div>
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
</style>
