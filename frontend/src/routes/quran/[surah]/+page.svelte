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
  import SidebarTabs from '$lib/components/reader/SidebarTabs.svelte';
  import BookChat from '$lib/components/reader/BookChat.svelte';
  import ResizeHandle from '$lib/components/layout/ResizeHandle.svelte';
  import { loadBooksConfig, getBookConfig } from '$lib/stores/books';
  import type { BooksConfig } from '$lib/types';
  import { preferences } from '$lib/stores/preferences';

  let booksConfig: BooksConfig | null = $state(null);
  let tafsirBookId: number | null = $derived(booksConfig?.tafsir_book_id ?? null);
  let tafsirBookConfig = $derived(
    tafsirBookId && booksConfig ? getBookConfig(booksConfig, tafsirBookId) : undefined
  );
  let tafsirBookName = $derived(tafsirBookConfig?.name_en ?? 'Tafsir');
  let tafsirDefaultQuestions = $derived(tafsirBookConfig?.default_questions ?? []);

  let data: SurahDetailResponse | null = $state(null);
  let loading = $state(true);
  let activeAyah = $state(0);
  let panelAyah: ApiAyah | ApiAyahSearchResult | null = $state(null);
  let noteTarget: { refType: 'ayah'; refId: string; label: string } | null = $state(null);
  let noteIndicators: NoteRefsIndicator = $state({});
  let tafsirMappings: Record<string, TafsirPageRef> = $state({});
  let tafsirTarget: { pageIndex: number; ayahRef: string } | null = $state(null);
  let playerRef: ReturnType<typeof RecitationPlayer> | undefined = $state(undefined);

  // Right sidebar state
  let rightCollapsed = $state(true);
  let rightWidth = $state(280);
  const RIGHT_MIN = 200;
  const RIGHT_MAX = 400;
  const RIGHT_COLLAPSED_W = 40;

  let surahNum = $derived(Number(page.params.surah));
  let startingAyah = $derived(Number(page.url.searchParams.get('ayah')) || 0);
  let reciterFolder = $derived($preferences.selectedReciter ?? 'Alafasy_128kbps');

  function saveRightState() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('quran_right_sidebar', JSON.stringify({ collapsed: rightCollapsed, width: rightWidth }));
    }
  }

  $effect(() => {
    if (typeof localStorage !== 'undefined') {
      try {
        const saved = JSON.parse(localStorage.getItem('quran_right_sidebar') ?? '{}');
        if (typeof saved.collapsed === 'boolean') rightCollapsed = saved.collapsed;
        if (typeof saved.width === 'number') rightWidth = Math.max(RIGHT_MIN, Math.min(saved.width, RIGHT_MAX));
      } catch { /* ignore */ }
    }
  });

  $effect(() => {
    loadBooksConfig().then((c) => { booksConfig = c; });
  });

  $effect(() => {
    loading = true;
    activeAyah = 0;
    getSurah(surahNum).then((d) => {
      data = d;
      loading = false;

      const refIds = d.ayahs.map((a: ApiAyah) => `${d.surah.surah_number}:${a.ayah_number}`);
      fetchNoteRefs('ayah', refIds)
        .then(indicators => { noteIndicators = indicators; })
        .catch(() => {});

      getSurahTafsirPages(surahNum)
        .then(res => { tafsirMappings = res.mappings; })
        .catch(() => {});

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

  function handleRightDrag(deltaX: number) {
    rightWidth = Math.max(RIGHT_MIN, Math.min(rightWidth - deltaX, RIGHT_MAX));
    saveRightState();
  }

  function toggleRight() {
    rightCollapsed = !rightCollapsed;
    saveRightState();
  }

  let mobileDrawerOpen = $state(false);
</script>

<div class="surah-page">
  {#if loading}
    <div class="page-content">
      <LoadingSpinner />
    </div>
  {:else if data}
    <!-- Scrollable content area -->
    <div class="page-content">
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
    </div>

    <!-- Desktop: right sidebar (full page height, animates to 0 when collapsed) -->
    <div class="right-area">
      {#if !rightCollapsed}
        <ResizeHandle ondrag={handleRightDrag} />
      {/if}
      <div
        class="right-sidebar"
        style="width: {rightCollapsed ? 0 : rightWidth}px"
      >
        {#if !rightCollapsed}
          <div class="sidebar-inner" style="width: {rightWidth}px">
            <div class="sidebar-header-bar">
              <button class="sidebar-close-btn" onclick={toggleRight} title="Close panel">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
              </button>
            </div>
            <div class="sidebar-scroll">
              <SidebarTabs>
                {#snippet content()}
                  <SurahSidebar
                    surah={data.surah}
                    totalAyahs={data.surah.ayah_count}
                    onNavigateAyah={handleAyahChange}
                    onClose={toggleRight}
                  />
                {/snippet}
                {#snippet chat()}
                  <BookChat
                    bookId={tafsirBookId ?? 0}
                    bookName={tafsirBookName}
                    currentPageIndex={0}
                    onNavigate={() => {}}
                    defaultQuestions={tafsirDefaultQuestions}
                  />
                {/snippet}
              </SidebarTabs>
            </div>
          </div>
        {/if}
      </div>
      <button class="sidebar-open-btn" class:visible={rightCollapsed} onclick={toggleRight} title="Open panel">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 18 9 12 15 6"/></svg>
      </button>
    </div>

    <!-- Mobile: floating button + drawer -->
    <button class="mobile-sidebar-btn" onclick={() => { mobileDrawerOpen = true; }} aria-label="Open panel">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="15" y2="12"/><line x1="3" y1="18" x2="18" y2="18"/></svg>
    </button>

    {#if mobileDrawerOpen}
      <button class="mobile-backdrop" onclick={() => { mobileDrawerOpen = false; }} aria-label="Close panel"></button>
      <div class="mobile-drawer">
        <SidebarTabs>
          {#snippet content()}
            <SurahSidebar
              surah={data.surah}
              totalAyahs={data.surah.ayah_count}
              onNavigateAyah={(ayah) => { mobileDrawerOpen = false; handleAyahChange(ayah); }}
              onClose={() => { mobileDrawerOpen = false; }}
            />
          {/snippet}
          {#snippet chat()}
            <BookChat
              bookId={TAFSIR_BOOK_ID}
              bookName="Tafsir Ibn Kathir"
              currentPageIndex={0}
              onNavigate={() => { mobileDrawerOpen = false; }}
            />
          {/snippet}
        </SidebarTabs>
      </div>
    {/if}
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
    bookId={tafsirBookId ?? 23604}
    pageIndex={tafsirTarget.pageIndex}
    title={tafsirBookName}
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
  /* Top-level: horizontal flex — content + sidebar side by side, full height */
  .surah-page {
    display: flex;
    height: 100%;
  }

  /* Scrollable content area — takes remaining width */
  .page-content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 24px;
    padding-bottom: 72px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  .page-content::-webkit-scrollbar { width: 4px; }
  .page-content::-webkit-scrollbar-track { background: transparent; }
  .page-content::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

  .surah-nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0;
    margin-bottom: 8px;
    gap: 8px;
    max-width: 800px;
    margin-left: auto;
    margin-right: auto;
    width: 100%;
  }
  .surah-nav.bottom {
    margin-top: 24px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
    max-width: 800px;
    margin-left: auto;
    margin-right: auto;
  }
  .nav-link { font-size: 0.85rem; color: var(--btn-text); }
  .nav-link:hover { text-decoration: underline; color: var(--text-secondary); }

  /* Ayah list — centered within content area */
  .ayah-list {
    max-width: 800px;
    margin: 0 auto;
    width: 100%;
  }

  /* Right sidebar area — full height alongside content */
  .right-area {
    display: flex;
    flex-shrink: 0;
    height: 100%;
  }

  .right-sidebar {
    height: 100%;
    overflow: hidden;
    transition: width 200ms ease;
    flex-shrink: 0;
  }

  .sidebar-inner {
    height: 100%;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border-subtle);
  }

  .sidebar-header-bar {
    display: flex;
    justify-content: flex-end;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .sidebar-close-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition);
  }
  .sidebar-close-btn:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .sidebar-scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  /* Collapse/expand button — always present, only visible when collapsed */
  .sidebar-open-btn {
    width: 0;
    height: 100%;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-left: 1px solid var(--border-subtle);
    background: var(--bg-primary);
    color: var(--text-muted);
    cursor: pointer;
    transition: width 200ms ease, opacity 200ms ease;
    overflow: hidden;
    opacity: 0;
    padding: 0;
  }
  .sidebar-open-btn.visible {
    width: 32px;
    opacity: 1;
  }
  .sidebar-open-btn:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }

  /* Mobile */
  .mobile-sidebar-btn { display: none; }
  .mobile-backdrop { display: none; }
  .mobile-drawer { display: none; }

  @media (max-width: 768px) {
    .surah-page { display: block; height: auto; }
    .page-content { padding: 12px; padding-bottom: 72px; overflow-y: visible; }
    .right-area { display: none; }

    .mobile-sidebar-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      position: fixed;
      bottom: 80px;
      right: 16px;
      width: 44px;
      height: 44px;
      border-radius: 50%;
      border: 1px solid var(--border);
      background: var(--bg-surface);
      color: var(--text-secondary);
      box-shadow: 0 2px 8px rgba(0,0,0,0.12);
      cursor: pointer;
      z-index: 30;
      transition: all var(--transition);
    }
    .mobile-sidebar-btn:hover {
      border-color: var(--accent);
      color: var(--accent);
    }

    .mobile-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 49;
      border: none;
      padding: 0;
      cursor: default;
    }

    .mobile-drawer {
      display: block;
      position: fixed;
      top: 0;
      right: 0;
      width: 85%;
      max-width: 340px;
      height: 100vh;
      z-index: 50;
      background: var(--bg-primary);
      box-shadow: -4px 0 20px rgba(0, 0, 0, 0.15);
      overflow-y: auto;
    }
  }
</style>
