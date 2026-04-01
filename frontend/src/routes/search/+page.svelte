<script lang="ts">
  import { page } from '$app/state';
  import { searchAll } from '$lib/api';
  import type { SearchResponse } from '$lib/types';
  import { truncate, stripHtml, formatScore } from '$lib/utils';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let result: SearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'text' | 'semantic' = $state('text');

  // React to URL param changes (e.g., from TopBar navigation)
  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as 'text' | 'semantic') || 'text');

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
      result = await searchAll(query, searchType);
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    window.history.pushState({}, '', `/search?q=${encodeURIComponent(query)}&type=${searchType}`);
    doSearch();
  }
</script>

<div class="search-page">
  <h1>Search</h1>

  <form class="search-form" onsubmit={handleSubmit}>
    <input type="text" placeholder="Search hadiths and narrators..." bind:value={query} class="search-input" />
    <div class="type-toggle">
      <button type="button" class="toggle-btn" class:active={searchType === 'text'} onclick={() => searchType = 'text'}>Text</button>
      <button type="button" class="toggle-btn" class:active={searchType === 'semantic'} onclick={() => searchType = 'semantic'}>Semantic</button>
    </div>
    <button type="submit" class="search-btn">Search</button>
  </form>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    {#if result.hadiths.length > 0}
      <section class="results-section">
        <h2>Hadiths ({result.hadiths.length})</h2>
        <div class="results-list">
          {#each result.hadiths as h}
            <a href="/hadiths/{h.id}" class="result-card">
              <div class="result-header">
                <Badge text="Book {h.book_id}" />
                <span class="hadith-num mono">#{h.hadith_number}</span>
                {#if h.score}<span class="score mono">{formatScore(h.score)}</span>{/if}
              </div>
              {#if h.narrator_text}<p class="narrator">{h.narrator_text}</p>{/if}
              <p class="text">{h.text_en ? truncate(stripHtml(h.text_en), 200) : truncate(h.text_ar, 200)}</p>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if result.narrators.length > 0}
      <section class="results-section">
        <h2>Narrators ({result.narrators.length})</h2>
        <div class="results-list">
          {#each result.narrators as n}
            <a href="/narrators/{n.id}" class="result-card">
              <div class="result-header">
                <span class="narrator-name">{n.name_ar || n.name_en}</span>
                {#if n.generation}<Badge text={n.generation} variant="accent" />{/if}
              </div>
              {#if n.name_ar}<p class="name-ar arabic" dir="rtl">{n.name_ar}</p>{/if}
              {#if n.hadith_count}<span class="hadith-count mono">{n.hadith_count} hadiths</span>{/if}
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if result.hadiths.length === 0 && result.narrators.length === 0}
      <div class="empty">No results found for "{result.query}".</div>
    {/if}
  {/if}
</div>

<style>
  .search-page { padding: 24px; }
  h1 { margin-bottom: 20px; }
  .search-form { display: flex; gap: 8px; margin-bottom: 24px; align-items: center; }
  .search-input { flex: 1; max-width: 500px; }
  .type-toggle { display: flex; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .toggle-btn { padding: 8px 14px; font-size: 0.8rem; background: var(--bg-surface); color: var(--text-secondary); transition: all var(--transition); }
  .toggle-btn.active { background: var(--accent); color: var(--bg-primary); }
  .search-btn { padding: 8px 20px; background: var(--accent); color: var(--bg-primary); border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; transition: background var(--transition); }
  .search-btn:hover { background: var(--accent-hover); }
  .results-section { margin-bottom: 28px; }
  .results-section h2 { margin-bottom: 12px; }
  .results-list { display: flex; flex-direction: column; gap: 10px; }
  .result-card { display: block; padding: 14px 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); transition: all var(--transition); }
  .result-card:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .result-header { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
  .hadith-num { color: var(--text-muted); font-size: 0.8rem; }
  .score { margin-left: auto; color: var(--success); font-size: 0.8rem; }
  .narrator { color: var(--accent); font-size: 0.85rem; margin-bottom: 4px; }
  .text { color: var(--text-secondary); font-size: 0.85rem; line-height: 1.5; }
  .narrator-name { font-weight: 600; font-size: 0.95rem; }
  .name-ar { color: var(--text-secondary); font-size: 0.95rem; }
  .hadith-count { color: var(--text-muted); font-size: 0.8rem; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
