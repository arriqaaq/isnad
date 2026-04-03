<script lang="ts">
  import { getManuscripts } from '$lib/api';
  import type { ApiManuscript, PaginatedResponse } from '$lib/types';
  import ManuscriptCard from '$lib/components/quran/ManuscriptCard.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: PaginatedResponse<ApiManuscript> | null = $state(null);
  let loading = $state(true);
  let currentPage = $state(1);

  async function load(page: number) {
    loading = true;
    try {
      data = await getManuscripts({ page, limit: 20 });
      currentPage = page;
    } catch (e) {
      console.error('Failed to load manuscripts:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load(1);
  });
</script>

<div class="manuscripts-page">
  <h1 class="page-title">Quran Manuscripts</h1>
  <p class="page-desc">
    Manuscript descriptions from the Corpus Coranicum project (TEI XML).
  </p>

  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="manuscript-list">
      {#each data.data as manuscript}
        <ManuscriptCard {manuscript} />
      {/each}
    </div>

    {#if data.data.length === 0}
      <div class="empty">No manuscripts found. Run <code>hadith ingest-manuscripts</code> to import data.</div>
    {/if}

    <Pagination page={currentPage} hasMore={data.has_more} onPageChange={load} />
  {/if}
</div>

<style>
  .manuscripts-page {
    padding: 24px;
    max-width: 800px;
    margin: 0 auto;
  }
  .page-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 8px;
  }
  .page-desc {
    font-size: 0.85rem;
    color: var(--text-muted);
    margin-bottom: 24px;
  }
  .manuscript-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: 40px 0;
  }
  .empty code {
    font-family: var(--font-mono);
    background: var(--bg-hover);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }
  @media (max-width: 640px) {
    .manuscripts-page { padding: 12px; }
  }
</style>
