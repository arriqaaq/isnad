<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { fetchAllNotes, fetchNoteTags, deleteNote, exportNotes } from '$lib/api';
  import NoteCard from '$lib/components/notes/NoteCard.svelte';
  import NoteModal from '$lib/components/notes/NoteModal.svelte';

  const COLORS = ['yellow', 'green', 'blue', 'pink', 'purple'] as const;

  let notes: UserNote[] = $state([]);
  let allTags: string[] = $state([]);
  let showNewNote = $state(false);
  let loading = $state(true);
  let searchQuery = $state('');
  let activeColor = $state<string | null>(null);
  let activeTag = $state<string | null>(null);
  let page = $state(1);
  let hasMore = $state(false);

  // Load all tags once on mount
  $effect(() => {
    fetchNoteTags().then(t => { allTags = t; }).catch(() => {});
  });

  // Re-fetch notes when filters change
  $effect(() => {
    const _color = activeColor;
    const _tag = activeTag;
    const _q = searchQuery;
    const _page = page;

    loading = true;
    const params: Record<string, string | number> = { page: _page, limit: 20 };
    if (_color) params.color = _color;
    if (_tag) params.tag = _tag;
    if (_q.trim()) params.q = _q.trim();

    fetchAllNotes(params as any)
      .then(res => {
        if (_page === 1) {
          notes = res.data;
        } else {
          notes = [...notes, ...res.data];
        }
        hasMore = res.has_more;
      })
      .catch(() => { if (_page === 1) notes = []; })
      .finally(() => { loading = false; });
  });

  function setColor(color: string | null) {
    activeColor = activeColor === color ? null : color;
    page = 1;
  }

  function setTag(tag: string | null) {
    activeTag = activeTag === tag ? null : tag;
    page = 1;
  }

  async function handleDelete(note: UserNote) {
    await deleteNote(note.id);
    notes = notes.filter(n => n.id !== note.id);
  }

  function handleNoteSaved() {
    showNewNote = false;
    page = 1;
    fetchNoteTags().then(t => { allTags = t; }).catch(() => {});
  }

  async function handleExport() {
    const data = await exportNotes();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `ilm-notes-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="notes-page">
  <div class="page-header">
    <h1>Notes</h1>
    <div class="header-actions">
      <button class="btn-new" onclick={() => { showNewNote = !showNewNote; }}>+ New Note</button>
      <button class="btn-export" onclick={handleExport}>Export</button>
    </div>
  </div>

  {#if showNewNote}
    <NoteModal
      onclose={() => { showNewNote = false; }}
      onsaved={handleNoteSaved}
    />
  {/if}

  <!-- Search -->
  <div class="search-bar">
    <input
      type="text"
      placeholder="Search notes..."
      bind:value={searchQuery}
      oninput={() => { page = 1; }}
    />
  </div>

  <!-- Filters: Tags + Colors -->
  <div class="filters">
    {#if allTags.length > 0}
      <div class="tag-filters">
        {#each allTags as tag}
          <button
            class="tag-chip"
            class:active={activeTag === tag}
            onclick={() => setTag(tag)}
          >
            {tag}
          </button>
        {/each}
      </div>
    {/if}

    <div class="color-filters">
      {#each COLORS as color}
        <button
          class="color-dot"
          class:active={activeColor === color}
          style="background: var(--note-{color})"
          onclick={() => setColor(color)}
          aria-label="Filter by {color}"
        ></button>
      {/each}
    </div>

    {#if activeTag || activeColor}
      <button class="clear-filters" onclick={() => { activeTag = null; activeColor = null; page = 1; }}>
        Clear filters
      </button>
    {/if}
  </div>

  <!-- Notes list -->
  {#if loading}
    <div class="loading">Loading notes...</div>
  {:else if notes.length === 0}
    <div class="empty">
      <div class="empty-icon">&#9998;</div>
      {#if activeTag || activeColor || searchQuery}
        <div class="empty-text">No notes match your filters</div>
      {:else}
        <div class="empty-text">No notes yet</div>
        <div class="empty-hint">Add notes from the Quran or Hadith pages, or create a new study note above</div>
      {/if}
    </div>
  {:else}
    <div class="notes-list">
      {#each notes as note (note.id)}
        <a href="/notes/{note.id}" class="note-link">
          <NoteCard {note} ondelete={handleDelete} />
        </a>
      {/each}
    </div>

    {#if hasMore}
      <button class="load-more" onclick={() => { page++; }}>
        Load more
      </button>
    {/if}
  {/if}
</div>

<style>
  .notes-page {
    padding: 32px;
    max-width: 960px;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 28px;
  }
  .page-header h1 {
    font-family: var(--font-serif);
    font-size: 2rem;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .header-actions {
    display: flex;
    gap: 10px;
  }
  .btn-new {
    padding: 8px 20px;
    font-size: 0.85rem;
    font-weight: 600;
    color: #fff;
    background: var(--accent);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    transition: background var(--transition), box-shadow var(--transition);
    box-shadow: 0 2px 8px var(--accent-muted);
  }
  .btn-new:hover {
    background: var(--accent-hover);
    box-shadow: 0 4px 16px var(--accent-muted);
  }
  .btn-export {
    padding: 8px 16px;
    font-size: 0.8rem;
    color: var(--text-secondary);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: all var(--transition);
  }
  .btn-export:hover {
    border-color: var(--btn-border-hover);
    background: var(--btn-bg-hover);
  }

  /* Search */
  .search-bar { margin-bottom: 20px; }
  .search-bar input {
    width: 100%;
    padding: 12px 20px;
    border: 1px solid transparent;
    border-radius: var(--radius-xl);
    background: var(--note-editor-bg);
    color: var(--text-primary);
    font-size: 0.95rem;
    font-family: var(--font-serif);
    outline: none;
    box-sizing: border-box;
    box-shadow: var(--shadow-card);
    transition: border-color var(--transition), box-shadow var(--transition);
  }
  .search-bar input::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
  .search-bar input:focus {
    border-color: var(--accent-muted);
    box-shadow: var(--shadow-card), 0 0 0 3px var(--accent-muted);
  }

  /* Filters */
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    margin-bottom: 24px;
  }
  .tag-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag-chip {
    padding: 5px 14px;
    font-size: 0.75rem;
    font-weight: 500;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .tag-chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .tag-chip.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  .color-filters {
    display: flex;
    gap: 8px;
  }
  .color-dot {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2.5px solid transparent;
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
    box-shadow: 0 1px 4px rgba(0,0,0,0.1);
  }
  .color-dot:hover {
    transform: scale(1.15);
  }
  .color-dot.active {
    border-color: var(--accent);
    transform: scale(1.2);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }
  .clear-filters {
    font-size: 0.75rem;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color var(--transition);
  }
  .clear-filters:hover { color: var(--accent); }

  /* Notes grid */
  .notes-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 16px;
  }
  .note-link {
    text-decoration: none;
    color: inherit;
    display: block;
  }
  .loading {
    text-align: center;
    padding: 60px;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .empty {
    text-align: center;
    padding: 80px 24px;
    color: var(--text-muted);
    background: var(--note-editor-bg);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-card);
  }
  .empty-icon { font-size: 2.5rem; margin-bottom: 12px; opacity: 0.5; }
  .empty-text {
    font-family: var(--font-serif);
    font-size: 1.15rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 6px;
  }
  .empty-hint {
    font-family: var(--font-serif);
    font-size: 0.9rem;
    font-style: italic;
    line-height: 1.6;
  }
  .load-more {
    display: block;
    margin: 24px auto;
    padding: 10px 32px;
    font-size: 0.85rem;
    font-family: var(--font-serif);
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius-xl);
    cursor: pointer;
    transition: all var(--transition);
  }
  .load-more:hover {
    background: var(--accent-muted);
  }

  @media (max-width: 768px) {
    .notes-page { padding: 20px; }
    .notes-list {
      grid-template-columns: 1fr;
    }
  }
</style>
