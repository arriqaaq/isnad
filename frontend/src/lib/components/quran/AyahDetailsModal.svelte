<script lang="ts">
  import type { ApiAyah, ApiAyahSearchResult, CCManuscript, AyahHadithResponse, AyahSimilarResponse } from '$lib/types';
  import { getAyahManuscripts, getAyahHadiths, getAyahSimilar } from '$lib/api';
  import ManuscriptCard from './ManuscriptCard.svelte';
  import AyahHadithList from './AyahHadithList.svelte';
  import SimilarAyahs from './SimilarAyahs.svelte';
  import { onMount } from 'svelte';

  let { ayah, onclose }: {
    ayah: ApiAyah | ApiAyahSearchResult;
    onclose: () => void;
  } = $props();

  // Local font size for panel content only (not global)
  let panelFontSize = $state(0.9);
  const PANEL_FONT_STEPS = [0.75, 0.85, 0.9, 1.0, 1.1, 1.25, 1.4];
  function stepPanelFont(dir: 1 | -1) {
    const idx = PANEL_FONT_STEPS.indexOf(panelFontSize);
    const nearest = idx === -1
      ? PANEL_FONT_STEPS.reduce((a, b) => Math.abs(b - panelFontSize) < Math.abs(a - panelFontSize) ? b : a)
      : panelFontSize;
    const nearIdx = PANEL_FONT_STEPS.indexOf(nearest);
    const next = Math.max(0, Math.min(nearIdx + dir, PANEL_FONT_STEPS.length - 1));
    panelFontSize = PANEL_FONT_STEPS[next];
  }

  let manuscripts: CCManuscript[] = $state([]);
  let manuscriptsLoading = $state(true);
  let hadithData: AyahHadithResponse | null = $state(null);
  let hadithLoading = $state(true);
  let similarData: AyahSimilarResponse | null = $state(null);
  let similarLoading = $state(true);

  type Tab = 'tafsir' | 'similar' | 'hadith' | 'manuscripts';
  let activeTab: Tab = $state('tafsir');

  // Panel position & size state (centered like BookViewerModal)
  let panelW = $state(Math.min(700, window.innerWidth - 40));
  let panelH = $state(Math.min(window.innerHeight * 0.85, window.innerHeight - 40));
  // svelte-ignore state_referenced_locally — intentional: captures initial size for centering
  let panelX = $state(Math.max(20, (window.innerWidth - panelW) / 2));
  // svelte-ignore state_referenced_locally
  let panelY = $state(Math.max(20, (window.innerHeight - panelH) / 2));

  // Drag state
  let dragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let dragStartPanelX = 0;
  let dragStartPanelY = 0;

  // Resize state
  let resizing = $state(false);
  let resizeEdge = '';
  let resizeStartX = 0;
  let resizeStartY = 0;
  let resizeStartW = 0;
  let resizeStartH = 0;
  let resizeStartPanelX = 0;
  let resizeStartPanelY = 0;

  onMount(() => {
    getAyahManuscripts(ayah.surah_number, ayah.ayah_number)
      .then(data => { manuscripts = data; })
      .catch(() => {})
      .finally(() => { manuscriptsLoading = false; });

    getAyahHadiths(ayah.surah_number, ayah.ayah_number, true)
      .then(data => { hadithData = data; })
      .catch(() => {})
      .finally(() => { hadithLoading = false; });

    getAyahSimilar(ayah.surah_number, ayah.ayah_number)
      .then(data => { similarData = data; })
      .catch(() => {})
      .finally(() => { similarLoading = false; });

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  });

  function startDrag(e: MouseEvent) {
    dragging = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    dragStartPanelX = panelX;
    dragStartPanelY = panelY;
    e.preventDefault();
  }

  function startResize(e: MouseEvent, edge: string) {
    resizing = true;
    resizeEdge = edge;
    resizeStartX = e.clientX;
    resizeStartY = e.clientY;
    resizeStartW = panelW;
    resizeStartH = panelH;
    resizeStartPanelX = panelX;
    resizeStartPanelY = panelY;
    e.preventDefault();
    e.stopPropagation();
  }

  function handleMouseMove(e: MouseEvent) {
    if (dragging) {
      panelX = Math.max(0, Math.min(window.innerWidth - 100, dragStartPanelX + e.clientX - dragStartX));
      panelY = Math.max(0, Math.min(window.innerHeight - 50, dragStartPanelY + e.clientY - dragStartY));
    } else if (resizing) {
      const dx = e.clientX - resizeStartX;
      const dy = e.clientY - resizeStartY;

      if (resizeEdge.includes('w')) {
        const newW = Math.max(320, resizeStartW - dx);
        panelX = resizeStartPanelX + (resizeStartW - newW);
        panelW = newW;
      }
      if (resizeEdge.includes('e')) {
        panelW = Math.max(320, resizeStartW + dx);
      }
      if (resizeEdge.includes('n')) {
        const newH = Math.max(200, resizeStartH - dy);
        panelY = resizeStartPanelY + (resizeStartH - newH);
        panelH = newH;
      }
      if (resizeEdge.includes('s')) {
        panelH = Math.max(200, resizeStartH + dy);
      }
    }
  }

  function handleMouseUp() {
    dragging = false;
    resizing = false;
  }

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('panel-backdrop')) {
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel-backdrop" onclick={handleBackdrop}>
  <div
    class="side-panel"
    class:is-moving={dragging || resizing}
    style="left: {panelX}px; top: {panelY}px; width: {panelW}px; height: {panelH}px;"
  >
    <!-- Resize handles -->
    <div class="resize-handle resize-n" onmousedown={(e) => startResize(e, 'n')}></div>
    <div class="resize-handle resize-s" onmousedown={(e) => startResize(e, 's')}></div>
    <div class="resize-handle resize-w" onmousedown={(e) => startResize(e, 'w')}></div>
    <div class="resize-handle resize-e" onmousedown={(e) => startResize(e, 'e')}></div>
    <div class="resize-handle resize-nw" onmousedown={(e) => startResize(e, 'nw')}></div>
    <div class="resize-handle resize-ne" onmousedown={(e) => startResize(e, 'ne')}></div>
    <div class="resize-handle resize-sw" onmousedown={(e) => startResize(e, 'sw')}></div>
    <div class="resize-handle resize-se" onmousedown={(e) => startResize(e, 'se')}></div>

    <div class="panel-header" onmousedown={startDrag}>
      <span class="panel-ref">{ayah.surah_number}:{ayah.ayah_number}</span>
      <div class="header-actions">
        <span class="drag-hint">⠿</span>
        <button class="panel-close" onclick={onclose}>&times;</button>
      </div>
    </div>

    <div class="font-bar">
      <span class="font-bar-label">Text size</span>
      <button class="font-btn" onclick={() => stepPanelFont(-1)}>A−</button>
      <button class="font-btn" onclick={() => stepPanelFont(1)}>A+</button>
    </div>

    <div class="panel-tabs">
      <button class="tab-btn" class:active={activeTab === 'tafsir'} onclick={() => activeTab = 'tafsir'}>
        Tafsir
      </button>
      <button class="tab-btn" class:active={activeTab === 'similar'} onclick={() => activeTab = 'similar'}>
        Similar Ayahs
      </button>
      <button class="tab-btn" class:active={activeTab === 'hadith'} onclick={() => activeTab = 'hadith'}>
        Hadith
      </button>
      <button class="tab-btn" class:active={activeTab === 'manuscripts'} onclick={() => activeTab = 'manuscripts'}>
        Manuscripts
      </button>
    </div>

    <div class="panel-content" style="zoom: {panelFontSize / 0.9}">
      {#if activeTab === 'tafsir'}
        <section class="panel-section">
          {#if ayah.tafsir_en}
            <div class="tafsir-content">{@html ayah.tafsir_en}</div>
          {:else}
            <div class="section-empty">No tafsir available for this ayah.</div>
          {/if}
        </section>
      {:else if activeTab === 'similar'}
        <section class="panel-section">
          {#if similarLoading}
            <div class="section-loading">Loading similar ayahs...</div>
          {:else if similarData && (similarData.similar.length > 0 || similarData.phrases.length > 0)}
            <SimilarAyahs data={similarData} />
          {:else}
            <div class="section-empty">No similar ayahs found.</div>
          {/if}
        </section>
      {:else if activeTab === 'hadith'}
        <section class="panel-section">
          {#if hadithLoading}
            <div class="section-loading">Loading hadiths...</div>
          {:else if hadithData && (hadithData.curated.length > 0 || (hadithData.related && hadithData.related.length > 0))}
            <AyahHadithList data={hadithData} />
          {:else}
            <div class="section-empty">No related hadiths found.</div>
          {/if}
        </section>
      {:else if activeTab === 'manuscripts'}
        <section class="panel-section">
          {#if manuscriptsLoading}
            <div class="section-loading">Loading manuscripts...</div>
          {:else if manuscripts.length > 0}
            <div class="manuscript-grid">
              {#each manuscripts as ms}
                <ManuscriptCard manuscript={ms} />
              {/each}
            </div>
          {:else}
            <div class="section-empty">No manuscripts found for this verse.</div>
          {/if}
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .panel-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 200;
    animation: fadeIn 0.15s ease-out;
  }

  .side-panel {
    position: fixed;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.25);
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease-out;
    min-width: 450px;
    min-height: 350px;
    overflow: hidden;
  }

  .side-panel.is-moving {
    user-select: none;
    transition: none;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    cursor: grab;
  }
  .panel-header:active {
    cursor: grabbing;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }


  .drag-hint {
    color: var(--text-muted);
    font-size: 1rem;
    opacity: 0.5;
  }

  .panel-ref {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }

  .panel-close {
    font-size: 1.5rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }
  .panel-close:hover {
    color: var(--text-primary);
  }

  .font-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 16px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }
  .font-bar-label {
    font-size: 0.7rem;
    color: var(--text-muted);
    margin-right: auto;
  }
  .font-btn {
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition);
  }
  .font-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .panel-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
    padding: 0 8px;
    gap: 4px;
    overflow-x: auto;
  }

  .tab-btn {
    padding: 10px 14px;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all var(--transition);
    white-space: nowrap;
  }

  .tab-btn:hover {
    color: var(--text-secondary);
  }

  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .panel-section {
    margin-bottom: 28px;
  }

  .section-loading, .section-empty {
    font-size: 0.85rem;
    color: var(--text-muted);
    padding: 8px 0;
  }

  .manuscript-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .tafsir-content {
    font-family: var(--font-serif);
    font-size: 0.9rem;
    line-height: 1.8;
    color: var(--text-secondary);
  }

  .tafsir-content :global(h2.title) {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 16px 0 8px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 4px;
  }
  .tafsir-content :global(h2.title:first-child) {
    margin-top: 0;
  }
  .tafsir-content :global(p) {
    margin: 8px 0;
    line-height: 1.7;
  }
  .tafsir-content :global(div.text_uthmani) {
    font-size: 1.1rem;
    text-align: right;
    direction: rtl;
    color: var(--text-primary);
    margin: 8px 0;
    padding: 8px;
    background: var(--bg-surface);
    border-radius: var(--radius-sm);
  }

  /* Resize handles */
  .resize-handle {
    position: absolute;
    z-index: 10;
  }
  .resize-n { top: -3px; left: 8px; right: 8px; height: 6px; cursor: n-resize; }
  .resize-s { bottom: -3px; left: 8px; right: 8px; height: 6px; cursor: s-resize; }
  .resize-w { left: -3px; top: 8px; bottom: 8px; width: 6px; cursor: w-resize; }
  .resize-e { right: -3px; top: 8px; bottom: 8px; width: 6px; cursor: e-resize; }
  .resize-nw { top: -3px; left: -3px; width: 12px; height: 12px; cursor: nw-resize; }
  .resize-ne { top: -3px; right: -3px; width: 12px; height: 12px; cursor: ne-resize; }
  .resize-sw { bottom: -3px; left: -3px; width: 12px; height: 12px; cursor: sw-resize; }
  .resize-se { bottom: -3px; right: -3px; width: 12px; height: 12px; cursor: se-resize; }

  @keyframes slideUp {
    from { transform: translateY(16px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @media (max-width: 640px) {
    .side-panel {
      left: 0 !important;
      top: 0 !important;
      width: 100vw !important;
      height: 100vh !important;
      border-radius: 0;
    }
    .resize-handle { display: none; }
  }
</style>
