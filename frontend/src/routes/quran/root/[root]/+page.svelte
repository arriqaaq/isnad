<script lang="ts">
  import { page } from '$app/stores';
  import { searchByRoot } from '$lib/api';
  import type { RootSearchResponse } from '$lib/types';

  let data: RootSearchResponse | null = $state(null);
  let loading = $state(true);
  let error: string | null = $state(null);

  let root = $derived(($page.params as Record<string, string>).root);

  $effect(() => {
    if (root) {
      loading = true;
      error = null;
      searchByRoot(root)
        .then(r => { data = r; })
        .catch(e => { error = e.message; })
        .finally(() => { loading = false; });
    }
  });

  // Group occurrences by surah:ayah
  let grouped = $derived.by(() => {
    if (!data) return [];
    const map = new Map<string, typeof data.occurrences>();
    for (const w of data.occurrences) {
      const key = `${w.surah_number}:${w.ayah_number}`;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(w);
    }
    return Array.from(map.entries()).map(([key, words]) => ({ key, words }));
  });
</script>

<svelte:head>
  <title>Root: {root} - Quran</title>
</svelte:head>

<div class="root-page">
  <div class="root-header">
    <a href="/quran/search" class="back-link">Back to Search</a>
    <h1 dir="rtl" class="root-title">{root}</h1>
    {#if data}
      <div class="root-stats">
        {data.occurrences.length} occurrences in {data.ayah_count} ayahs
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if grouped.length === 0}
    <div class="empty">No occurrences found for this root.</div>
  {:else}
    <div class="root-results">
      {#each grouped as { key, words }}
        <div class="root-ayah">
          <a href="/quran/{words[0].surah_number}#{key}" class="ayah-link">
            <span class="ref">{key}</span>
          </a>
          <div class="word-list" dir="rtl">
            {#each words as word}
              <span class="root-word">
                <span class="rw-ar">{word.text_ar}</span>
                {#if word.translation}
                  <span class="rw-en">{word.translation}</span>
                {/if}
                <span class="rw-pos">{word.pos}</span>
              </span>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .root-page {
    max-width: 800px;
    margin: 0 auto;
    padding: 24px;
  }
  .root-header {
    margin-bottom: 24px;
  }
  .back-link {
    font-size: 0.85rem;
    color: var(--accent);
    text-decoration: none;
  }
  .root-title {
    font-size: 3rem;
    color: var(--text-primary);
    margin: 8px 0;
  }
  .root-stats {
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .loading, .error, .empty {
    padding: 40px;
    text-align: center;
    color: var(--text-muted);
  }
  .root-ayah {
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }
  .ayah-link {
    text-decoration: none;
  }
  .ref {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--accent);
    white-space: nowrap;
    min-width: 50px;
  }
  .word-list {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .root-word {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 6px 10px;
    background: var(--bg-hover);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }
  .rw-ar {
    font-size: 1.3rem;
    color: var(--text-primary);
  }
  .rw-en {
    font-size: 0.65rem;
    color: var(--text-muted);
  }
  .rw-pos {
    font-size: 0.55rem;
    color: var(--accent);
    font-family: var(--font-mono);
  }
</style>
