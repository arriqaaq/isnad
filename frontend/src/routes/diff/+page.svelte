<script lang="ts">
  import { searchAll, getHadiths, getMatnDiff } from '$lib/api';
  import type { ApiHadith, ApiHadithSearchResult, ApiMatnDiff } from '$lib/types';
  import DiffViewer from '$lib/components/hadith/DiffViewer.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  // Book names for filter dropdown
  const BOOKS = [
    { id: 0, label: 'All Books' },
    { id: 1, label: 'Sahih al-Bukhari' },
    { id: 2, label: 'Sahih Muslim' },
    { id: 3, label: 'Sunan Abu Dawud' },
    { id: 4, label: 'Jami al-Tirmidhi' },
    { id: 5, label: 'Sunan al-Nasai' },
    { id: 6, label: 'Sunan Ibn Majah' },
  ];

  // Side A state
  let queryA = $state('');
  let bookA = $state(0);
  let resultsA: ApiHadithSearchResult[] = $state([]);
  let searchingA = $state(false);
  let selectedA: ApiHadithSearchResult | null = $state(null);
  let debounceA: ReturnType<typeof setTimeout> | null = null;

  // Side B state
  let queryB = $state('');
  let bookB = $state(0);
  let resultsB: ApiHadithSearchResult[] = $state([]);
  let searchingB = $state(false);
  let selectedB: ApiHadithSearchResult | null = $state(null);
  let debounceB: ReturnType<typeof setTimeout> | null = null;

  // Diff state
  let diffResult: ApiMatnDiff | null = $state(null);
  let diffLoading = $state(false);

  async function doSearch(side: 'A' | 'B') {
    const query = side === 'A' ? queryA : queryB;
    const book = side === 'A' ? bookA : bookB;
    if (!query.trim()) {
      if (side === 'A') resultsA = [];
      else resultsB = [];
      return;
    }

    if (side === 'A') searchingA = true;
    else searchingB = true;

    try {
      const num = parseInt(query.trim());
      let hadiths: ApiHadithSearchResult[];

      if (!isNaN(num) && num > 0) {
        // Numeric query — search by hadith number (optionally within a book)
        const res = await getHadiths({ number: num, book: book > 0 ? book : undefined, limit: 20 });
        hadiths = res.data.map(h => ({
          id: h.id,
          hadith_number: h.hadith_number,
          book_id: h.book_id,
          text_ar: h.text_ar,
          text_en: h.text_en,
          narrator_text: h.narrator_text,
          score: null,
        }));
      } else {
        // Text query — full-text search
        const res = await searchAll(query, 'text', 15);
        hadiths = res.hadiths;
        if (book > 0) {
          hadiths = hadiths.filter(h => h.book_id === book);
        }
      }

      if (side === 'A') resultsA = hadiths;
      else resultsB = hadiths;
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      if (side === 'A') searchingA = false;
      else searchingB = false;
    }
  }

  function onInputA() {
    if (debounceA) clearTimeout(debounceA);
    debounceA = setTimeout(() => doSearch('A'), 300);
  }

  function onInputB() {
    if (debounceB) clearTimeout(debounceB);
    debounceB = setTimeout(() => doSearch('B'), 300);
  }

  function selectA(h: ApiHadithSearchResult) {
    selectedA = h;
    resultsA = [];
    queryA = '';
    diffResult = null;
  }

  function selectB(h: ApiHadithSearchResult) {
    selectedB = h;
    resultsB = [];
    queryB = '';
    diffResult = null;
  }

  async function runDiff() {
    if (!selectedA || !selectedB || selectedA.id === selectedB.id) return;
    diffLoading = true;
    diffResult = null;
    try {
      diffResult = await getMatnDiff(selectedA.id, selectedB.id);
    } catch (e) {
      console.error('Diff failed:', e);
    } finally {
      diffLoading = false;
    }
  }

  function bookName(bookId: number): string {
    return BOOKS.find(b => b.id === bookId)?.label ?? `Book ${bookId}`;
  }

  function truncate(text: string | null, len: number): string {
    if (!text) return '';
    return text.length > len ? text.slice(0, len) + '...' : text;
  }
</script>

<svelte:head>
  <title>Diff Hadiths - Ilm</title>
</svelte:head>

<div class="compare-page">
  <div class="page-header">
    <h1>Diff Hadiths</h1>
    <p class="subtitle">Compare the text (matn) of any two hadiths side by side. Search by text or hadith number.</p>
  </div>

  <div class="selectors">
    <!-- Side A -->
    <div class="selector-panel">
      <div class="panel-label">Hadith A</div>
      {#if selectedA}
        <div class="selected-card">
          <div class="selected-info">
            <span class="selected-ref">#{selectedA.hadith_number}</span>
            <span class="selected-book">{selectedA.text_ar ? bookName(selectedA.book_id) : ''}</span>
          </div>
          <div class="selected-preview" dir="rtl">{truncate(selectedA.text_ar, 80)}</div>
          <button class="clear-btn" onclick={() => { selectedA = null; diffResult = null; }}>Change</button>
        </div>
      {:else}
        <div class="search-area">
          <div class="search-row">
            <input
              type="text"
              class="search-input"
              placeholder="Search by text or number..."
              bind:value={queryA}
              oninput={onInputA}
            />
            <select class="book-filter" bind:value={bookA} onchange={() => { if (queryA) doSearch('A'); }}>
              {#each BOOKS as b}
                <option value={b.id}>{b.label}</option>
              {/each}
            </select>
          </div>
          {#if searchingA}
            <div class="search-status">Searching...</div>
          {/if}
          {#if resultsA.length > 0}
            <div class="results-list">
              {#each resultsA as h}
                <button class="result-item" onclick={() => selectA(h)}>
                  <span class="result-ref">#{h.hadith_number} — {bookName(h.book_id)}</span>
                  <span class="result-text" dir="rtl">{truncate(h.text_ar, 60)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Side B -->
    <div class="selector-panel">
      <div class="panel-label">Hadith B</div>
      {#if selectedB}
        <div class="selected-card">
          <div class="selected-info">
            <span class="selected-ref">#{selectedB.hadith_number}</span>
            <span class="selected-book">{selectedB.text_ar ? bookName(selectedB.book_id) : ''}</span>
          </div>
          <div class="selected-preview" dir="rtl">{truncate(selectedB.text_ar, 80)}</div>
          <button class="clear-btn" onclick={() => { selectedB = null; diffResult = null; }}>Change</button>
        </div>
      {:else}
        <div class="search-area">
          <div class="search-row">
            <input
              type="text"
              class="search-input"
              placeholder="Search by text or number..."
              bind:value={queryB}
              oninput={onInputB}
            />
            <select class="book-filter" bind:value={bookB} onchange={() => { if (queryB) doSearch('B'); }}>
              {#each BOOKS as b}
                <option value={b.id}>{b.label}</option>
              {/each}
            </select>
          </div>
          {#if searchingB}
            <div class="search-status">Searching...</div>
          {/if}
          {#if resultsB.length > 0}
            <div class="results-list">
              {#each resultsB as h}
                <button class="result-item" onclick={() => selectB(h)}>
                  <span class="result-ref">#{h.hadith_number} — {bookName(h.book_id)}</span>
                  <span class="result-text" dir="rtl">{truncate(h.text_ar, 60)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="compare-action">
    <button
      class="compare-btn"
      onclick={runDiff}
      disabled={!selectedA || !selectedB || selectedA.id === selectedB.id || diffLoading}
    >
      {diffLoading ? 'Computing...' : 'Compare'}
    </button>
    {#if selectedA && selectedB && selectedA.id === selectedB.id}
      <span class="compare-warn">Select two different hadiths</span>
    {/if}
  </div>

  {#if diffLoading}
    <LoadingSpinner />
  {/if}

  {#if diffResult}
    <DiffViewer result={diffResult} />
  {/if}
</div>

<style>
  .compare-page {
    padding: 24px;
    max-width: 1000px;
    margin: 0 auto;
  }

  .page-header {
    margin-bottom: 24px;
  }
  .page-header h1 {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 4px;
  }
  .subtitle {
    font-size: 0.88rem;
    color: var(--text-muted);
    margin: 0;
  }

  /* Two-panel selector layout */
  .selectors {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 20px;
  }

  .selector-panel {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
    min-height: 160px;
  }

  .panel-label {
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 10px;
  }

  /* Search area */
  .search-row {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }

  .search-input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.88rem;
    outline: none;
  }
  .search-input:focus {
    border-color: var(--accent);
  }

  .book-filter {
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.8rem;
    min-width: 120px;
  }

  .search-status {
    font-size: 0.78rem;
    color: var(--text-muted);
    padding: 4px 0;
  }

  .results-list {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .result-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    text-align: left;
    cursor: pointer;
    transition: background var(--transition);
    width: 100%;
  }
  .result-item:hover {
    background: var(--bg-hover);
  }

  .result-ref {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--accent);
  }

  .result-text {
    font-size: 0.8rem;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Selected hadith card */
  .selected-card {
    background: var(--accent-muted);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    padding: 12px;
  }

  .selected-info {
    display: flex;
    gap: 8px;
    align-items: baseline;
    margin-bottom: 6px;
  }

  .selected-ref {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--accent);
  }

  .selected-book {
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .selected-preview {
    font-size: 0.85rem;
    color: var(--text-primary);
    line-height: 1.8;
    margin-bottom: 8px;
    max-height: 60px;
    overflow: hidden;
  }

  .clear-btn {
    font-size: 0.75rem;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
  .clear-btn:hover {
    color: var(--accent);
  }

  /* Compare button */
  .compare-action {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 24px;
  }

  .compare-btn {
    padding: 10px 28px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--radius);
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity var(--transition);
  }
  .compare-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
  .compare-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .compare-warn {
    font-size: 0.8rem;
    color: var(--warning);
  }

  @media (max-width: 768px) {
    .compare-page { padding: 12px; }
    .selectors { grid-template-columns: 1fr; }
    .search-row { flex-direction: column; }
    .book-filter { min-width: unset; }
  }
</style>
