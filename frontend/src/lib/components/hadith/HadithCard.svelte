<script lang="ts">
  import type { ApiHadith, SharhPageRef } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import Badge from '$lib/components/common/Badge.svelte';
  import { language } from '$lib/stores/language';

  let { hadith, sharhPage, onopensharh }: {
    hadith: ApiHadith;
    sharhPage?: SharhPageRef;
    onopensharh?: (info: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number }) => void;
  } = $props();
</script>

<div class="hadith-card-wrapper">
  <a href="/hadiths/{hadith.id}" class="hadith-card">
    <div class="card-header">
      {#if hadith.book_name}
        <Badge text={hadith.book_name} variant="accent" />
      {:else}
        <Badge text="Book {hadith.book_id}" />
      {/if}
      <span class="hadith-num mono">#{hadith.hadith_number}</span>
    </div>

    {#if hadith.narrator_text}
      <p class="narrator">{hadith.narrator_text}</p>
    {/if}

    {#if $language === 'en' && hadith.text_en}
      <p class="text-preview">{truncate(stripHtml(hadith.text_en), 180)}</p>
    {:else if hadith.text_ar}
      <p class="text-ar arabic" dir="rtl">{truncate(hadith.text_ar, 150)}</p>
    {:else if hadith.text_en}
      <p class="text-preview">{truncate(stripHtml(hadith.text_en), 180)}</p>
    {/if}
  </a>

  {#if sharhPage && onopensharh}
    <div class="card-actions">
      <button
        class="sharh-btn"
        onclick={() => onopensharh({ bookId: sharhPage.sharh_book_id, pageIndex: sharhPage.page_index, bookName: sharhPage.book_name, hadithNumber: hadith.hadith_number })}
        title="View {sharhPage.book_name}"
      >
        Sharh
      </button>
    </div>
  {/if}
</div>

<style>
  .hadith-card-wrapper {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: all var(--transition);
    overflow: hidden;
  }
  .hadith-card-wrapper:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .hadith-card {
    display: block;
    padding: 16px;
    color: var(--text-primary);
    text-decoration: none;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .hadith-num {
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  .narrator {
    color: var(--accent);
    font-size: 0.85rem;
    margin-bottom: 8px;
    font-weight: 500;
  }

  .text-preview {
    font-family: var(--font-serif);
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.6;
    margin-bottom: 8px;
  }

  .text-ar {
    color: var(--text-secondary);
    font-family: var(--font-arabic-text);
    font-size: 0.95rem;
    opacity: 0.8;
  }

  .card-actions {
    padding: 0 16px 10px;
    display: flex;
    gap: 8px;
  }

  .sharh-btn {
    font-size: 0.75rem;
    color: var(--accent);
    background: var(--bg-primary);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 3px 12px;
    cursor: pointer;
    transition: all var(--transition);
    font-weight: 500;
  }
  .sharh-btn:hover {
    background: var(--accent-muted);
  }
</style>
