<script lang="ts">
  import { page } from '$app/state';
  import { getNarrators } from '$lib/api';
  import type { ApiNarratorWithCount, PaginatedResponse } from '$lib/types';
  import NarratorCard from '$lib/components/narrator/NarratorCard.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let result: PaginatedResponse<ApiNarratorWithCount> | null = $state(null);
  let loading = $state(true);
  let searchQuery = $state('');
  let selectedGeneration = $state('');

  const generations = [
    { value: '', label: 'All' },
    { value: '1', label: '1 - Sahaba' },
    { value: '2', label: '2 - Tabi\'in' },
    { value: '3', label: '3 - Tabi\' al-Tabi\'in' },
    { value: '4', label: '4th' },
    { value: '5', label: '5th' },
    { value: '6', label: '6th' },
    { value: '7', label: '7th' },
    { value: '8', label: '8th' },
    { value: '9', label: '9th' },
    { value: '10', label: '10th' },
    { value: '11', label: '11th' },
    { value: '12', label: '12th' },
  ];

  let currentPage = $derived(Number(page.url.searchParams.get('page')) || 1);

  async function load() {
    loading = true;
    try {
      result = await getNarrators({
        q: searchQuery || undefined,
        page: currentPage,
        generation: selectedGeneration || undefined,
      });
    } catch (e) {
      console.error('Failed to load narrators:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void currentPage;
    load();
  });

  function handleSearch(e: Event) {
    e.preventDefault();
    load();
  }

  function selectGeneration(gen: string) {
    selectedGeneration = gen;
    changePage(1);
    load();
  }

  function changePage(newPage: number) {
    const sp = new URLSearchParams();
    sp.set('page', String(newPage));
    if (searchQuery) sp.set('q', searchQuery);
    if (selectedGeneration) sp.set('generation', selectedGeneration);
    window.history.pushState({}, '', `/narrators?${sp}`);
  }
</script>

<div class="narrator-list">
  <h1>Narrators</h1>

  <form class="search-bar" onsubmit={handleSearch}>
    <input type="text" placeholder="Search narrators..." bind:value={searchQuery} />
    <button type="submit" class="search-btn">Search</button>
  </form>

  <div class="generation-tabs">
    {#each generations as gen}
      <button
        class="gen-tab"
        class:active={selectedGeneration === gen.value}
        onclick={() => selectGeneration(gen.value)}
      >
        {gen.label}
      </button>
    {/each}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if result && result.data.length > 0}
    <div class="grid">
      {#each result.data as narrator (narrator.id)}
        <NarratorCard {narrator} />
      {/each}
    </div>
    <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
  {:else}
    <div class="empty">No narrators found.</div>
  {/if}
</div>

<style>
  .narrator-list { padding: 24px; }
  h1 { margin-bottom: 16px; }
  .search-bar { display: flex; gap: 8px; margin-bottom: 16px; }
  .search-bar input { flex: 1; max-width: 400px; }
  .search-btn { padding: 8px 20px; background: var(--accent); color: var(--bg-primary); border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; transition: background var(--transition); }
  .search-btn:hover { background: var(--accent-hover); }

  .generation-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 20px;
  }
  .gen-tab {
    font-size: 0.8rem;
    padding: 5px 12px;
    border-radius: var(--radius);
    color: var(--btn-text);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    cursor: pointer;
    transition: all var(--transition);
  }
  .gen-tab:hover {
    background: var(--btn-bg-hover);
    border-color: var(--btn-border-hover);
  }
  .gen-tab.active {
    background: var(--accent);
    color: var(--bg-primary);
    border-color: var(--accent);
  }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
