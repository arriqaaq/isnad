<script lang="ts">
  import type { ApiAyah, ApiAyahSearchResult, CCManuscript, AyahHadithResponse, AyahSimilarResponse } from '$lib/types';
  import { getAyahManuscripts, getAyahHadiths, getAyahSimilar } from '$lib/api';
  import ManuscriptCard from './ManuscriptCard.svelte';
  import AyahHadithList from './AyahHadithList.svelte';
  import SimilarAyahs from './SimilarAyahs.svelte';
  import { onMount } from 'svelte';

  let { ayah, onclose }: {
    ayah: ApiAyah | ApiAyahSearchResult;
    onclose: () => void;
  } = $props();

  let manuscripts: CCManuscript[] = $state([]);
  let manuscriptsLoading = $state(true);
  let hadithData: AyahHadithResponse | null = $state(null);
  let hadithLoading = $state(true);
  let similarData: AyahSimilarResponse | null = $state(null);
  let similarLoading = $state(true);

  onMount(() => {
    // Load all data in parallel
    getAyahManuscripts(ayah.surah_number, ayah.ayah_number)
      .then(data => { manuscripts = data; })
      .catch(() => {})
      .finally(() => { manuscriptsLoading = false; });

    getAyahHadiths(ayah.surah_number, ayah.ayah_number, true)
      .then(data => { hadithData = data; })
      .catch(() => {})
      .finally(() => { hadithLoading = false; });

    getAyahSimilar(ayah.surah_number, ayah.ayah_number)
      .then(data => { similarData = data; })
      .catch(() => {})
      .finally(() => { similarLoading = false; });
  });

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('panel-backdrop')) {
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel-backdrop" onclick={handleBackdrop}>
  <div class="side-panel">
    <div class="panel-header">
      <span class="panel-ref">{ayah.surah_number}:{ayah.ayah_number}</span>
      <button class="panel-close" onclick={onclose}>&times;</button>
    </div>

    <div class="panel-content">
      <!-- Related Hadiths -->
      <section class="panel-section">
        <div class="section-label">Related Hadiths</div>
        {#if hadithLoading}
          <div class="section-loading">Loading hadiths...</div>
        {:else if hadithData && (hadithData.curated.length > 0 || (hadithData.related && hadithData.related.length > 0))}
          <AyahHadithList data={hadithData} />
        {:else}
          <div class="section-empty">No related hadiths found.</div>
        {/if}
      </section>

      <!-- Similar Ayahs -->
      <section class="panel-section">
        <div class="section-label">Similar Ayahs</div>
        {#if similarLoading}
          <div class="section-loading">Loading similar ayahs...</div>
        {:else if similarData && (similarData.similar.length > 0 || similarData.phrases.length > 0)}
          <SimilarAyahs data={similarData} />
        {:else}
          <div class="section-empty">No similar ayahs found.</div>
        {/if}
      </section>

      <!-- Tafsir -->
      {#if ayah.tafsir_en}
        <section class="panel-section">
          <div class="section-label">Tafsir Ibn Kathir</div>
          <div class="tafsir-content">{@html ayah.tafsir_en}</div>
        </section>
      {/if}

      <!-- Manuscripts -->
      <section class="panel-section">
        <div class="section-label">Manuscripts</div>
        {#if manuscriptsLoading}
          <div class="section-loading">Loading manuscripts...</div>
        {:else if manuscripts.length > 0}
          <div class="manuscript-grid">
            {#each manuscripts as ms}
              <ManuscriptCard manuscript={ms} />
            {/each}
          </div>
        {:else}
          <div class="section-empty">No manuscripts found for this verse.</div>
        {/if}
      </section>
    </div>
  </div>
</div>

<style>
  .panel-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    z-index: 200;
    animation: fadeIn 0.15s ease-out;
  }

  .side-panel {
    position: fixed;
    right: 0;
    top: 0;
    height: 100vh;
    width: 600px;
    max-width: 100vw;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.15);
    display: flex;
    flex-direction: column;
    animation: slideIn 0.2s ease-out;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .panel-ref {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }

  .panel-close {
    font-size: 1.5rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }
  .panel-close:hover {
    color: var(--text-primary);
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .panel-section {
    margin-bottom: 28px;
  }

  .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    margin-bottom: 12px;
  }

  .section-loading, .section-empty {
    font-size: 0.85rem;
    color: var(--text-muted);
    padding: 8px 0;
  }

  .manuscript-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .tafsir-content {
    font-size: 0.85rem;
    line-height: 1.7;
    color: var(--text-secondary);
  }

  .tafsir-content :global(h2.title) {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 16px 0 8px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 4px;
  }
  .tafsir-content :global(h2.title:first-child) {
    margin-top: 0;
  }
  .tafsir-content :global(p) {
    margin: 8px 0;
    line-height: 1.7;
  }
  .tafsir-content :global(div.text_uthmani) {
    font-size: 1.1rem;
    text-align: right;
    direction: rtl;
    color: var(--text-primary);
    margin: 8px 0;
    padding: 8px;
    background: var(--bg-surface);
    border-radius: var(--radius-sm);
  }

  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @media (max-width: 640px) {
    .side-panel {
      width: 100vw;
    }
  }
</style>
