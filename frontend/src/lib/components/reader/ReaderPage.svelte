<script lang="ts">
  import type { BookPage } from '$lib/types';
  import { convertPageToHtml } from '$lib/utils';

  let { page }: { page: BookPage } = $props();

  let html = $derived(convertPageToHtml(page.text));
</script>

<article class="reader-page" dir="rtl">
  {@html html}
  <p class="page-label">{page.vol} / {page.page_num}</p>
</article>

<style>
  .reader-page {
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
    font-size: 1.35rem;
    line-height: 2.5;
    color: var(--text-primary);
    padding: 1.75rem 0;
    border-bottom: 1px solid var(--border-subtle);
    text-align: right;
  }

  /* Title spans from turath (chapter/section headings) */
  .reader-page :global(span[data-type="title"]) {
    display: block;
    font-size: 1.6rem;
    font-weight: 700;
    text-align: center;
    margin: 2rem 0;
    line-height: 1.8;
    color: var(--text-primary);
  }

  /* Block-level divisions from convertPageToHtml */
  .reader-page :global(.block) {
    margin-bottom: 0.5rem;
  }

  /* Footnotes section */
  .reader-page :global(.footnotes) {
    font-size: 0.9rem;
    color: var(--text-muted);
    margin-top: 1rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-subtle);
    line-height: 1.8;
  }

  /* Quran verse brackets styling */
  .reader-page :global(span) {
    color: var(--text-primary);
  }

  .page-label {
    text-align: center;
    font-family: var(--font-sans);
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 1.5rem;
    direction: ltr;
  }

  @media (max-width: 640px) {
    .reader-page {
      font-size: 1.15rem;
      line-height: 2.2;
      padding: 1.25rem 0;
    }
    .reader-page :global(span[data-type="title"]) {
      font-size: 1.3rem;
    }
  }
</style>
