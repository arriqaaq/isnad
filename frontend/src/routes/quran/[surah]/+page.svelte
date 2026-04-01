<script lang="ts">
  import { page } from '$app/state';
  import { getSurah } from '$lib/api';
  import type { SurahDetailResponse } from '$lib/types';
  import SurahHeader from '$lib/components/quran/SurahHeader.svelte';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: SurahDetailResponse | null = $state(null);
  let loading = $state(true);

  let surahNum = $derived(Number(page.params.surah));

  $effect(() => {
    loading = true;
    getSurah(surahNum).then(d => {
      data = d;
      loading = false;
    });
  });
</script>

<div class="surah-page">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="surah-nav">
      {#if data.surah.surah_number > 1}
        <a href="/quran/{data.surah.surah_number - 1}" class="nav-link">← Previous</a>
      {/if}
      <a href="/quran" class="nav-link">All Surahs</a>
      {#if data.surah.surah_number < 114}
        <a href="/quran/{data.surah.surah_number + 1}" class="nav-link">Next →</a>
      {/if}
    </div>

    <SurahHeader surah={data.surah} />

    <div class="ayah-list">
      {#each data.ayahs as ayah}
        <AyahCard {ayah} />
      {/each}
    </div>

    <div class="surah-nav bottom">
      {#if data.surah.surah_number > 1}
        <a href="/quran/{data.surah.surah_number - 1}" class="nav-link">← Previous Surah</a>
      {/if}
      <a href="/quran" class="nav-link">All Surahs</a>
      {#if data.surah.surah_number < 114}
        <a href="/quran/{data.surah.surah_number + 1}" class="nav-link">Next Surah →</a>
      {/if}
    </div>
  {/if}
</div>

<style>
  .surah-page { padding: 24px; max-width: 800px; margin: 0 auto; }
  .surah-nav { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; margin-bottom: 8px; }
  .surah-nav.bottom { margin-top: 24px; padding-top: 24px; border-top: 1px solid var(--border); }
  .nav-link { font-size: 0.85rem; color: var(--accent); }
  .nav-link:hover { text-decoration: underline; }
  .ayah-list { display: flex; flex-direction: column; }
</style>
