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
      <div class="similar-item">
        <a href="/quran/{sim.ayah_key.split(':')[0]}?ayah={sim.ayah_key.split(':')[1]}" class="similar-link">
          {sim.ayah_key}
        </a>
        <span class="score-badge">{sim.score}</span>
        <span class="coverage-pct">{sim.coverage}%</span>
      </div>
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
  .similar-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
  }
  .similar-item:last-child {
    border-bottom: none;
  }
  .similar-link {
    font-size: 0.8rem;
    font-family: var(--font-mono);
    color: var(--accent);
    text-decoration: none;
  }
  .similar-link:hover {
    text-decoration: underline;
  }
  .score-badge {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--success);
    padding: 1px 6px;
    background: var(--bg-surface);
    border-radius: 8px;
  }
  .coverage-pct {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .empty {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
