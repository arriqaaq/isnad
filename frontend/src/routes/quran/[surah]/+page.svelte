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
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';
  import SurahSidebar from '$lib/components/quran/SurahSidebar.svelte';
  import { preferences } from '$lib/stores/preferences';

  let data: SurahDetailResponse | null = $state(null);
  let loading = $state(true);
  let activeAyah = $state(0);
  let panelAyah: ApiAyah | ApiAyahSearchResult | null = $state(null);
  let noteTarget: { refType: 'ayah'; refId: string; label: string } | null = $state(null);
  let noteIndicators: NoteRefsIndicator = $state({});
  let tafsirMappings: Record<string, TafsirPageRef> = $state({});
  let tafsirTarget: { pageIndex: number; ayahRef: string } | null = $state(null);
  let sidebarOpen = $state(false);
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
      <button class="panel-toggle" class:active={sidebarOpen} onclick={() => { sidebarOpen = !sidebarOpen; }} aria-label="Toggle panel">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="15" y2="12"/><line x1="3" y1="18" x2="18" y2="18"/></svg>
      </button>
    </div>

    <SurahHeader surah={data.surah} />

    <div class="surah-body" class:sidebar-open={sidebarOpen}>
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

      {#if sidebarOpen}
        <div class="surah-sidebar-panel">
          <SurahSidebar
            surah={data.surah}
            totalAyahs={data.surah.ayah_count}
            onNavigateAyah={handleAyahChange}
            onClose={() => { sidebarOpen = false; }}
          />
        </div>
      {/if}
    </div>

    {#if sidebarOpen}
      <button class="sidebar-backdrop-mobile" onclick={() => { sidebarOpen = false; }} aria-label="Close panel"></button>
    {/if}

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
  <BookViewerModal
    bookId={23604}
    pageIndex={tafsirTarget.pageIndex}
    title="Tafsir Ibn Kathir"
    subtitle={tafsirTarget.ayahRef}
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
  .surah-page { padding: 24px; max-width: 1200px; margin: 0 auto; padding-bottom: 72px; }
  .surah-nav { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; margin-bottom: 8px; gap: 8px; }
  .surah-nav.bottom { margin-top: 24px; padding-top: 24px; border-top: 1px solid var(--border); }
  .nav-link { font-size: 0.85rem; color: var(--btn-text); }
  .nav-link:hover { text-decoration: underline; color: var(--text-secondary); }

  .panel-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
    flex-shrink: 0;
  }
  .panel-toggle:hover, .panel-toggle.active {
    background: var(--bg-hover);
    border-color: var(--accent);
    color: var(--accent);
  }

  .surah-body {
    display: flex;
    gap: 0;
  }
  .ayah-list { display: flex; flex-direction: column; flex: 1; min-width: 0; max-width: 800px; }
  .surah-sidebar-panel {
    width: 300px;
    flex-shrink: 0;
    position: sticky;
    top: 0;
    height: calc(100vh - 60px);
    overflow: hidden;
  }

  .sidebar-backdrop-mobile { display: none; }

  @media (max-width: 768px) {
    .surah-page { padding: 12px; padding-bottom: 72px; }
    .surah-nav { padding: 8px 0; }
    .surah-sidebar-panel {
      position: fixed;
      top: 0;
      right: 0;
      width: 85%;
      max-width: 340px;
      height: 100vh;
      z-index: 50;
      background: var(--bg-primary);
      box-shadow: -4px 0 20px rgba(0, 0, 0, 0.15);
    }
    .sidebar-backdrop-mobile {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 49;
      border: none;
      padding: 0;
      cursor: default;
    }
  }
</style>
