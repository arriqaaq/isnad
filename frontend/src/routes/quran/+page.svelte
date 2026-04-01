<script lang="ts">
  import { getSurahs, getQuranStats } from '$lib/api';
  import type { ApiSurah, QuranStatsResponse } from '$lib/types';
  import SurahRow from '$lib/components/quran/SurahRow.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let surahs: ApiSurah[] = $state([]);
  let stats: QuranStatsResponse | null = $state(null);
  let loading = $state(true);
  let filter = $state('');
  let sortBy: 'number' | 'revelation' = $state('number');

  $effect(() => {
    Promise.all([getSurahs(), getQuranStats()]).then(([s, st]) => {
      surahs = s;
      stats = st;
      loading = false;
    });
  });

  let filtered = $derived(() => {
    let list = surahs;
    if (filter.trim()) {
      const q = filter.toLowerCase();
      list = list.filter(s =>
        s.name_translit.toLowerCase().includes(q) ||
        s.name_en.toLowerCase().includes(q) ||
        s.name_ar.includes(filter) ||
        String(s.surah_number) === q
      );
    }
    if (sortBy === 'revelation') {
      // Meccan first, then Medinan
      list = [...list].sort((a, b) => {
        if (a.revelation_type !== b.revelation_type) {
          return a.revelation_type === 'Meccan' ? -1 : 1;
        }
        return a.surah_number - b.surah_number;
      });
    }
    return list;
  });
</script>

<div class="quran-page">
  <div class="page-header">
    <h1>Quran</h1>
    {#if stats}
      <div class="stats-row">
        <span class="stat">{stats.surah_count} Surahs</span>
        <span class="stat-sep">·</span>
        <span class="stat">{stats.ayah_count} Ayahs</span>
      </div>
    {/if}
  </div>

  <div class="controls">
    <input type="text" placeholder="Search surahs..." bind:value={filter} class="search-input" />
    <div class="sort-toggle">
      <button class="toggle-btn" class:active={sortBy === 'number'} onclick={() => sortBy = 'number'}>Surah</button>
      <button class="toggle-btn" class:active={sortBy === 'revelation'} onclick={() => sortBy = 'revelation'}>Revelation</button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else}
    <div class="surah-list">
      {#each filtered() as surah}
        <SurahRow {surah} />
      {/each}
      {#if filtered().length === 0}
        <div class="empty">No surahs match "{filter}"</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .quran-page { padding: 24px; }
  .page-header { margin-bottom: 20px; }
  h1 { margin-bottom: 4px; }
  .stats-row { display: flex; gap: 8px; align-items: center; font-size: 0.85rem; color: var(--text-muted); }
  .stat-sep { color: var(--border); }
  .controls { display: flex; gap: 8px; margin-bottom: 16px; align-items: center; }
  .search-input { flex: 1; max-width: 400px; }
  .sort-toggle { display: flex; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .toggle-btn { padding: 8px 14px; font-size: 0.8rem; background: var(--bg-surface); color: var(--text-secondary); transition: all var(--transition); }
  .toggle-btn.active { background: var(--accent); color: var(--bg-primary); }
  .surah-list { border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
