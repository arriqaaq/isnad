<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { getAllTafsirsForAyah, getSurah } from '$lib/api';
  import type { AllTafsirsResponse, ApiAyah, SurahDetailResponse } from '$lib/types';
  import { parseVerseRef, AYAH_COUNTS } from '$lib/constants/ayahCounts';
  import VersePicker from '$lib/components/tafsir/VersePicker.svelte';
  import TafsirAccordion from '$lib/components/tafsir/TafsirAccordion.svelte';
  import TafsirAskDrawer from '$lib/components/tafsir/TafsirAskDrawer.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let current = $derived.by(() => {
    const raw = page.url.searchParams.get('verse') ?? '1:1';
    return parseVerseRef(raw) ?? { surah: 1, ayah: 1 };
  });
  const surah = $derived(current.surah);
  const ayah = $derived(current.ayah);

  let surahData: SurahDetailResponse | null = $state(null);
  let surahLoading = $state(false);
  let tafsirData: AllTafsirsResponse | null = $state(null);
  let tafsirLoading = $state(false);
  let errorMsg: string | null = $state(null);

  let askOpen = $state(false);
  let lastLoadedSurah = $state(-1);

  $effect(() => {
    if (surah === lastLoadedSurah && surahData) return;
    surahLoading = true;
    getSurah(surah)
      .then((d) => { surahData = d; lastLoadedSurah = surah; })
      .catch((e) => { errorMsg = `Failed to load surah: ${e?.message ?? e}`; })
      .finally(() => { surahLoading = false; });
  });

  $effect(() => {
    tafsirLoading = true;
    errorMsg = null;
    // Anchor on (surah, ayah) so the fetch re-runs whenever either changes.
    const s = surah;
    const a = ayah;
    getAllTafsirsForAyah(s, a)
      .then((d) => { tafsirData = d; })
      .catch((e) => {
        tafsirData = null;
        errorMsg = `Failed to load tafsir: ${e?.message ?? e}`;
      })
      .finally(() => { tafsirLoading = false; });
  });

  const currentAyah = $derived.by(() => {
    if (!surahData) return null;
    return surahData.ayahs.find((row: ApiAyah) => row.ayah_number === ayah) ?? null;
  });
  const surahName = $derived.by(() =>
    surahData ? surahData.surah.name_translit : `Surah ${surah}`
  );
  const surahNameAr = $derived.by(() => (surahData ? surahData.surah.name_ar : ''));

  function handlePick(v: { surah: number; ayah: number }) {
    const url = new URL(page.url);
    url.searchParams.set('verse', `${v.surah}:${v.ayah}`);
    goto(url.pathname + url.search, { keepFocus: true, noScroll: true, replaceState: false });
  }

  function handleKey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      askOpen = !askOpen;
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

<svelte:head>
  <title>Tafsir · {surah}:{ayah}</title>
</svelte:head>

<div class="tafsir-page">
  <header class="page-header">
    <div class="title-row">
      <h1 class="page-title">Tafsir</h1>
      <button
        class="ask-btn"
        type="button"
        onclick={() => (askOpen = true)}
        title="Ask AI (⌘K)"
      >
        <span>Ask AI</span>
        <kbd>⌘K</kbd>
      </button>
    </div>
    <p class="page-subtitle">
      Pick a verse to read every tafsir for it, or ask a free-form question across the whole tafsir corpus.
    </p>
  </header>

  <VersePicker surah={surah} ayah={ayah} onsubmit={handlePick} />

  <section class="ayah-context" dir="rtl">
    <div class="context-meta">
      <span class="surah-label">{surahName} <span class="surah-ar">{surahNameAr}</span></span>
      <span class="ref-label">{surah}:{ayah}</span>
    </div>
    {#if currentAyah}
      <p class="ayah-ar">{currentAyah.text_ar}</p>
      {#if currentAyah.text_en}
        <p class="ayah-en" dir="ltr">{currentAyah.text_en}</p>
      {/if}
    {:else if surahLoading}
      <div class="context-placeholder"><LoadingSpinner /></div>
    {:else}
      <p class="context-placeholder">Ayah text unavailable.</p>
    {/if}
  </section>

  <section class="tafsir-body">
    {#if tafsirLoading && !tafsirData}
      <div class="loading-row"><LoadingSpinner /> <span>Loading tafsir…</span></div>
    {:else if errorMsg}
      <div class="error-state">{errorMsg}</div>
    {:else if tafsirData}
      <TafsirAccordion entries={tafsirData.entries} english={tafsirData.english} />
    {/if}
  </section>
</div>

<TafsirAskDrawer
  open={askOpen}
  verse={{ surah, ayah }}
  onclose={() => (askOpen = false)}
/>

<style>
  .tafsir-page {
    max-width: 900px;
    margin: 0 auto;
    padding: 24px 0 80px;
  }
  .page-header {
    padding: 0 20px 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  .page-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }
  .page-subtitle {
    font-size: 0.9rem;
    color: var(--text-muted);
    margin: 6px 0 0;
  }

  .ask-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--bg-primary);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity var(--transition);
  }
  .ask-btn:hover { opacity: 0.9; }
  .ask-btn kbd {
    background: rgba(255, 255, 255, 0.25);
    padding: 1px 6px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
  }

  .ayah-context {
    padding: 18px 20px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    text-align: right;
  }
  .context-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    direction: ltr;
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-bottom: 12px;
  }
  .surah-label { font-weight: 500; color: var(--text-secondary); }
  .surah-ar { font-family: var(--font-arabic-text), serif; margin-left: 6px; color: var(--text-muted); }
  .ref-label {
    font-family: var(--font-mono);
    color: var(--accent);
    background: var(--accent-muted);
    padding: 2px 8px;
    border-radius: 10px;
  }
  .ayah-ar {
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
    font-size: 1.4rem;
    line-height: 2.2;
    color: var(--text-primary);
    margin: 0;
  }
  .ayah-en {
    margin: 12px 0 0;
    font-family: var(--font-serif);
    font-size: 0.92rem;
    line-height: 1.7;
    color: var(--text-secondary);
  }
  .context-placeholder {
    color: var(--text-muted);
    font-size: 0.85rem;
    display: flex;
    justify-content: center;
  }

  .tafsir-body {
    min-height: 120px;
  }
  .loading-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 32px 20px;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .error-state {
    padding: 24px 20px;
    color: var(--danger, #c33);
    font-size: 0.9rem;
  }
</style>
