<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { fetchNotesForRef, createNote, updateNote, deleteNote } from '$lib/api';
  import NoteCard from './NoteCard.svelte';
  import NoteEditor from './NoteEditor.svelte';
  import { onMount } from 'svelte';

  let { refType, refId, refLabel, onclose, onsaved }: {
    refType?: 'ayah' | 'hadith';
    refId?: string;
    refLabel?: string;
    onclose: () => void;
    onsaved?: () => void;
  } = $props();

  let existingNotes: UserNote[] = $state([]);
  let loadingExisting = $state(false);
  let showCreateForm = $state(!refType); // If no ref, go straight to create
  let editingNote: UserNote | null = $state(null);
  let newTitle = $state('');
  let saving = $state(false);

  onMount(() => {
    if (refType && refId) {
      loadingExisting = true;
      fetchNotesForRef(refType, refId)
        .then(res => { existingNotes = res.data; })
        .catch(() => {})
        .finally(() => { loadingExisting = false; });
    }
  });

  // Pre-populate content with the embedded ref
  let initialContent = $derived(
    refType && refId ? `@${refId}\n` : ''
  );

  async function handleCreate(data: { content: string; color: string; tags: string[] }) {
    if (!newTitle.trim()) return;
    saving = true;
    try {
      const note = await createNote({
        ref_type: refType ?? 'topic',
        ref_id: refId,
        title: newTitle.trim(),
        content: data.content,
        color: data.color,
        tags: data.tags,
      });
      existingNotes = [note, ...existingNotes];
      showCreateForm = false;
      editingNote = null;
      newTitle = '';
      onsaved?.();
    } catch (e) {
      console.error('Failed to create note:', e);
    } finally {
      saving = false;
    }
  }

  async function handleUpdate(data: { content: string; color: string; tags: string[] }) {
    if (!editingNote) return;
    saving = true;
    try {
      const updated = await updateNote(editingNote.id, data);
      existingNotes = existingNotes.map(n => n.id === updated.id ? updated : n);
      editingNote = null;
    } catch (e) {
      console.error('Failed to update note:', e);
    } finally {
      saving = false;
    }
  }

  async function handleDelete(note: UserNote) {
    try {
      await deleteNote(note.id);
      existingNotes = existingNotes.filter(n => n.id !== note.id);
    } catch (e) {
      console.error('Failed to delete note:', e);
    }
  }

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('note-modal-backdrop')) {
      onclose();
    }
  }

  let modalTitle = $derived.by(() => {
    if (editingNote) return 'Edit Note';
    if (showCreateForm) return refLabel ? `New Note on ${refLabel}` : 'New Study Note';
    return refLabel ? `Notes on ${refLabel}` : 'Notes';
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="note-modal-backdrop" onclick={handleBackdrop}>
  <div class="note-modal">
    <div class="modal-header">
      <h2>{modalTitle}</h2>
      <button class="modal-close" onclick={onclose}>&times;</button>
    </div>

    <div class="modal-body">
      {#if editingNote}
        <!-- Edit existing note -->
        <NoteEditor
          note={editingNote}
          startExpanded={true}
          onsave={handleUpdate}
          oncancel={() => { editingNote = null; }}
        />

      {:else if showCreateForm}
        <!-- Create new note -->
        <input
          type="text"
          class="title-input"
          placeholder="Give your note a title..."
          bind:value={newTitle}
        />
        <NoteEditor
          startExpanded={true}
          initialContent={initialContent}
          onsave={handleCreate}
          oncancel={() => {
            if (refType && existingNotes.length > 0) {
              showCreateForm = false;
            } else {
              onclose();
            }
            newTitle = '';
          }}
        />

      {:else}
        <!-- Show existing notes for this ref -->
        {#if loadingExisting}
          <div class="loading">Loading notes...</div>
        {:else if existingNotes.length > 0}
          <div class="existing-notes">
            {#each existingNotes as note (note.id)}
              <NoteCard
                {note}
                onedit={(n) => { editingNote = n; }}
                ondelete={handleDelete}
              />
            {/each}
          </div>
        {:else}
          <div class="no-notes">No notes on this yet</div>
        {/if}

        <button class="btn-create-new" onclick={() => { showCreateForm = true; }}>
          + Create new note{refLabel ? ` on ${refLabel}` : ''}
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .note-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 0.15s ease-out;
  }
  .note-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-2xl);
    box-shadow: 0 16px 64px rgba(0, 0, 0, 0.18);
    width: 90%;
    max-width: 660px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }
  .modal-header h2 {
    font-family: var(--font-serif);
    font-size: 1.15rem;
    font-weight: 600;
  }
  .modal-close {
    background: none;
    border: none;
    font-size: 1.4rem;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    line-height: 1;
    transition: all var(--transition);
  }
  .modal-close:hover { color: var(--text-primary); background: var(--bg-hover); }
  .modal-body {
    padding: 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: 1;
  }
  .title-input {
    width: 100%;
    padding: 12px 16px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-serif);
    font-size: 1.15rem;
    font-weight: 600;
    outline: none;
    box-sizing: border-box;
    transition: border-color var(--transition);
  }
  .title-input::placeholder {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 400;
  }
  .title-input:focus {
    border-color: var(--accent);
  }
  .existing-notes {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .loading, .no-notes {
    text-align: center;
    padding: 24px;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-size: 0.9rem;
    font-style: italic;
  }
  .btn-create-new {
    padding: 12px 20px;
    font-size: 0.9rem;
    font-family: var(--font-serif);
    font-weight: 600;
    color: var(--accent);
    background: none;
    border: 1.5px dashed var(--accent-muted);
    border-radius: var(--radius-xl);
    cursor: pointer;
    transition: all var(--transition);
    text-align: center;
  }
  .btn-create-new:hover {
    background: var(--accent-muted);
    border-color: var(--accent);
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideUp { from { transform: translateY(20px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
</style>
