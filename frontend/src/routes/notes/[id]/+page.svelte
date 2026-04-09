<script lang="ts">
  import { page } from '$app/state';
  import type { UserNote } from '$lib/types';
  import { fetchNote, updateNote, updateRefAnnotation, removeRefFromNote } from '$lib/api';
  import NoteEditor from '$lib/components/notes/NoteEditor.svelte';
  import RefCard from '$lib/components/notes/RefCard.svelte';
  import TagInput from '$lib/components/notes/TagInput.svelte';
  import Badge from '$lib/components/common/Badge.svelte';

  let note: UserNote | null = $state(null);
  let loading = $state(true);
  let editingTitle = $state(false);
  let titleInput = $state('');

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true;
    fetchNote(id)
      .then(n => {
        note = n;
        titleInput = n.title ?? '';
      })
      .catch(() => { note = null; })
      .finally(() => { loading = false; });
  });

  async function handleTitleSave() {
    if (!note) return;
    const updated = await updateNote(note.id, { title: titleInput.trim() });
    note = updated;
    editingTitle = false;
  }

  async function handleContentSave(data: { content: string; color: string; tags: string[] }) {
    if (!note) return;
    const updated = await updateNote(note.id, {
      content: data.content,
      color: data.color,
      tags: data.tags,
    });
    note = updated;
  }

  async function handleTagsChange(tags: string[]) {
    if (!note) return;
    const updated = await updateNote(note.id, { tags });
    note = updated;
  }

  async function handleRefAnnotation(idx: number, annotation: string) {
    if (!note) return;
    const updated = await updateRefAnnotation(note.id, idx, annotation);
    note = updated;
  }

  async function handleRemoveRef(idx: number) {
    if (!note || !note.refs[idx]) return;
    const updated = await removeRefFromNote(note.id, note.refs[idx]);
    note = updated;
  }

  let sourceLabel = $derived.by(() => {
    if (!note) return '';
    const hasAyah = note.refs.some(r => r.ref_type === 'ayah');
    const hasHadith = note.refs.some(r => r.ref_type === 'hadith');
    if (hasAyah && hasHadith) return 'Quran + Hadith';
    if (hasAyah) return 'Quran';
    if (hasHadith) return 'Hadith';
    return 'Topic';
  });
</script>

<div class="note-detail">
  {#if loading}
    <div class="loading">Loading note...</div>
  {:else if !note}
    <div class="not-found">Note not found</div>
  {:else}
    <!-- Title -->
    <div class="title-area">
      {#if editingTitle}
        <input
          class="title-input"
          bind:value={titleInput}
          onblur={handleTitleSave}
          onkeydown={(e) => { if (e.key === 'Enter') handleTitleSave(); }}
        />
      {:else}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <h1 class="title" onclick={() => { editingTitle = true; }}>
          {note.title ?? 'Untitled Note'}
          <span class="edit-hint">&#9998;</span>
        </h1>
      {/if}
      <div class="title-meta">
        <Badge text={sourceLabel} variant="default" />
        <span class="ref-count">{note.refs.length} references</span>
      </div>
    </div>

    <!-- Tags -->
    <div class="tags-area">
      <TagInput tags={note.tags} onchange={handleTagsChange} />
    </div>

    <!-- Overall Notes -->
    <section class="section">
      <h2 class="section-label">Overall Notes</h2>
      <NoteEditor
        note={note}
        onsave={handleContentSave}
      />
    </section>

    <!-- Collected References -->
    {#if note.refs.length > 0}
      <section class="section">
        <h2 class="section-label">Collected References ({note.refs.length})</h2>
        <div class="refs-list">
          {#each note.refs as ref, idx}
            <RefCard
              {ref}
              onupdateannotation={(ann) => handleRefAnnotation(idx, ann)}
              onremove={() => handleRemoveRef(idx)}
            />
          {/each}
        </div>
      </section>
    {/if}

    <a href="/notes" class="back-link">&larr; All Notes</a>
  {/if}
</div>

<style>
  .note-detail {
    padding: 32px;
    max-width: 800px;
  }
  .title-area {
    margin-bottom: 20px;
  }
  .title {
    font-family: var(--font-serif);
    font-size: 2rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 10px;
    letter-spacing: -0.01em;
    line-height: 1.3;
  }
  .edit-hint {
    font-size: 0.8rem;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition);
  }
  .title:hover .edit-hint { opacity: 1; }
  .title-input {
    font-family: var(--font-serif);
    font-size: 2rem;
    font-weight: 600;
    border: none;
    border-bottom: 2px solid var(--accent);
    background: transparent;
    color: var(--text-primary);
    width: 100%;
    outline: none;
    padding: 4px 0;
    letter-spacing: -0.01em;
  }
  .title-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
  .ref-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .tags-area {
    margin-bottom: 24px;
  }
  .section {
    margin-bottom: 32px;
  }
  .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    margin-bottom: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .section-label::before {
    content: '';
    display: inline-block;
    width: 3px;
    height: 14px;
    background: var(--accent);
    border-radius: 2px;
  }
  .refs-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .loading, .not-found {
    text-align: center;
    padding: 60px;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 24px;
    padding: 6px 14px;
    font-size: 0.85rem;
    font-family: var(--font-serif);
    color: var(--accent);
    text-decoration: none;
    border: 1px solid var(--accent-muted);
    border-radius: var(--radius);
    transition: all var(--transition);
  }
  .back-link:hover {
    background: var(--accent-muted);
    color: var(--accent-hover);
  }
</style>
