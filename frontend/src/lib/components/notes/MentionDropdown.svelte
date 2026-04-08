<script lang="ts">
  import { getNarrators, getSurah, getHadith } from '$lib/api';
  import type { ApiNarratorWithCount } from '$lib/types';
  import { onDestroy } from 'svelte';

  let { query, position, onselect, onclose }: {
    query: string;
    position: { x: number; y: number };
    onselect: (type: 'ayah' | 'hadith' | 'narrator', refId: string) => void;
    onclose: () => void;
  } = $props();

  let matchType = $state<'ayah' | 'hadith' | 'narrator' | 'hint' | 'none'>('hint');
  let previewText = $state('');
  let previewLoading = $state(false);
  let narratorResults: ApiNarratorWithCount[] = $state([]);
  let narratorLoading = $state(false);
  let selectedIdx = $state(0);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => { if (debounceTimer) clearTimeout(debounceTimer); });

  $effect(() => {
    const q = query;
    previewText = '';
    narratorResults = [];
    selectedIdx = 0;

    if (q.length === 0) {
      matchType = 'hint';
      return;
    }

    // Ayah pattern: 2:255, 1:1, etc
    if (/^\d+:\d+$/.test(q)) {
      matchType = 'ayah';
      previewLoading = true;
      const [s, a] = q.split(':').map(Number);
      getSurah(s).then(res => {
        const ayah = res.ayahs.find((ay: any) => ay.ayah_number === a);
        previewText = ayah ? (ayah.text_en ?? ayah.text_ar ?? '').slice(0, 120) : 'Ayah not found';
      }).catch(() => { previewText = 'Could not load'; })
        .finally(() => { previewLoading = false; });
      return;
    }

    // Hadith pattern: any word containing underscore or starting with letter then _ (actual DB IDs like im_1, sb_123)
    if (/^[a-z]{2,}_\d/.test(q)) {
      matchType = 'hadith';
      previewLoading = true;
      getHadith(q).then(res => {
        const h = res.hadith;
        previewText = (h.text_en ?? h.text_ar ?? '').slice(0, 120);
      }).catch(() => { previewText = 'Could not load hadith'; })
        .finally(() => { previewLoading = false; });
      return;
    }

    // Narrator search (2+ chars, works with Arabic)
    if (q.length >= 2) {
      matchType = 'narrator';
      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(async () => {
        narratorLoading = true;
        try {
          const res = await getNarrators({ q, limit: 5 });
          narratorResults = res.data;
        } catch { /* ignore */ }
        finally { narratorLoading = false; }
      }, 250);
      return;
    }

    matchType = 'none';
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (matchType === 'ayah') {
        onselect('ayah', query);
      } else if (matchType === 'hadith') {
        onselect('hadith', query);
      } else if (matchType === 'narrator' && narratorResults.length > 0) {
        const n = narratorResults[selectedIdx];
        onselect('narrator', n.id);
      }
      return;
    }
    if (e.key === 'ArrowDown' && matchType === 'narrator') {
      e.preventDefault();
      selectedIdx = Math.min(selectedIdx + 1, narratorResults.length - 1);
    }
    if (e.key === 'ArrowUp' && matchType === 'narrator') {
      e.preventDefault();
      selectedIdx = Math.max(selectedIdx - 1, 0);
    }
  }

  // Expose for parent to forward keydown events
  export { handleKeydown };
</script>

<div class="mention-dropdown" style="top: {position.y}px; left: {position.x}px;">
  {#if matchType === 'hint'}
    <div class="mention-hint">
      <div><code>@2:255</code> — Quran ayah</div>
      <div><code>@im_1</code> — Hadith (by ID)</div>
      <div><code>@name</code> — Narrator (Arabic)</div>
      <div>URLs are auto-detected</div>
    </div>
  {:else if matchType === 'ayah'}
    <div class="mention-match">
      <div class="match-header ayah">Quran {query}</div>
      {#if previewLoading}
        <div class="match-preview">Loading...</div>
      {:else}
        <div class="match-preview">{previewText}</div>
      {/if}
      <div class="match-action">Press <kbd>Enter</kbd> to embed</div>
    </div>
  {:else if matchType === 'hadith'}
    <div class="mention-match">
      <div class="match-header hadith">Hadith {query}</div>
      {#if previewLoading}
        <div class="match-preview">Loading...</div>
      {:else}
        <div class="match-preview">{previewText}</div>
      {/if}
      <div class="match-action">Press <kbd>Enter</kbd> to embed</div>
    </div>
  {:else if matchType === 'narrator'}
    {#if narratorLoading}
      <div class="mention-hint">Searching...</div>
    {:else if narratorResults.length > 0}
      {#each narratorResults as narrator, i}
        <button
          class="narrator-item"
          class:selected={i === selectedIdx}
          onmousedown={(e) => { e.preventDefault(); onselect('narrator', narrator.id); }}
          onmouseenter={() => { selectedIdx = i; }}
        >
          <span class="narrator-name">{narrator.name_en}</span>
          {#if narrator.generation}
            <span class="narrator-meta">Gen {narrator.generation}</span>
          {/if}
          <span class="narrator-meta">{narrator.hadith_count}</span>
        </button>
      {/each}
    {:else}
      <div class="mention-hint">No results for "{query}"</div>
    {/if}
  {:else}
    <div class="mention-hint">Keep typing...</div>
  {/if}
</div>

<style>
  .mention-dropdown {
    position: fixed;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 16px rgba(0,0,0,0.18);
    z-index: 9999;
    max-height: 280px;
    min-width: 260px;
    max-width: 400px;
    overflow-y: auto;
  }
  .mention-hint {
    padding: 10px 14px;
    font-size: 0.75rem;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .mention-hint code {
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .mention-match {
    padding: 10px 14px;
  }
  .match-header {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }
  .match-header.ayah { color: var(--accent); }
  .match-header.hadith { color: var(--success); }
  .match-preview {
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .match-action {
    margin-top: 8px;
    font-size: 0.65rem;
    color: var(--text-muted);
  }
  .match-action kbd {
    padding: 1px 5px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.6rem;
  }
  .narrator-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 8px 14px;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--text-primary);
  }
  .narrator-item:hover, .narrator-item.selected {
    background: var(--bg-hover);
  }
  .narrator-name { font-weight: 600; flex: 1; }
  .narrator-meta {
    font-size: 0.65rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
</style>
