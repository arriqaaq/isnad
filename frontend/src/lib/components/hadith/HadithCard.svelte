<script lang="ts">
  import type { ApiHadith } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import Badge from '$lib/components/common/Badge.svelte';
  import { language } from '$lib/stores/language';

  let { hadith }: { hadith: ApiHadith } = $props();
</script>

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

<style>
  .hadith-card {
    display: block;
    padding: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: all var(--transition);
    color: var(--text-primary);
  }

  .hadith-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    color: var(--text-primary);
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
    color: var(--text-secondary);
    font-size: 0.85rem;
    line-height: 1.5;
    margin-bottom: 8px;
  }

  .text-ar {
    color: var(--text-secondary);
    font-size: 0.95rem;
    opacity: 0.8;
  }
</style>
