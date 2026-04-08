<script lang="ts">
  import type { ApiAyah, ApiAyahSearchResult, ApiQuranWord } from '$lib/types';
  import { getAyahWords } from '$lib/api';
  import { truncate } from '$lib/utils';
  import { preferences } from '$lib/stores/preferences';
  import { fetchGlyphData, loadPageFont, getPageFontFamily, getVerseGlyph } from '$lib/quranFonts';
  import WordMorphology from './WordMorphology.svelte';

  // QCF glyph state
  let glyphText: string | null = $state(null);
  let glyphPage: number | null = $state(null);
  let glyphFontFamily: string = $state('');
  let glyphReady = $state(false);

  $effect(() => {
    const mode = $preferences.quranFont;
    if (mode === 'uthmani') {
      glyphReady = false;
      return;
    }
    const chapter = ayah.surah_number;
    const verse = ayah.ayah_number;
    fetchGlyphData(chapter).then(async () => {
      const glyph = getVerseGlyph(chapter, verse);
      if (!glyph) return;
      glyphText = glyph.code_v2;
      glyphPage = glyph.page;
      glyphFontFamily = getPageFontFamily(glyph.page);
      await loadPageFont(glyph.page, mode);
      glyphReady = true;
    }).catch(() => { glyphReady = false; });
  });

  let { ayah, showScore = false, compact = false, active = false, onplay, onopenpanel, reciterFolder }: {
    ayah: ApiAyah | ApiAyahSearchResult;
    showScore?: boolean;
    compact?: boolean;
    active?: boolean;
    onplay?: (ayah: number) => void;
    onopenpanel?: (ayah: ApiAyah | ApiAyahSearchResult) => void;
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

  // State
  let showWords = $state(false);
  let words: ApiQuranWord[] | null = $state(null);
  let selectedWord: ApiQuranWord | null = $state(null);
  let score = $derived('score' in ayah ? (ayah as ApiAyahSearchResult).score : null);

  // Eagerly preload word data (for hover/click on default Arabic view)
  $effect(() => {
    if (!compact && !words) {
      getAyahWords(ayah.surah_number, ayah.ayah_number)
        .then(w => { words = w; })
        .catch(() => {});
    }
  });
</script>

<div class="ayah-card" class:compact class:active>
  <!-- Arabic text: always render as interactive word spans when data is loaded -->
  {#if showWords && words}
    <!-- Detailed word-by-word grid mode -->
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
  {:else}
    <!-- Default view: interactive inline word spans or plain text fallback -->
    <div class="ayah-arabic" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">
      {#if $preferences.quranFont !== 'uthmani' && glyphReady && glyphText}
        <span class="arabic-text qcf-text" style="font-family: {glyphFontFamily}">{glyphText}</span>
      {:else if words}
        <!-- Interactive words: each word is a hoverable/clickable span -->
        {#each words as word}
          <span
            class="word-inline"
            title={word.translation ?? ''}
            role="button"
            tabindex="0"
            onclick={() => { playWordAudio(word); selectedWord = word; }}
            onkeydown={(e) => { if (e.key === 'Enter') { playWordAudio(word); selectedWord = word; } }}
          >{word.text_ar}</span>{' '}
        {/each}
        <span class="verse-badge">{ayah.ayah_number}</span>
      {:else}
        <!-- Fallback: plain text while words load -->
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
      <button class="words-toggle" class:active-toggle={showWords} onclick={() => showWords = !showWords}>
        Words
      </button>
    {/if}
    {#if !compact && onopenpanel}
      <button class="detail-toggle" onclick={() => onopenpanel(ayah)}>
        Details
      </button>
    {/if}
  </div>


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
  .qcf-text {
    letter-spacing: 0;
    word-spacing: 0.08em;
  }
  .word-inline {
    color: var(--text-primary);
    cursor: pointer;
    border-radius: 2px;
    padding: 0 1px;
    transition: background 0.15s;
  }
  .word-inline:hover {
    background: var(--accent-muted);
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
    flex-wrap: wrap;
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
    color: var(--btn-text);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
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
    background: var(--btn-bg-hover);
    border-color: var(--btn-border-hover);
  }
  .download-btn {
    font-size: 0.85rem;
  }
  .words-toggle, .detail-toggle {
    font-size: 0.75rem;
    color: var(--btn-text);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    border-radius: var(--radius-sm);
    padding: 2px 10px;
    cursor: pointer;
    transition: all var(--transition);
  }
  .words-toggle.active-toggle {
    background: var(--btn-text);
    color: var(--bg-primary);
    border-color: var(--btn-text);
  }
  .words-toggle:hover, .detail-toggle:hover {
    background: var(--btn-bg-hover);
    border-color: var(--btn-border-hover);
  }
  @media (max-width: 640px) {
    .ayah-card { padding: 14px 0; }
    .ayah-arabic { padding: 0 12px; }
    .ayah-translation { padding: 0 12px; }
    .ayah-footer { padding: 0 12px; }
    .hadith-block { margin-left: 12px; margin-right: 12px; }
  }
</style>
