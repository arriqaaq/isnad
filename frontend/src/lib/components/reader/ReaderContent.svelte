<script lang="ts">
  import type { BookPage } from '$lib/types';
  import ReaderPage from './ReaderPage.svelte';

  let { pages, currentPageIndex = $bindable(0), onNeedMore }: {
    pages: Map<number, BookPage>;
    currentPageIndex: number;
    onNeedMore: (startIndex: number) => void;
    totalPages: number;
  } = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let pageElements: Map<number, HTMLElement> = new Map();

  // Window of pages to render around current position
  let windowSize = 40;
  let renderStart = $derived(Math.max(0, currentPageIndex - 10));

  let renderedIndices = $derived.by(() => {
    const indices: number[] = [];
    for (let i = renderStart; i < renderStart + windowSize; i++) {
      if (pages.has(i)) indices.push(i);
    }
    return indices;
  });

  // Track scroll position to update currentPageIndex
  function handleScroll() {
    if (!containerEl) return;
    const scrollTop = containerEl.scrollTop;
    const containerRect = containerEl.getBoundingClientRect();
    const mid = containerRect.top + containerRect.height / 3;

    // Find which page element is closest to viewport center
    let closest = currentPageIndex;
    let closestDist = Infinity;

    for (const [idx, el] of pageElements) {
      const rect = el.getBoundingClientRect();
      const dist = Math.abs(rect.top - mid);
      if (dist < closestDist) {
        closestDist = dist;
        closest = idx;
      }
    }
    currentPageIndex = closest;

    // Request more pages when nearing edges
    const maxLoaded = Math.max(...pages.keys(), 0);
    if (closest >= maxLoaded - 5) {
      onNeedMore(maxLoaded + 1);
    }

    // Also request pages before current if scrolling up
    const minLoaded = Math.min(...pages.keys(), 0);
    if (closest <= minLoaded + 5 && minLoaded > 0) {
      onNeedMore(Math.max(0, minLoaded - 20));
    }
  }

  export function scrollToPage(index: number) {
    // If page is already rendered, scroll to it
    const el = pageElements.get(index);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
      currentPageIndex = index;
      return;
    }

    // Otherwise, request the chunk and set current index
    // (the page will render after data loads, then we scroll)
    currentPageIndex = index;
    onNeedMore(Math.max(0, index - 5));

    // Wait for render then scroll
    requestAnimationFrame(() => {
      setTimeout(() => {
        const newEl = pageElements.get(index);
        if (newEl) {
          newEl.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
      }, 100);
    });
  }

  function registerPage(index: number, el: HTMLElement) {
    pageElements.set(index, el);
  }

  function unregisterPage(index: number) {
    pageElements.delete(index);
  }
</script>

<div class="reader-content" bind:this={containerEl} onscroll={handleScroll}>
  <div class="reader-inner">
    {#each renderedIndices as idx (idx)}
      {@const page = pages.get(idx)}
      {#if page}
        <div
          class="page-wrapper"
          data-page-index={idx}
          use:registerEl={{ index: idx, register: registerPage, unregister: unregisterPage }}
        >
          <ReaderPage {page} />
        </div>
      {/if}
    {/each}
  </div>
</div>

<!-- Svelte action for registering page elements -->
<script lang="ts" module>
  function registerEl(node: HTMLElement, params: { index: number; register: (i: number, el: HTMLElement) => void; unregister: (i: number) => void }) {
    params.register(params.index, node);
    return {
      destroy() {
        params.unregister(params.index);
      }
    };
  }
</script>

<style>
  .reader-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .reader-inner {
    max-width: 900px;
    margin: 0 auto;
    padding: 0 2rem;
  }
  .page-wrapper {
    scroll-margin-top: 60px;
  }

  @media (max-width: 640px) {
    .reader-inner {
      padding: 0 1rem;
    }
  }
</style>
