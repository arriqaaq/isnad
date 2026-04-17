<script lang="ts">
  import { onMount } from 'svelte';
  import { getCollections } from '$lib/api';
  import type { ApiCollection } from '$lib/types';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let books: ApiCollection[] = $state([]);
  let loading = $state(true);

  onMount(async () => {
    try { books = await getCollections(); }
    catch (e) { console.error('Failed to load books:', e); }
    finally { loading = false; }
  });
</script>

<div class="books-page">
  <div class="page-header">
    <h1>Books</h1>
    <p class="page-subtitle">{books.length} collections across the major hadith sources</p>
  </div>
  {#if loading}
    <LoadingSpinner />
  {:else}
    <div class="books-grid">
      {#each books as book (book.id)}
        <a href="/hadiths?book={book.collection_id}" class="book-card">
          <div class="book-number">{book.collection_id}</div>
          <h3 class="book-title arabic" dir="rtl">{book.name_ar || book.name_en}</h3>
          {#if book.name_ar && book.name_en}
            <span class="book-en">{book.name_en}</span>
          {/if}
        </a>
      {/each}
    </div>
    {#if books.length === 0}
      <div class="empty">No books found.</div>
    {/if}
  {/if}
</div>

<style>
  .books-page {
    padding: 32px;
    max-width: 1100px;
    margin: 0 auto;
  }
  .page-header {
    margin-bottom: 32px;
  }
  .page-header h1 {
    font-size: 1.6rem;
    font-weight: 700;
    margin-bottom: 4px;
  }
  .page-subtitle {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .books-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 16px;
  }

  .book-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
    padding: 28px 20px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    color: var(--text-primary);
    text-decoration: none;
    transition: all 0.25s ease;
    position: relative;
    overflow: hidden;
  }
  .book-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--accent);
    opacity: 0;
    transition: opacity 0.25s ease;
  }
  .book-card:hover {
    border-color: var(--accent);
    box-shadow: 0 8px 32px rgba(214,51,132,0.08);
    transform: translateY(-3px);
    color: var(--text-primary);
  }
  .book-card:hover::before {
    opacity: 1;
  }

  .book-number {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 50%;
    font-weight: 700;
    font-size: 0.85rem;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .book-title {
    font-size: 1.1rem;
    font-weight: 600;
    line-height: 1.8;
    color: var(--text-primary);
  }

  .book-en {
    font-size: 0.78rem;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: 60px;
    font-size: 0.9rem;
  }

  @media (max-width: 600px) {
    .books-page { padding: 20px; }
    .books-grid { grid-template-columns: 1fr 1fr; gap: 12px; }
    .book-card { padding: 20px 16px; }
  }
</style>
