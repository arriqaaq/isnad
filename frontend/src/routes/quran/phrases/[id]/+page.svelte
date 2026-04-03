<script lang="ts">
  import { page } from '$app/stores';
  import { getPhraseDetail } from '$lib/api';
  import type { ApiPhraseWithAyahs } from '$lib/types';

  let data: ApiPhraseWithAyahs | null = $state(null);
  let loading = $state(true);
  let error: string | null = $state(null);

  let phraseId = $derived(($page.params as Record<string, string>).id);

  $effect(() => {
    if (phraseId) {
      loading = true;
      error = null;
      getPhraseDetail(phraseId)
        .then(r => { data = r; })
        .catch(e => { error = e.message; })
        .finally(() => { loading = false; });
    }
  });
</script>

<svelte:head>
  <title>Shared Phrase - Quran</title>
</svelte:head>

<div class="phrase-page">
  <div class="phrase-header">
    <a href="/quran" class="back-link">Back to Quran</a>

    {#if data}
      <h1 dir="rtl" class="phrase-title">{data.text_ar}</h1>
      <div class="phrase-stats">
        {data.occurrence} occurrences across {data.ayah_keys.length} ayahs
      </div>
    {:else}
      <h1 class="phrase-title">Shared Phrase</h1>
    {/if}
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if data && data.ayah_keys.length === 0}
    <div class="empty">No ayahs found for this phrase.</div>
  {:else if data}
    <div class="ayah-list">
      {#each data.ayah_keys as key}
        {@const parts = key.split(':')}
        <a href="/quran/{parts[0]}?ayah={parts[1]}" class="ayah-item">
          <span class="ayah-ref">{key}</span>
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .phrase-page {
    max-width: 800px;
    margin: 0 auto;
    padding: 24px;
  }
  .phrase-header {
    margin-bottom: 24px;
  }
  .back-link {
    font-size: 0.85rem;
    color: var(--accent);
    text-decoration: none;
  }
  .phrase-title {
    font-size: 2.5rem;
    color: var(--text-primary);
    margin: 8px 0;
    line-height: 1.6;
  }
  .phrase-stats {
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .loading, .error, .empty {
    padding: 40px;
    text-align: center;
    color: var(--text-muted);
  }
  .ayah-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .ayah-item {
    display: inline-flex;
    align-items: center;
    padding: 8px 16px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    text-decoration: none;
    transition: all var(--transition);
  }
  .ayah-item:hover {
    border-color: var(--accent);
    background: var(--accent-muted);
  }
  .ayah-ref {
    font-family: var(--font-mono);
    font-size: 0.9rem;
    color: var(--accent);
  }
</style>
