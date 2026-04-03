<script lang="ts">
  import type { ApiAyah, ApiAyahSearchResult, ApiQuranWord, ApiVariantReading, AyahHadithResponse, AyahSimilarResponse } from '$lib/types';
  import { getAyahHadiths, getAyahWords, getAyahVariants, getAyahSimilar } from '$lib/api';
  import { truncate } from '$lib/utils';
  import { preferences } from '$lib/stores/preferences';
  import AyahHadithList from './AyahHadithList.svelte';
  import WordMorphology from './WordMorphology.svelte';
  import VariantReadings from './VariantReadings.svelte';
  import SimilarAyahs from './SimilarAyahs.svelte';

  let { ayah, showScore = false, compact = false, hadithCount = 0, similarCount = 0, variantCount = 0, active = false, onplay, reciterFolder }: {
    ayah: ApiAyah | ApiAyahSearchResult;
    showScore?: boolean;
    compact?: boolean;
    hadithCount?: number;
    similarCount?: number;
    variantCount?: number;
    active?: boolean;
    onplay?: (ayah: number) => void;
    reciterFolder?: string;
  } = $props();

  function pad3(n: number): string {
    return String(n).padStart(3, '0');
  }

  let downloadUrl = $derived(
    reciterFolder
      ? `https://everyayah.com/data/${reciterFolder}/${pad3(ayah.surah_number)}${pad3(ayah.ayah_number)}.mp3`
      : `https://everyayah.com/data/Alafasy_128kbps/${pad3(ayah.surah_number)}${pad3(ayah.ayah_number)}.mp3`
  );

  let wordAudio: HTMLAudioElement | null = $state(null);

  function playWordAudio(word: ApiQuranWord) {
    const url = `https://audio.qurancdn.com/wbw/${pad3(word.surah_number)}_${pad3(word.ayah_number)}_${pad3(word.word_position)}.mp3`;
    if (wordAudio) { wordAudio.pause(); }
    wordAudio = new Audio(url);
    wordAudio.play().catch(() => {});
  }

  let showTafsir = $state(false);
  let showHadiths = $state(false);
  let showWords = $state(false);
  let showVariants = $state(false);
  let hadithData: AyahHadithResponse | null = $state(null);
  let hadithLoading = $state(false);
  let words: ApiQuranWord[] | null = $state(null);
  let wordsLoading = $state(false);
  let selectedWord: ApiQuranWord | null = $state(null);
  let variantData: ApiVariantReading[] | null = $state(null);
  let variantLoading = $state(false);
  let showSimilar = $state(false);
  let similarData: AyahSimilarResponse | null = $state(null);
  let similarLoading = $state(false);

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

  async function toggleWords() {
    showWords = !showWords;
    if (showWords && !words && !wordsLoading) {
      wordsLoading = true;
      try {
        words = await getAyahWords(ayah.surah_number, ayah.ayah_number);
      } catch (e) {
        console.error('Failed to load words:', e);
      } finally {
        wordsLoading = false;
      }
    }
  }

  async function toggleVariants() {
    showVariants = !showVariants;
    if (showVariants && !variantData && !variantLoading) {
      variantLoading = true;
      try {
        variantData = await getAyahVariants(ayah.surah_number, ayah.ayah_number);
      } catch (e) {
        console.error('Failed to load variants:', e);
      } finally {
        variantLoading = false;
      }
    }
  }

  async function toggleSimilar() {
    showSimilar = !showSimilar;
    if (showSimilar && !similarData && !similarLoading) {
      similarLoading = true;
      try {
        similarData = await getAyahSimilar(ayah.surah_number, ayah.ayah_number);
      } catch (e) {
        console.error('Failed to load similar ayahs:', e);
      } finally {
        similarLoading = false;
      }
    }
  }
</script>

<div class="ayah-card" class:compact class:active>
  {#if showWords && words}
    <div class="word-grid" dir="rtl">
      {#each words as word}
        <button
          class="word-token"
          title={word.translation ?? ''}
          onclick={() => { playWordAudio(word); selectedWord = word; }}
        >
          <span class="word-ar" style="font-size: {$preferences.arabicFontSize * 0.85}rem">{word.text_ar}</span>
          {#if word.translation}
            <span class="word-en">{word.translation}</span>
          {/if}
          <span class="word-pos">{word.pos}</span>
        </button>
      {/each}
    </div>
  {:else if showWords && wordsLoading}
    <div class="words-loading">Loading word data...</div>
  {:else}
    <div class="ayah-arabic" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">
      {#if ayah.text_ar_tajweed}
        <span class="arabic-text tajweed-text">{@html ayah.text_ar_tajweed}</span>
      {:else}
        <span class="arabic-text">{ayah.text_ar}</span>
        <span class="verse-badge">{ayah.ayah_number}</span>
      {/if}
    </div>
  {/if}

  {#if showWords && words && words[0]?.transliteration}
    <div class="transliteration-line">{words[0].transliteration}</div>
  {/if}

  {#if selectedWord}
    <WordMorphology word={selectedWord} onclose={() => selectedWord = null} />
  {/if}

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
    {#if onplay}
      <button class="audio-btn" onclick={() => onplay(ayah.ayah_number)} aria-label="Play ayah">
        &#9654;
      </button>
    {/if}
    <a class="audio-btn download-btn" href={downloadUrl} download aria-label="Download MP3">
      &#8595;
    </a>
    {#if !compact}
      <button class="words-toggle" class:active-toggle={showWords} onclick={toggleWords}>
        Words
      </button>
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
    {#if variantCount > 0}
      <button class="readings-toggle" class:active-toggle={showVariants} onclick={toggleVariants}>
        Readings ({variantCount})
      </button>
    {/if}
    {#if similarCount > 0}
      <button class="similar-toggle" class:active-toggle={showSimilar} onclick={toggleSimilar}>
        Similar ({similarCount})
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

  {#if showVariants}
    <div class="variant-block">
      {#if variantLoading}
        <div class="variant-loading">Loading variant readings...</div>
      {:else if variantData}
        <VariantReadings variants={variantData} />
      {/if}
    </div>
  {/if}

  {#if showSimilar}
    <div class="similar-block">
      {#if similarLoading}
        <div class="similar-loading">Loading similar ayahs...</div>
      {:else if similarData}
        <SimilarAyahs data={similarData} />
      {/if}
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
  .tajweed-text {
    font-family: 'Amiri Quran', 'Amiri', 'Noto Naskh Arabic', serif;
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
  .ayah-card.active {
    border-left: 3px solid var(--accent);
    background: var(--accent-muted);
  }
  .word-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px;
    justify-content: flex-start;
    margin-bottom: 12px;
  }
  .word-token {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 8px 12px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition);
    min-width: 50px;
  }
  .word-token:hover {
    border-color: var(--accent);
    background: var(--accent-muted);
  }
  .word-ar {
    color: var(--text-primary);
    line-height: 1.6;
  }
  .word-en {
    font-size: 0.65rem;
    color: var(--text-muted);
    max-width: 80px;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .word-pos {
    font-size: 0.55rem;
    color: var(--accent);
    font-family: var(--font-mono);
    font-weight: 600;
  }
  .words-loading {
    padding: 16px 8px;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .transliteration-line {
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
    padding: 0 8px;
    margin-bottom: 8px;
    direction: ltr;
    text-align: left;
  }
  .audio-btn {
    font-size: 0.75rem;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 2px 8px;
    cursor: pointer;
    transition: all var(--transition);
    text-decoration: none;
    line-height: 1.4;
    display: inline-flex;
    align-items: center;
  }
  .audio-btn:hover {
    background: var(--accent-muted);
  }
  .download-btn {
    font-size: 0.85rem;
  }
  .words-toggle, .tafsir-toggle, .hadith-toggle, .readings-toggle, .similar-toggle {
    font-size: 0.75rem;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 2px 10px;
    cursor: pointer;
    transition: all var(--transition);
  }
  .words-toggle.active-toggle, .readings-toggle.active-toggle, .similar-toggle.active-toggle {
    background: var(--accent);
    color: var(--bg-primary);
  }
  .words-toggle:hover, .tafsir-toggle:hover, .hadith-toggle:hover, .readings-toggle:hover, .similar-toggle:hover {
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
  .variant-block {
    margin-top: 12px;
    padding: 16px;
    background: var(--bg-hover);
    border-radius: var(--radius);
    border-left: 3px solid #e89d0d;
  }
  .variant-loading {
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .similar-block {
    margin-top: 12px;
    padding: 16px;
    background: var(--bg-hover);
    border-radius: var(--radius);
    border-left: 3px solid #2196F3;
  }
  .similar-loading {
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
