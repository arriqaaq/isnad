<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { fetchAllNotes, addRefToNote, createNote } from '$lib/api';
  import { onMount, onDestroy } from 'svelte';

  let { refType, refId, refLabel, anchorX = 0, anchorY = 0, onclose }: {
    refType: 'ayah' | 'hadith';
    refId: string;
    refLabel: string;
    anchorX?: number;
    anchorY?: number;
    onclose: () => void;
  } = $props();

  let recentNotes: UserNote[] = $state([]);
  let loading = $state(true);
  let showNewNote = $state(false);
  let newTitle = $state('');
  let searchQuery = $state('');
  let flash = $state('');
  let timers: ReturnType<typeof setTimeout>[] = [];

  onDestroy(() => { timers.forEach(clearTimeout); });

  function safeTimeout(fn: () => void, ms: number) {
    const t = setTimeout(fn, ms);
    timers.push(t);
    return t;
  }

  onMount(() => {
    fetchAllNotes({ ref_type: 'topic', limit: 5 })
      .then(res => { recentNotes = res.data; })
      .catch(() => {})
      .finally(() => { loading = false; });
  });

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.save-popover-inner')) {
      onclose();
    }
  }

  async function handleSelectNote(note: UserNote) {
    try {
      await addRefToNote(note.id, { ref_type: refType, ref_id: refId });
      flash = `Added to "${note.title}"`;
      safeTimeout(() => { onclose(); }, 800);
    } catch {
      flash = 'Failed to add';
      safeTimeout(() => { flash = ''; }, 1500);
    }
  }

  async function handleCreateNew() {
    if (!newTitle.trim()) return;
    try {
      const note = await createNote({
        ref_type: 'topic',
        title: newTitle.trim(),
        content: '',
        color: 'yellow',
        refs: [{ ref_type: refType, ref_id: refId }],
      });
      flash = `Created "${note.title}"`;
      safeTimeout(() => { onclose(); }, 800);
    } catch {
      flash = 'Failed to create';
      safeTimeout(() => { flash = ''; }, 1500);
    }
  }

  let filteredNotes = $derived(
    searchQuery
      ? recentNotes.filter(n =>
          n.title?.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : recentNotes
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="save-popover-backdrop" onclick={handleClickOutside}>
  <div class="save-popover-inner" style="top: {anchorY}px; left: {anchorX}px;">
    {#if flash}
      <div class="flash">{flash}</div>
    {:else}
      <div class="popover-header">Save {refLabel}</div>

      {#if showNewNote}
        <div class="new-note-form">
          <input
            type="text"
            bind:value={newTitle}
            placeholder="Note title..."
            onkeydown={(e) => { if (e.key === 'Enter') handleCreateNew(); }}
          />
          <div class="new-note-actions">
            <button class="btn-create" onclick={handleCreateNew}>Create</button>
            <button class="btn-back" onclick={() => { showNewNote = false; }}>Back</button>
          </div>
        </div>
      {:else}
        <button class="new-note-btn" onclick={() => { showNewNote = true; }}>
          + New Note
        </button>

        {#if recentNotes.length > 0}
          <div class="divider"></div>
          <div class="section-label">Recent Notes</div>
          {#if recentNotes.length > 3}
            <input
              type="text"
              class="search-input"
              placeholder="Search notes..."
              bind:value={searchQuery}
            />
          {/if}
          <div class="notes-list">
            {#each filteredNotes as note (note.id)}
              <button class="note-row" onclick={() => handleSelectNote(note)}>
                <span class="note-color-dot" style="background: var(--note-{note.color})"></span>
                <span class="note-row-title">{note.title ?? 'Untitled'}</span>
                <span class="note-row-count">{note.refs.length}</span>
              </button>
            {/each}
          </div>
        {/if}

        {#if loading}
          <div class="loading-text">Loading...</div>
        {/if}
      {/if}
    {/if}
  </div>
</div>

<style>
  .save-popover-backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
  }
  .save-popover-inner {
    position: fixed;
    min-width: 280px;
    max-width: 320px;
    max-height: 400px;
    overflow-y: auto;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
    padding: 8px 0;
    animation: popIn 0.12s ease-out;
  }
  .popover-header {
    padding: 6px 14px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .new-note-btn {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 14px;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }
  .new-note-btn:hover { background: var(--bg-hover); }
  .new-note-form {
    padding: 8px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .new-note-form input {
    width: 100%;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.85rem;
    outline: none;
    box-sizing: border-box;
  }
  .new-note-form input:focus { border-color: var(--accent); }
  .new-note-actions {
    display: flex;
    gap: 6px;
  }
  .btn-create {
    padding: 4px 12px;
    font-size: 0.8rem;
    font-weight: 600;
    color: #fff;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .btn-back {
    padding: 4px 12px;
    font-size: 0.8rem;
    color: var(--text-muted);
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .divider {
    height: 1px;
    background: var(--border);
    margin: 4px 14px;
  }
  .section-label {
    padding: 4px 14px;
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .search-input {
    margin: 0 14px 4px;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.75rem;
    outline: none;
    width: calc(100% - 28px);
    box-sizing: border-box;
  }
  .notes-list {
    display: flex;
    flex-direction: column;
  }
  .note-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border: none;
    background: none;
    cursor: pointer;
    text-align: left;
    width: 100%;
    font-size: 0.85rem;
    color: var(--text-primary);
  }
  .note-row:hover { background: var(--bg-hover); }
  .note-color-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .note-row-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note-row-count {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .loading-text {
    padding: 8px 14px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .flash {
    padding: 14px;
    text-align: center;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--success);
  }
  @keyframes popIn {
    from { transform: scale(0.95); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
</style>
