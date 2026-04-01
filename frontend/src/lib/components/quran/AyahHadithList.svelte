<script lang="ts">
  import type { AyahHadithResponse } from '$lib/types';
  import { truncate } from '$lib/utils';

  let { data }: { data: AyahHadithResponse } = $props();
</script>

{#if data.curated.length > 0}
  <div class="section">
    <div class="section-label">Referenced Hadiths</div>
    {#each data.curated as hadith}
      <div class="hadith-item">
        <div class="hadith-meta">
          <span class="book-name">{hadith.book_name ?? 'Unknown'}</span>
          <span class="hadith-num">#{hadith.hadith_number}</span>
          {#if hadith.grade}
            <span class="grade-badge">{hadith.grade}</span>
          {/if}
          <a href="/hadiths/{hadith.id}" class="detail-link">View</a>
        </div>
        {#if hadith.text_en}
          <div class="hadith-text">{truncate(hadith.text_en, 300)}</div>
        {:else if hadith.matn}
          <div class="hadith-text arabic" dir="rtl">{truncate(hadith.matn, 200)}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

{#if data.related && data.related.length > 0}
  <div class="section">
    <div class="section-label">Related Hadiths</div>
    {#each data.related as hadith}
      <div class="hadith-item">
        <div class="hadith-meta">
          <span class="hadith-num">#{hadith.hadith_number}</span>
          {#if hadith.score}
            <span class="score">{hadith.score.toFixed(3)}</span>
          {/if}
          <a href="/hadiths/{hadith.id}" class="detail-link">View</a>
        </div>
        {#if hadith.text_en}
          <div class="hadith-text">{truncate(hadith.text_en, 200)}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

{#if data.curated.length === 0 && (!data.related || data.related.length === 0)}
  <div class="empty">No hadiths found for this verse.</div>
{/if}

<style>
  .section {
    margin-bottom: 16px;
  }
  .section:last-child {
    margin-bottom: 0;
  }
  .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--success);
    margin-bottom: 8px;
  }
  .hadith-item {
    padding: 10px 12px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    margin-bottom: 8px;
  }
  .hadith-item:last-child {
    margin-bottom: 0;
  }
  .hadith-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }
  .book-name {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .hadith-num {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .grade-badge {
    font-size: 0.65rem;
    padding: 1px 6px;
    border-radius: 8px;
    background: var(--success);
    color: white;
  }
  .score {
    font-size: 0.7rem;
    color: var(--success);
    font-family: var(--font-mono);
  }
  .detail-link {
    margin-left: auto;
    font-size: 0.7rem;
    color: var(--accent);
  }
  .detail-link:hover {
    text-decoration: underline;
  }
  .hadith-text {
    font-size: 0.8rem;
    line-height: 1.6;
    color: var(--text-secondary);
  }
  .hadith-text.arabic {
    font-size: 0.9rem;
    line-height: 1.8;
  }
  .empty {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
