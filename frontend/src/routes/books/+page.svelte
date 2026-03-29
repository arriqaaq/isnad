<script lang="ts">
  import { onMount } from 'svelte';
  import { getBooks } from '$lib/api';
  import type { ApiBook } from '$lib/types';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let books: ApiBook[] = $state([]);
  let loading = $state(true);

  onMount(async () => {
    try { books = await getBooks(); }
    catch (e) { console.error('Failed to load books:', e); }
    finally { loading = false; }
  });
</script>

<div class="books-page">
  <h1>Books</h1>
  {#if loading}
    <LoadingSpinner />
  {:else}
    <div class="books-grid">
      {#each books as book (book.id)}
        <a href="/hadiths?book={book.book_number}" class="book-card">
          <span class="book-num mono">Book {book.book_number}</span>
          <h3 class="book-name">{book.name_en}</h3>
          {#if book.name_ar}
            <p class="book-ar arabic" dir="rtl">{book.name_ar}</p>
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
  .books-page { padding: 24px; }
  h1 { margin-bottom: 20px; }
  .books-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
  .book-card { display: flex; flex-direction: column; gap: 6px; padding: 20px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); transition: all var(--transition); }
  .book-card:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .book-num { font-size: 0.75rem; color: var(--text-muted); }
  .book-name { font-size: 0.95rem; }
  .book-ar { color: var(--text-secondary); font-size: 0.95rem; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
