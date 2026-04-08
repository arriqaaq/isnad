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
    padding: 24px;
    max-width: 800px;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }
  .header-actions {
    display: flex;
    gap: 8px;
  }
  .btn-new {
    padding: 6px 16px;
    font-size: 0.85rem;
    font-weight: 600;
    color: #fff;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .btn-export {
    padding: 6px 12px;
    font-size: 0.8rem;
    color: var(--text-secondary);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }


  /* Search */
  .search-bar { margin-bottom: 16px; }
  .search-bar input {
    width: 100%;
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
    box-sizing: border-box;
  }
  .search-bar input:focus { border-color: var(--accent); }

  /* Filters */
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin-bottom: 20px;
  }
  .tag-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tag-chip {
    padding: 4px 12px;
    font-size: 0.75rem;
    font-weight: 500;
    border: 1px solid var(--border);
    border-radius: 12px;
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
    gap: 6px;
  }
  .color-dot {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
  }
  .color-dot.active {
    border-color: var(--text-primary);
    transform: scale(1.15);
  }
  .clear-filters {
    font-size: 0.7rem;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    text-decoration: underline;
  }
  .clear-filters:hover { color: var(--accent); }

  /* Notes list */
  .notes-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .note-link {
    text-decoration: none;
    color: inherit;
    display: block;
  }
  .loading {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }
  .empty {
    text-align: center;
    padding: 60px 20px;
    color: var(--text-muted);
  }
  .empty-icon { font-size: 2rem; margin-bottom: 8px; }
  .empty-text { font-size: 1rem; font-weight: 600; margin-bottom: 4px; }
  .empty-hint { font-size: 0.85rem; }
  .load-more {
    display: block;
    margin: 16px auto;
    padding: 8px 24px;
    font-size: 0.85rem;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    cursor: pointer;
  }
</style>
