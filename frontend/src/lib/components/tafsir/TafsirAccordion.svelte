<script lang="ts">
  import type { AllTafsirsEntry, InlineEnglishTafsir } from '$lib/types';
  import { convertPageToHtml } from '$lib/utils';

  let { entries, english }: {
    entries: AllTafsirsEntry[];
    english: InlineEnglishTafsir;
  } = $props();

  // Keys for open-state tracking. `english` uses a fixed key; turath entries
  // use their book_id.
  const ENGLISH_KEY = 'inline-en';

  // Default expand: first Arabic entry (or English if no Arabic).
  let openSet: Set<string> = $state(new Set());

  $effect(() => {
    const initial = new Set<string>();
    if (entries.length > 0) initial.add(String(entries[0].book_id));
    else if (english.body) initial.add(ENGLISH_KEY);
    openSet = initial;
  });

  function toggle(key: string) {
    const next = new Set(openSet);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    openSet = next;
  }

  function expandAll() {
    const all = new Set<string>();
    for (const e of entries) all.add(String(e.book_id));
    if (english.body) all.add(ENGLISH_KEY);
    openSet = all;
  }

  function collapseAll() {
    openSet = new Set();
  }

  const totalCount = $derived(entries.length + (english.body ? 1 : 0));
</script>

{#if totalCount === 0}
  <div class="empty-state">No tafsir available for this ayah yet.</div>
{:else}
  <div class="accordion-toolbar">
    <span class="toolbar-count">{totalCount} {totalCount === 1 ? 'source' : 'sources'}</span>
    <div class="toolbar-actions">
      <button class="toolbar-btn" type="button" onclick={expandAll}>Expand all</button>
      <button class="toolbar-btn" type="button" onclick={collapseAll}>Collapse all</button>
    </div>
  </div>

  <ul class="accordion">
    {#each entries as entry (entry.book_id)}
      {@const key = String(entry.book_id)}
      {@const isOpen = openSet.has(key)}
      <li class="accordion-item" class:open={isOpen}>
        <button
          class="accordion-header"
          type="button"
          aria-expanded={isOpen}
          onclick={() => toggle(key)}
        >
          <span class="chevron" aria-hidden="true">{isOpen ? '▾' : '▸'}</span>
          <span class="title-group">
            <span class="title-en">{entry.name_en}</span>
            <span class="title-ar" dir="rtl">{entry.name_ar}</span>
          </span>
          <span class="page-chip">Vol {entry.vol} · Page {entry.page_num}</span>
        </button>
        {#if isOpen}
          <div class="accordion-body">
            {#if entry.heading}
              <div class="body-heading" dir="rtl">{entry.heading}</div>
            {/if}
            <article class="body-text" dir="rtl">
              {@html convertPageToHtml(entry.text)}
            </article>
            <div class="body-footer">
              <a
                class="reader-link"
                href="/tafsir/{entry.book_id}?page={entry.page_index}"
              >Open in full reader →</a>
            </div>
          </div>
        {/if}
      </li>
    {/each}

    {#if english.body}
      {@const isOpen = openSet.has(ENGLISH_KEY)}
      <li class="accordion-item" class:open={isOpen}>
        <button
          class="accordion-header"
          type="button"
          aria-expanded={isOpen}
          onclick={() => toggle(ENGLISH_KEY)}
        >
          <span class="chevron" aria-hidden="true">{isOpen ? '▾' : '▸'}</span>
          <span class="title-group">
            <span class="title-en">Ibn Kathir (English)</span>
            <span class="title-ar">QUL translation</span>
          </span>
          <span class="page-chip">Inline</span>
        </button>
        {#if isOpen}
          <div class="accordion-body">
            <div class="body-text english">
              {@html english.body}
            </div>
          </div>
        {/if}
      </li>
    {/if}
  </ul>
{/if}

<style>
  .empty-state {
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .accordion-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-secondary);
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .toolbar-actions { display: flex; gap: 8px; }
  .toolbar-btn {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-secondary);
    padding: 3px 10px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all var(--transition);
  }
  .toolbar-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .accordion {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .accordion-item {
    border-bottom: 1px solid var(--border-subtle);
  }
  .accordion-header {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 14px 20px;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    font: inherit;
    color: var(--text-primary);
    transition: background var(--transition);
  }
  .accordion-header:hover {
    background: var(--bg-secondary);
  }
  .chevron {
    color: var(--text-muted);
    font-size: 0.85rem;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }
  .title-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .title-en {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .title-ar {
    font-size: 0.8rem;
    color: var(--text-muted);
    font-family: var(--font-arabic-text), serif;
  }
  .page-chip {
    font-size: 0.72rem;
    color: var(--accent);
    background: var(--accent-muted);
    padding: 2px 8px;
    border-radius: 10px;
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .accordion-body {
    padding: 4px 20px 20px 20px;
  }
  .body-heading {
    direction: rtl;
    text-align: right;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 10px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .body-text {
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
    font-size: 1.1rem;
    line-height: 2.1;
    color: var(--text-primary);
  }
  .body-text.english {
    direction: ltr;
    text-align: left;
    font-family: var(--font-serif);
    font-size: 0.92rem;
    line-height: 1.75;
    color: var(--text-secondary);
  }
  .body-text :global(span[data-type="title"]) {
    display: block;
    font-size: 1.3rem;
    font-weight: 700;
    text-align: center;
    margin: 1.2rem 0;
    color: var(--text-primary);
  }
  .body-text :global(.block) {
    margin-bottom: 0.5rem;
  }
  .body-text :global(.footnotes) {
    font-size: 0.9rem;
    color: var(--text-muted);
    margin-top: 1rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-subtle);
    line-height: 1.8;
  }
  .body-text.english :global(p) { margin: 8px 0; }
  .body-text.english :global(h2.title) {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 16px 0 8px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 4px;
  }
  .body-text.english :global(div.text_uthmani) {
    direction: rtl;
    text-align: right;
    font-family: var(--font-arabic-text), serif;
    font-size: 1.1rem;
    color: var(--text-primary);
    margin: 8px 0;
    padding: 8px;
    background: var(--bg-surface);
    border-radius: var(--radius-sm);
  }

  .body-footer {
    margin-top: 16px;
    padding-top: 10px;
    border-top: 1px dashed var(--border-subtle);
    text-align: left;
    direction: ltr;
  }
  .reader-link {
    font-size: 0.8rem;
    color: var(--accent);
    text-decoration: none;
  }
  .reader-link:hover { text-decoration: underline; }
</style>
