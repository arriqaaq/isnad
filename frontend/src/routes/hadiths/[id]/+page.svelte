<script lang="ts">
  import { page } from '$app/state';
  import { getHadith, getChainGraph } from '$lib/api';
  import type { HadithDetailResponse, GraphData } from '$lib/types';
  import { stripHtml } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import ChainView from '$lib/components/graph/ChainView.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: HadithDetailResponse | null = $state(null);
  let graphData: GraphData | null = $state(null);
  let loading = $state(true);

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true;
    Promise.all([getHadith(id), getChainGraph(id)])
      .then(([d, g]) => { data = d; graphData = g; })
      .catch((e) => console.error('Failed to load hadith:', e))
      .finally(() => { loading = false; });
  });

  /**
   * Highlight the matn (quoted speech) in Arabic text.
   */
  function highlightMatn(text: string): string {
    return text
      .replace(/"([^"]+)"/g, '<span class="matn">"$1"</span>')
      .replace(/«([^»]+)»/g, '<span class="matn">«$1»</span>');
  }
</script>

<div class="hadith-view">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="view-header">
      <h1>Hadith #{data.hadith.hadith_number}</h1>
      <div class="badges">
        {#if data.hadith.book_name}
          <Badge text={data.hadith.book_name} variant="accent" />
        {:else}
          <Badge text="Book {data.hadith.book_id}" />
        {/if}
        {#if data.hadith.grade}
          <Badge text={data.hadith.grade} variant="success" />
        {/if}
      </div>
    </div>

    {#if data.hadith.narrator_text}
      <div class="narrator-text">{data.hadith.narrator_text}</div>
    {/if}

    <div class="text-section">
      {#if $language === 'en'}
        {#if data.hadith.text_en}
          <div class="text-en">
            {stripHtml(data.hadith.text_en)}
          </div>
        {:else if data.hadith.text_ar}
          <div class="text-ar arabic" dir="rtl">
            {@html highlightMatn(data.hadith.text_ar)}
          </div>
        {/if}
      {:else}
        {#if data.hadith.text_ar}
          <div class="text-ar arabic" dir="rtl">
            {@html highlightMatn(data.hadith.text_ar)}
          </div>
        {/if}
      {/if}
    </div>

    {#if data.narrators.length > 0}
      <section class="section">
        <h2>Narrators</h2>
        <div class="chips">
          {#each data.narrators as narrator}
            <NarratorChip {narrator} />
          {/each}
        </div>
      </section>
    {/if}

    <section class="section">
      <h2>Narrator Chain</h2>
      <ChainView data={graphData} />
    </section>

    {#if data.linked_ayahs && data.linked_ayahs.length > 0}
      <section class="section">
        <h2>Referenced Quran Verses</h2>
        <div class="ayah-list">
          {#each data.linked_ayahs as ayah}
            <a href="/quran/{ayah.surah_number}" class="ayah-item">
              <div class="ayah-meta">
                <span class="ayah-ref">{ayah.surah_number}:{ayah.ayah_number}</span>
              </div>
              <div class="ayah-text arabic" dir="rtl">{ayah.text_ar}</div>
              {#if ayah.text_en}
                <div class="ayah-text-en">{ayah.text_en}</div>
              {/if}
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {:else}
    <div class="empty">Hadith not found.</div>
  {/if}
</div>

<style>
  .hadith-view { padding: 24px; max-width: 900px; }
  .view-header { display: flex; align-items: center; gap: 16px; margin-bottom: 16px; flex-wrap: wrap; }
  .badges { display: flex; gap: 8px; }

  .narrator-text {
    color: var(--accent);
    font-size: 0.95rem;
    font-weight: 500;
    margin-bottom: 20px;
    padding: 12px 16px;
    background: var(--accent-muted);
    border-radius: var(--radius);
  }

  .text-section {
    display: flex;
    flex-direction: column;
    gap: 24px;
    margin-bottom: 28px;
  }

  /* English text — matn only, entirely in green bold italic blockquote */
  .text-en {
    font-family: 'Georgia', 'Palatino Linotype', 'Book Antiqua', serif;
    font-size: 1.1rem;
    line-height: 1.9;
    color: var(--success);
    font-weight: 600;
    font-style: italic;
    padding: 20px 24px;
    border-left: 3px solid var(--accent);
    background: var(--bg-hover);
    border-radius: 0 var(--radius) var(--radius) 0;
    letter-spacing: 0.01em;
  }

  /* Arabic text with highlighted matn */
  .text-ar {
    padding: 24px 28px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 1.3em;
    line-height: 2.4;
  }

  .text-ar :global(.matn) {
    color: var(--text-primary);
    font-weight: 600;
    font-size: 1.05em;
  }

  .section { margin-bottom: 24px; }
  .section h2 { margin-bottom: 12px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }

  .ayah-list { display: flex; flex-direction: column; gap: 10px; }
  .ayah-item {
    display: block;
    padding: 12px 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s;
  }
  .ayah-item:hover { border-color: var(--accent); }
  .ayah-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
  .ayah-ref {
    font-size: 0.75rem;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--accent);
  }
  .ayah-text {
    font-size: 1.15rem;
    line-height: 2.2;
    color: var(--text-primary);
  }
  .ayah-text-en {
    font-size: 0.85rem;
    line-height: 1.6;
    color: var(--text-secondary);
    margin-top: 6px;
  }
</style>
