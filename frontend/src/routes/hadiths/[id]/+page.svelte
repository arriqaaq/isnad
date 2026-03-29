<script lang="ts">
  import { page } from '$app/state';
  import { getHadith, getChainGraph } from '$lib/api';
  import type { HadithDetailResponse, GraphData } from '$lib/types';
  import { stripHtml } from '$lib/utils';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import GraphView from '$lib/components/graph/GraphView.svelte';
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
   * Arabic quotes are typically between " ... " or « ... ».
   * Returns HTML with <span class="matn"> around quoted portions.
   */
  function highlightMatn(text: string): string {
    // Match text between Arabic-style quotes
    return text
      .replace(/"([^"]+)"/g, '<span class="matn">"$1"</span>')
      .replace(/«([^»]+)»/g, '<span class="matn">«$1»</span>');
  }

  /**
   * Highlight the Prophet's speech in English text.
   * English hadith quotes are typically between "..." after narrator intro.
   */
  function highlightEnglish(text: string | null): string {
    if (!text) return '';
    const cleaned = stripHtml(text);
    // Highlight quoted speech on its own line, bold + italic
    return cleaned.replace(/"([^"]+)"/g, '<br/><span class="prophet-speech">"$1"</span>');
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
      {#if data.hadith.text_en}
        <div class="text-en">
          {@html highlightEnglish(data.hadith.text_en)}
        </div>
      {/if}

      {#if data.hadith.text_ar}
        <div class="text-ar arabic" dir="rtl">
          {@html highlightMatn(data.hadith.text_ar)}
        </div>
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
      <GraphView data={graphData} />
    </section>
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

  /* English text — rich serif font with highlighted Prophet's speech */
  .text-en {
    font-family: 'Georgia', 'Palatino Linotype', 'Book Antiqua', serif;
    font-size: 1.1rem;
    line-height: 1.9;
    color: var(--text-primary);
    padding: 24px 28px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    letter-spacing: 0.01em;
  }

  /* Prophet's speech highlighted in English — bold italic on new line */
  .text-en :global(.prophet-speech) {
    display: block;
    color: #1a5c2e;
    font-weight: 700;
    font-style: italic;
    margin-top: 8px;
    padding: 12px 16px;
    border-left: 3px solid #2d8f4e;
    background: rgba(45, 143, 78, 0.04);
    border-radius: 0 var(--radius) var(--radius) 0;
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

  /* Matn (actual hadith content) highlighted in Arabic */
  .text-ar :global(.matn) {
    color: var(--text-primary);
    font-weight: 600;
    font-size: 1.05em;
  }

  .section { margin-bottom: 24px; }
  .section h2 { margin-bottom: 12px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
