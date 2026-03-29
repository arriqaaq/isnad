<script lang="ts">
  import { getFamilies } from '$lib/api';
  import type { ApiHadithFamily, PaginatedResponse } from '$lib/types';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: PaginatedResponse<ApiHadithFamily> | null = $state(null);
  let loading = $state(true);
  let currentPage = $state(1);

  async function load() {
    loading = true;
    try {
      data = await getFamilies({ page: currentPage, limit: 20 });
    } catch (e) {
      console.error('Failed to load families:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { load(); });

  function onPageChange(page: number) {
    currentPage = page;
    load();
  }
</script>

<div class="families-page">
  <h1>Hadith Families</h1>
  <p class="subtitle">Groups of hadith variants sharing the same report across different chains</p>

  {#if loading}
    <LoadingSpinner />
  {:else if data && data.data.length > 0}
    <div class="family-grid">
      {#each data.data as family (family.id)}
        <a href="/families/{family.id}" class="family-card">
          <div class="family-label">{family.family_label ?? 'Unnamed Family'}</div>
          <div class="family-meta">
            <span class="variant-count">{family.variant_count ?? 0} variants</span>
          </div>
        </a>
      {/each}
    </div>
    <Pagination page={currentPage} hasMore={data.has_more} {onPageChange} />
  {:else}
    <div class="empty">
      <p>No hadith families computed yet.</p>
      <p class="hint">Run <code>hadith analyze --families</code> to cluster hadiths into families.</p>
    </div>
  {/if}
</div>

<style>
  .families-page { padding: 24px; max-width: 1000px; }
  .subtitle { color: var(--text-muted); font-size: 0.9rem; margin-bottom: 24px; }
  .family-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; margin-bottom: 20px; }
  .family-card {
    padding: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: all var(--transition);
    cursor: pointer;
  }
  .family-card:hover { border-color: var(--accent); }
  .family-label { font-size: 0.95rem; font-weight: 500; margin-bottom: 8px; color: var(--text-primary); }
  .family-meta { display: flex; gap: 8px; }
  .variant-count { font-size: 0.8rem; color: var(--accent); background: var(--accent-muted); padding: 2px 8px; border-radius: 12px; }
  .empty { text-align: center; color: var(--text-muted); padding: 60px 20px; }
  .hint { font-size: 0.85rem; margin-top: 8px; }
  code { background: var(--bg-surface); padding: 2px 6px; border-radius: 4px; font-size: 0.85rem; }
</style>
