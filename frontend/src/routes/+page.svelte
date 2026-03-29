<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats, getBooks } from '$lib/api';
  import type { StatsResponse, ApiBook } from '$lib/types';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let stats: StatsResponse | null = $state(null);
  let books: ApiBook[] = $state([]);
  let loading = $state(true);

  onMount(async () => {
    try {
      [stats, books] = await Promise.all([getStats(), getBooks()]);
    } catch (e) {
      console.error('Failed to load dashboard:', e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="dashboard">
  <h1>Dashboard</h1>

  {#if loading}
    <LoadingSpinner />
  {:else}
    <div class="stats-grid">
      <div class="stat-card">
        <span class="stat-value mono">{stats?.hadith_count.toLocaleString() ?? 0}</span>
        <span class="stat-label">Hadiths</span>
      </div>
      <div class="stat-card">
        <span class="stat-value mono">{stats?.narrator_count.toLocaleString() ?? 0}</span>
        <span class="stat-label">Narrators</span>
      </div>
      <div class="stat-card">
        <span class="stat-value mono">{stats?.book_count.toLocaleString() ?? 0}</span>
        <span class="stat-label">Books</span>
      </div>
    </div>

    {#if books.length > 0}
      <section class="books-section">
        <h2>Browse by Book</h2>
        <div class="books-grid">
          {#each books as book}
            <a href="/hadiths?book={book.book_number}" class="book-tile">
              <span class="book-num mono">{book.book_number}</span>
              <span class="book-name">{book.name_en}</span>
              {#if book.name_ar}
                <span class="book-ar arabic" dir="rtl">{book.name_ar}</span>
              {/if}
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .dashboard { padding: 24px; }
  h1 { margin-bottom: 24px; }
  .stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; margin-bottom: 32px; }
  .stat-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 24px; display: flex; flex-direction: column; gap: 4px; }
  .stat-value { font-size: 2rem; font-weight: 700; color: var(--accent); }
  .stat-label { font-size: 0.85rem; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; }
  .books-section { margin-top: 8px; }
  .books-section h2 { margin-bottom: 16px; }
  .books-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 12px; }
  .book-tile { display: flex; flex-direction: column; gap: 4px; padding: 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); transition: all var(--transition); }
  .book-tile:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .book-num { font-size: 0.75rem; color: var(--text-muted); }
  .book-name { font-weight: 500; font-size: 0.9rem; }
  .book-ar { font-size: 0.9rem; color: var(--text-secondary); }
</style>
