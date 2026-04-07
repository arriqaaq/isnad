<script lang="ts">
  import { page } from '$app/state';
  import { getSurah } from '$lib/api';
  import type { ApiAyah, ApiAyahSearchResult, SurahDetailResponse } from '$lib/types';
  import SurahHeader from '$lib/components/quran/SurahHeader.svelte';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import AyahSidePanel from '$lib/components/quran/AyahSidePanel.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import RecitationPlayer from '$lib/components/quran/RecitationPlayer.svelte';
  import { preferences } from '$lib/stores/preferences';

  let data: SurahDetailResponse | null = $state(null);
  let loading = $state(true);
  let activeAyah = $state(0);
  let panelAyah: ApiAyah | ApiAyahSearchResult | null = $state(null);
  let playerRef: ReturnType<typeof RecitationPlayer> | undefined = $state(undefined);

  let surahNum = $derived(Number(page.params.surah));
  let startingAyah = $derived(Number(page.url.searchParams.get('ayah')) || 0);
  let reciterFolder = $derived($preferences.selectedReciter ?? 'Alafasy_128kbps');

  $effect(() => {
    loading = true;
    activeAyah = 0;
    getSurah(surahNum).then((d) => {
      data = d;
      loading = false;

      // Scroll to specific ayah if ?ayah=N is in the URL
      if (startingAyah > 0) {
        activeAyah = startingAyah;
        requestAnimationFrame(() => {
          setTimeout(() => {
            const el = document.getElementById(`${surahNum}:${startingAyah}`);
            if (el) {
              el.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
          }, 100);
        });
      }
    });
  });

  function handleAyahChange(ayah: number) {
    activeAyah = ayah;
    // Scroll the active ayah into view
    const el = document.getElementById(`${surahNum}:${ayah}`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  function handleAyahPlay(ayah: number) {
    activeAyah = ayah;
    if (playerRef) {
      playerRef.playAyah(ayah);
    }
  }
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
        <div id="{data.surah.surah_number}:{ayah.ayah_number}">
          <AyahCard
            {ayah}
            active={ayah.ayah_number === activeAyah}
            onplay={handleAyahPlay}
            onopenpanel={(a) => panelAyah = a}
            {reciterFolder}
          />
        </div>
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

{#if panelAyah}
  <AyahSidePanel ayah={panelAyah} onclose={() => panelAyah = null} />
{/if}

{#if data}
  <RecitationPlayer
    bind:this={playerRef}
    surahNumber={data.surah.surah_number}
    ayahCount={data.surah.ayah_count}
    onayahchange={handleAyahChange}
  />
{/if}

<style>
  .surah-page { padding: 24px; max-width: 800px; margin: 0 auto; padding-bottom: 72px; }
  .surah-nav { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; margin-bottom: 8px; }
  .surah-nav.bottom { margin-top: 24px; padding-top: 24px; border-top: 1px solid var(--border); }
  .nav-link { font-size: 0.85rem; color: var(--accent); }
  .nav-link:hover { text-decoration: underline; }
  .ayah-list { display: flex; flex-direction: column; }
  @media (max-width: 640px) {
    .surah-page { padding: 12px; padding-bottom: 72px; }
    .surah-nav { padding: 8px 0; }
  }
</style>
