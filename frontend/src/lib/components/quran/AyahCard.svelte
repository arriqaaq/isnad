<script lang="ts">
  import type { ApiAyah, ApiAyahSearchResult, AyahHadithResponse } from '$lib/types';
  import { getAyahHadiths } from '$lib/api';
  import { truncate } from '$lib/utils';
  import { preferences } from '$lib/stores/preferences';
  import AyahHadithList from './AyahHadithList.svelte';

  let { ayah, showScore = false, compact = false, hadithCount = 0 }: {
    ayah: ApiAyah | ApiAyahSearchResult;
    showScore?: boolean;
    compact?: boolean;
    hadithCount?: number;
  } = $props();

  let showTafsir = $state(false);
  let showHadiths = $state(false);
  let hadithData: AyahHadithResponse | null = $state(null);
  let hadithLoading = $state(false);
  let score = $derived('score' in ayah ? (ayah as ApiAyahSearchResult).score : null);

  async function toggleHadiths() {
    showHadiths = !showHadiths;
    if (showHadiths && !hadithData && !hadithLoading) {
      hadithLoading = true;
      try {
        hadithData = await getAyahHadiths(ayah.surah_number, ayah.ayah_number, true);
      } catch (e) {
        console.error('Failed to load hadiths:', e);
      } finally {
        hadithLoading = false;
      }
    }
  }
</script>

<div class="ayah-card" class:compact>
  <div class="ayah-arabic" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">
    {#if ayah.text_ar_tajweed}
      <span class="arabic-text tajweed-text">{@html ayah.text_ar_tajweed}</span>
    {:else}
      <span class="arabic-text">{ayah.text_ar}</span>
      <span class="verse-badge">{ayah.ayah_number}</span>
    {/if}
  </div>

  {#if ayah.text_en}
    <div class="ayah-translation" style="font-size: {$preferences.englishFontSize}rem">
      {#if compact}
        {truncate(ayah.text_en, 200)}
      {:else}
        {ayah.text_en}
      {/if}
    </div>
  {/if}

  <div class="ayah-footer">
    <span class="verse-ref">{ayah.surah_number}:{ayah.ayah_number}</span>
    {#if showScore && score}
      <span class="score mono">{score.toFixed(3)}</span>
    {/if}
    {#if hadithCount > 0}
      <button class="hadith-toggle" onclick={toggleHadiths}>
        {showHadiths ? 'Hide' : 'Show'} Hadith ({hadithCount})
      </button>
    {/if}
    {#if ayah.tafsir_en}
      <button class="tafsir-toggle" onclick={() => showTafsir = !showTafsir}>
        {showTafsir ? 'Hide' : 'Show'} Tafsir
      </button>
    {/if}
  </div>

  {#if showHadiths}
    <div class="hadith-block">
      {#if hadithLoading}
        <div class="hadith-loading">Loading hadiths...</div>
      {:else if hadithData}
        <AyahHadithList data={hadithData} />
      {/if}
    </div>
  {/if}

  {#if showTafsir && ayah.tafsir_en}
    <div class="tafsir-block">
      <div class="tafsir-label">Tafsir Ibn Kathir</div>
      <div class="tafsir-text">{@html ayah.tafsir_en}</div>
    </div>
  {/if}
</div>

<style>
  .ayah-card {
    padding: 20px 0;
    border-bottom: 1px solid var(--border);
  }
  .ayah-card.compact {
    padding: 14px 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .ayah-arabic {
    text-align: right;
    line-height: 2.2;
    margin-bottom: 12px;
    padding: 0 8px;
  }
  .arabic-text {
    color: var(--text-primary);
  }
  .verse-badge {
    display: inline;
    font-size: 0.65em;
    color: var(--accent);
    vertical-align: middle;
    font-family: var(--font-mono);
  }
  .ayah-translation {
    line-height: 1.7;
    color: var(--text-secondary);
    text-align: left;
    margin-bottom: 8px;
    padding: 0 8px;
  }
  .ayah-footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 8px;
  }
  .verse-ref {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .score {
    font-size: 0.75rem;
    color: var(--success);
  }
  .tafsir-toggle, .hadith-toggle {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 2px 10px;
    cursor: pointer;
    transition: all var(--transition);
  }
  .hadith-toggle {
    margin-left: 0;
  }
  .tafsir-toggle:hover, .hadith-toggle:hover {
    background: var(--accent-muted);
  }
  .hadith-block {
    margin-top: 12px;
    padding: 16px;
    background: var(--bg-hover);
    border-radius: var(--radius);
    border-left: 3px solid var(--success);
  }
  .hadith-loading {
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .tafsir-block {
    margin-top: 12px;
    padding: 16px;
    background: var(--bg-hover);
    border-radius: var(--radius);
    border-left: 3px solid var(--accent);
  }
  .tafsir-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }
  .tafsir-text {
    font-size: 0.85rem;
    line-height: 1.7;
    color: var(--text-secondary);
    max-height: 400px;
    overflow-y: auto;
  }
  /* Tafsir HTML content styling */
  .tafsir-text :global(h2.title) {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 16px 0 8px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 4px;
  }
  .tafsir-text :global(h2.title:first-child) {
    margin-top: 0;
  }
  .tafsir-text :global(p) {
    margin: 8px 0;
    line-height: 1.7;
  }
  .tafsir-text :global(div.text_uthmani) {
    font-size: 1.1rem;
    text-align: right;
    direction: rtl;
    color: var(--text-primary);
    margin: 8px 0;
    padding: 8px;
    background: var(--bg-surface);
    border-radius: var(--radius-sm);
  }

  /* Tajweed color coding */
  .tajweed-text :global(tajweed.ham_wasl) { color: #AAAAAA; }
  .tajweed-text :global(tajweed.laam_shamsiyah) { color: transparent; font-size: 0; }
  .tajweed-text :global(tajweed.madda_normal) { color: #E87D0D; }
  .tajweed-text :global(tajweed.madda_permissible) { color: #2196F3; }
  .tajweed-text :global(tajweed.madda_necessary) { color: #D50000; }
  .tajweed-text :global(tajweed.madda_obligatory) { color: #00BCD4; }
  .tajweed-text :global(tajweed.ghunnah) { color: #4CAF50; }
  .tajweed-text :global(tajweed.ikhpiaa_shafawi) { color: #4CAF50; }
  .tajweed-text :global(tajweed.ikhfa) { color: #4CAF50; }
  .tajweed-text :global(tajweed.iqlab) { color: #009688; }
  .tajweed-text :global(tajweed.idgham_ghunnah) { color: #4CAF50; }
  .tajweed-text :global(tajweed.idgham_no_ghunnah) { color: #4CAF50; }
  .tajweed-text :global(tajweed.idgham_shafawi) { color: #4CAF50; }
  .tajweed-text :global(tajweed.qalpiaqpiala) { color: #B71C1C; }
  /* Mobile responsive */
  @media (max-width: 640px) {
    .ayah-card { padding: 14px 0; }
    .ayah-arabic { padding: 0 12px; }
    .ayah-translation { padding: 0 12px; }
    .ayah-footer { padding: 0 12px; flex-wrap: wrap; }
    .tafsir-block, .hadith-block { margin-left: 12px; margin-right: 12px; }
  }

  /* Verse end number badge from quran.com tajweed text */
  .tajweed-text :global(span.end) {
    display: inline;
    font-size: 0.65em;
    color: var(--accent);
    vertical-align: middle;
  }
</style>
