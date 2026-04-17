<script lang="ts">
  import { searchQuran } from '$lib/api';
  import type { ApiSurah, ApiAyahSearchResult } from '$lib/types';

  let { surah, totalAyahs, onNavigateAyah, onClose }: {
    surah: ApiSurah;
    totalAyahs: number;
    onNavigateAyah: (ayahNumber: number) => void;
    onClose?: () => void;
  } = $props();

  let searchQuery = $state('');
  let searchType: 'text' | 'semantic' | 'hybrid' = $state('text');
  let searchResults: ApiAyahSearchResult[] = $state([]);
  let searching = $state(false);
  let jumpInput = $state('');

  async function handleSearch() {
    if (!searchQuery.trim()) return;
    searching = true;
    try {
      const res = await searchQuran(searchQuery, searchType, 20);
      searchResults = res.ayahs ?? [];
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      searching = false;
    }
  }

  function handleSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter') handleSearch();
  }

  function handleJump() {
    const num = parseInt(jumpInput, 10);
    if (num >= 1 && num <= totalAyahs) {
      onNavigateAyah(num);
      jumpInput = '';
    }
  }

  function handleJumpKey(e: KeyboardEvent) {
    if (e.key === 'Enter') handleJump();
  }
</script>

<aside class="surah-sidebar">
  {#if onClose}
    <button class="sidebar-close" onclick={onClose} aria-label="Close">&times;</button>
  {/if}

  <!-- Search -->
  <div class="sidebar-section">
    <div class="section-label">Search Quran</div>
    <div class="search-row">
      <input
        class="search-input"
        type="text"
        placeholder="Search ayahs..."
        bind:value={searchQuery}
        onkeydown={handleSearchKey}
      />
      <button class="search-btn" onclick={handleSearch} disabled={searching}>
        {#if searching}...{:else}Go{/if}
      </button>
    </div>
    <div class="search-type-row">
      <label class="type-option">
        <input type="radio" bind:group={searchType} value="text" /> Text
      </label>
      <label class="type-option">
        <input type="radio" bind:group={searchType} value="semantic" /> Semantic
      </label>
      <label class="type-option">
        <input type="radio" bind:group={searchType} value="hybrid" /> Hybrid
      </label>
    </div>

    {#if searchResults.length > 0}
      <div class="search-results">
        {#each searchResults as result}
          <button
            class="search-result"
            onclick={() => {
              if (result.surah_number === surah.surah_number) {
                onNavigateAyah(result.ayah_number);
              } else {
                window.location.href = `/quran/${result.surah_number}?ayah=${result.ayah_number}`;
              }
            }}
          >
            <span class="result-ref">{result.surah_number}:{result.ayah_number}</span>
            <span class="result-text" dir="rtl">{result.text_ar?.slice(0, 60)}...</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Jump to Ayah -->
  <div class="sidebar-section">
    <div class="section-label">Jump to Ayah</div>
    <div class="search-row">
      <input
        class="search-input"
        type="number"
        min="1"
        max={totalAyahs}
        placeholder="Ayah #"
        bind:value={jumpInput}
        onkeydown={handleJumpKey}
      />
      <button class="search-btn" onclick={handleJump}>Go</button>
    </div>
  </div>

  <!-- Surah Info -->
  <div class="sidebar-section">
    <div class="section-label">Surah Info</div>
    <div class="info-grid">
      <span class="info-key">Name</span>
      <span class="info-val" dir="rtl">{surah.name_ar}</span>
      <span class="info-key">English</span>
      <span class="info-val">{surah.name_en}</span>
      <span class="info-key">Verses</span>
      <span class="info-val">{surah.ayah_count}</span>
      <span class="info-key">Type</span>
      <span class="info-val">{surah.revelation_type}</span>
    </div>
  </div>
</aside>

<style>
  .surah-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    padding: 16px;
    position: relative;
    gap: 4px;
  }
  .sidebar-close {
    display: none;
    position: absolute;
    top: 8px;
    left: 8px;
    width: 30px;
    height: 30px;
    border: none;
    background: var(--bg-hover);
    border-radius: var(--radius-sm);
    font-size: 1.2rem;
    color: var(--text-muted);
    cursor: pointer;
    align-items: center;
    justify-content: center;
  }
  .sidebar-close:hover { background: var(--bg-active); }

  .sidebar-section {
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .sidebar-section:last-child { border-bottom: none; }

  .section-label {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    margin-bottom: 8px;
  }

  .search-row {
    display: flex;
    gap: 6px;
  }
  .search-input {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.82rem;
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); }
  .search-btn {
    padding: 7px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.8rem;
    cursor: pointer;
    transition: all var(--transition);
    white-space: nowrap;
  }
  .search-btn:hover:not(:disabled) { background: var(--bg-hover); border-color: var(--accent); }
  .search-btn:disabled { opacity: 0.5; }

  .search-type-row {
    display: flex;
    gap: 12px;
    margin-top: 6px;
  }
  .type-option {
    font-size: 0.72rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 3px;
    cursor: pointer;
  }
  .type-option input { width: 12px; height: 12px; accent-color: var(--accent); }

  .search-results {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 300px;
    overflow-y: auto;
  }
  .search-result {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    border: none;
    background: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--transition);
  }
  .search-result:hover { background: var(--bg-hover); }
  .result-ref {
    font-size: 0.7rem;
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .result-text {
    font-size: 0.78rem;
    color: var(--text-secondary);
    line-height: 1.5;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .info-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    font-size: 0.8rem;
  }
  .info-key { color: var(--text-muted); }
  .info-val { color: var(--text-primary); font-weight: 500; }

  @media (max-width: 768px) {
    .sidebar-close { display: flex; }
    .surah-sidebar { padding-top: 44px; }
  }
</style>
