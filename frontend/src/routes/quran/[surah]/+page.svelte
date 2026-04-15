<script lang="ts">
  import { page } from '$app/state';
  import { getSurah, fetchNoteRefs, getSurahTafsirPages } from '$lib/api';
  import type { ApiAyah, ApiAyahSearchResult, SurahDetailResponse, NoteRefsIndicator, TafsirPageRef } from '$lib/types';
  import SurahHeader from '$lib/components/quran/SurahHeader.svelte';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import AyahSidePanel from '$lib/components/quran/AyahSidePanel.svelte';
  import NoteModal from '$lib/components/notes/NoteModal.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import RecitationPlayer from '$lib/components/quran/RecitationPlayer.svelte';
  import TafsirModal from '$lib/components/reader/TafsirModal.svelte';
  import { preferences } from '$lib/stores/preferences';

  let data: SurahDetailResponse | null = $state(null);
  let loading = $state(true);
  let activeAyah = $state(0);
  let panelAyah: ApiAyah | ApiAyahSearchResult | null = $state(null);
  let noteTarget: { refType: 'ayah'; refId: string; label: string } | null = $state(null);
  let noteIndicators: NoteRefsIndicator = $state({});
  let tafsirMappings: Record<string, TafsirPageRef> = $state({});
  let tafsirTarget: { pageIndex: number; ayahRef: string } | null = $state(null);
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

      // Load note indicators for all ayahs in this surah
      const refIds = d.ayahs.map((a: ApiAyah) => `${d.surah.surah_number}:${a.ayah_number}`);
      fetchNoteRefs('ayah', refIds)
        .then(indicators => { noteIndicators = indicators; })
        .catch(() => {});

      // Load tafsir page mappings for this surah
      getSurahTafsirPages(surahNum)
        .then(res => { tafsirMappings = res.mappings; })
        .catch(() => {});

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
            onopennote={(a) => { noteTarget = { refType: 'ayah', refId: `${a.surah_number}:${a.ayah_number}`, label: `${a.surah_number}:${a.ayah_number}` }; }}
            onopentafsir={(info) => { tafsirTarget = info; }}
            noteIndicator={noteIndicators[`${data.surah.surah_number}:${ayah.ayah_number}`]}
            tafsirPage={tafsirMappings[String(ayah.ayah_number)]}
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

{#if noteTarget}
  <NoteModal
    refType={noteTarget.refType}
    refId={noteTarget.refId}
    refLabel="Quran {noteTarget.refId}"
    onclose={() => noteTarget = null}
  />
{/if}

{#if tafsirTarget}
  <TafsirModal
    bookId={23604}
    pageIndex={tafsirTarget.pageIndex}
    ayahRef={tafsirTarget.ayahRef}
    onclose={() => { tafsirTarget = null; }}
  />
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
  .nav-link { font-size: 0.85rem; color: var(--btn-text); }
  .nav-link:hover { text-decoration: underline; color: var(--text-secondary); }
  .ayah-list { display: flex; flex-direction: column; }
  @media (max-width: 640px) {
    .surah-page { padding: 12px; padding-bottom: 72px; }
    .surah-nav { padding: 8px 0; }
  }
</style>
