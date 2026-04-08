<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { parseContent } from '$lib/editor';
  import Badge from '$lib/components/common/Badge.svelte';
  import MentionChip from './MentionChip.svelte';
  import EmbeddedRef from './EmbeddedRef.svelte';
  import LinkPreviewCard from './LinkPreviewCard.svelte';

  let { note, onedit, ondelete }: {
    note: UserNote;
    onedit?: (note: UserNote) => void;
    ondelete?: (note: UserNote) => void;
  } = $props();

  let sourceLabel = $derived.by(() => {
    if (note.ref_type === 'topic') {
      const hasAyah = note.refs.some(r => r.ref_type === 'ayah');
      const hasHadith = note.refs.some(r => r.ref_type === 'hadith');
      if (hasAyah && hasHadith) return 'Quran + Hadith';
      if (hasAyah) return 'Quran';
      if (hasHadith) return 'Hadith';
      return 'Topic';
    }
    return note.ref_type === 'ayah' ? 'Quran' : 'Hadith';
  });

  let refLabel = $derived(
    note.ref_type === 'ayah' ? `Quran ${note.ref_id}`
      : note.ref_type === 'hadith' ? `Hadith ${note.ref_id}`
      : null
  );

  let timeAgo = $derived.by(() => {
    const d = new Date(note.updated_at);
    const now = Date.now();
    const diff = now - d.getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    if (days < 30) return `${days}d ago`;
    return d.toLocaleDateString();
  });

  let contentParts = $derived(parseContent(note.content));
</script>

<div class="note-card" style="border-left-color: var(--note-{note.color})">
  <div class="note-header">
    <div class="note-meta">
      {#if note.title}
        <span class="note-title">{note.title}</span>
      {:else if refLabel}
        <a href={note.ref_type === 'ayah' ? `/quran/${note.ref_id?.split(':')[0]}` : `/hadiths/${note.ref_id}`} class="note-ref">
          {refLabel}
        </a>
      {/if}
      <Badge text={sourceLabel} variant="default" />
      {#if note.ref_type === 'topic' && note.refs.length > 0}
        <span class="ref-count">{note.refs.length} refs</span>
      {/if}
    </div>
    <div class="note-actions">
      <span class="note-time">{timeAgo}</span>
      {#if onedit}
        <button class="action-btn" onclick={() => onedit?.(note)} aria-label="Edit">&#9998;</button>
      {/if}
      {#if ondelete}
        <button class="action-btn delete-btn" onclick={() => ondelete?.(note)} aria-label="Delete">&times;</button>
      {/if}
    </div>
  </div>

  {#if note.content}
    <div class="note-content">
      {#each contentParts as part}
        {#if part.type === 'text'}
          <span class="text-segment">{part.value}</span>
        {:else if part.type === 'narrator'}
          <MentionChip refType="narrator" refId={part.refId} />
        {:else if part.type === 'ayah'}
          <EmbeddedRef refType="ayah" refId={part.refId} />
        {:else if part.type === 'hadith'}
          <EmbeddedRef refType="hadith" refId={part.refId} />
        {:else if part.type === 'url'}
          <LinkPreviewCard url={part.value} />
        {/if}
      {/each}
    </div>
  {/if}

  {#if note.tags.length > 0}
    <div class="note-tags">
      {#each note.tags as tag}
        <Badge text={tag} variant="default" />
      {/each}
    </div>
  {/if}
</div>

<style>
  .note-card {
    border-left: 4px solid var(--note-yellow);
    padding: 12px 16px;
    background: var(--bg-surface);
    border-radius: 0 var(--radius) var(--radius) 0;
    border: 1px solid var(--border);
    border-left-width: 4px;
    transition: border-color var(--transition);
  }
  .note-card:hover {
    border-color: var(--border);
  }
  .note-card:hover .action-btn {
    opacity: 1;
  }
  .note-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 6px;
  }
  .note-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .note-title {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--text-primary);
  }
  .note-ref {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    color: var(--accent);
    text-decoration: none;
    font-weight: 600;
  }
  .note-ref:hover { text-decoration: underline; }
  .ref-count {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .note-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .note-time {
    font-size: 0.65rem;
    color: var(--text-muted);
  }
  .action-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.9rem;
    padding: 2px 4px;
    opacity: 0;
    transition: opacity var(--transition), color var(--transition);
  }
  .action-btn:hover { color: var(--text-primary); }
  .delete-btn:hover { color: var(--error); }
  .note-content {
    font-size: 0.85rem;
    line-height: 1.6;
    color: var(--text-secondary);
    word-break: break-word;
  }
  .text-segment {
    white-space: pre-wrap;
  }
  .note-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
</style>
