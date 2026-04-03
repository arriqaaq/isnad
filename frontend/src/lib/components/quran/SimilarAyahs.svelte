<script lang="ts">
  import type { AyahSimilarResponse } from '$lib/types';

  let { data }: { data: AyahSimilarResponse } = $props();

  let expandedPhrases: Set<string> = $state(new Set());

  function togglePhrase(id: string) {
    const next = new Set(expandedPhrases);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedPhrases = next;
  }
</script>

{#if data.phrases.length > 0}
  <div class="section">
    <div class="section-label">Shared Phrases (Mutashabihat)</div>
    <div class="phrase-list">
      {#each data.phrases as phrase}
        <div class="phrase-item">
          <button class="phrase-chip" onclick={() => togglePhrase(phrase.id)}>
            <span class="phrase-ar" dir="rtl">{phrase.text_ar}</span>
            <span class="phrase-meta">{phrase.ayah_keys.length + 1} ayahs &middot; {phrase.occurrence}x</span>
          </button>
          {#if expandedPhrases.has(phrase.id)}
            <div class="phrase-ayahs">
              {#each phrase.ayah_keys as key}
                <a href="/quran/{key.split(':')[0]}?ayah={key.split(':')[1]}" class="ayah-link">{key}</a>
              {/each}
              <a href="/quran/phrases/{phrase.id}" class="detail-link">View all</a>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

{#if data.similar.length > 0}
  <div class="section">
    <div class="section-label">Similar Verses</div>
    {#each data.similar as sim}
      <a href="/quran/{sim.ayah_key.split(':')[0]}?ayah={sim.ayah_key.split(':')[1]}" class="similar-card">
        <div class="similar-header">
          <span class="similar-ref">{sim.ayah_key}</span>
          <span class="score-badge">Score {sim.score}</span>
          <span class="coverage-pct">{sim.coverage}% match</span>
        </div>
        {#if sim.text_ar}
          <div class="similar-ar" dir="rtl">{sim.text_ar}</div>
        {/if}
        {#if sim.text_en}
          <div class="similar-en">{sim.text_en.length > 150 ? sim.text_en.slice(0, 150) + '...' : sim.text_en}</div>
        {/if}
      </a>
    {/each}
  </div>
{/if}

{#if data.phrases.length === 0 && data.similar.length === 0}
  <div class="empty">No similar verses or shared phrases found.</div>
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
    color: var(--accent);
    margin-bottom: 8px;
  }
  .phrase-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .phrase-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .phrase-chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition);
    text-align: left;
  }
  .phrase-chip:hover {
    border-color: var(--accent);
    background: var(--accent-muted);
  }
  .phrase-ar {
    font-size: 1rem;
    color: var(--text-primary);
    line-height: 1.6;
  }
  .phrase-meta {
    font-size: 0.65rem;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .phrase-ayahs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 0 4px 12px;
  }
  .ayah-link {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    color: var(--accent);
    padding: 2px 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    text-decoration: none;
  }
  .ayah-link:hover {
    background: var(--accent-muted);
    border-color: var(--accent);
  }
  .detail-link {
    font-size: 0.7rem;
    color: var(--text-muted);
    text-decoration: underline;
    padding: 2px 8px;
  }
  .similar-card {
    display: block;
    padding: 10px 12px;
    margin-bottom: 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-decoration: none;
    transition: all var(--transition);
  }
  .similar-card:hover {
    border-color: var(--accent);
    background: var(--accent-muted);
  }
  .similar-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .similar-ref {
    font-size: 0.8rem;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--accent);
  }
  .score-badge {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--success);
    padding: 1px 6px;
    background: var(--bg-hover);
    border-radius: 8px;
  }
  .coverage-pct {
    font-size: 0.65rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .similar-ar {
    font-size: 0.95rem;
    color: var(--text-primary);
    line-height: 1.8;
    margin-bottom: 4px;
  }
  .similar-en {
    font-size: 0.8rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .empty {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
