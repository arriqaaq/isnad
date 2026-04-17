<script lang="ts">
  import type { Book } from '$lib/types';

  let { book, currentPage, totalPages, onToggleSidebar }: {
    book: Book;
    currentPage: number;
    totalPages: number;
    onToggleSidebar: () => void;
  } = $props();
</script>

<header class="reader-header">
  <div class="header-left">
    <a href="/quran" class="back-btn" aria-label="Back">&larr;</a>
    <div class="book-info">
      <h1 class="book-title arabic" dir="rtl">{book.name_ar}</h1>
      <span class="book-author">{book.author_ar}</span>
    </div>
  </div>
  <div class="header-right">
    <span class="page-indicator">{currentPage + 1} / {totalPages}</span>
    <button class="sidebar-toggle" onclick={onToggleSidebar} aria-label="Toggle chapters">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>
      </svg>
    </button>
  </div>
</header>

<style>
  .reader-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-primary);
    flex-shrink: 0;
    gap: 12px;
    min-height: 52px;
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .back-btn {
    font-size: 1.2rem;
    color: var(--text-muted);
    text-decoration: none;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    transition: all var(--transition);
    flex-shrink: 0;
  }
  .back-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .book-info {
    min-width: 0;
    overflow: hidden;
  }
  .book-title {
    font-size: 1.05rem;
    font-weight: 700;
    line-height: 1.6;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .book-author {
    font-size: 0.75rem;
    color: var(--text-muted);
    direction: rtl;
  }
  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .page-indicator {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .sidebar-toggle {
    display: none;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .sidebar-toggle:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  @media (max-width: 768px) {
    .sidebar-toggle { display: flex; }
    .reader-header { padding: 8px 12px; }
    .book-title { font-size: 0.95rem; }
  }
</style>
