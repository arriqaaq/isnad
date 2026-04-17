<script lang="ts">
  import type { ApiMatnDiff } from '$lib/types';

  let { result }: { result: ApiMatnDiff } = $props();
</script>

<div class="diff-result">
  <div class="diff-similarity">Similarity: {(result.similarity_ratio * 100).toFixed(1)}%</div>
  <div class="diff-panels">
    <div class="diff-panel">
      <h4>Hadith A</h4>
      <div class="diff-text" dir="rtl">
        {#each result.segments_a as seg}
          <span class="seg seg-{seg.kind.toLowerCase()}">{seg.text} </span>
        {/each}
      </div>
    </div>
    <div class="diff-panel">
      <h4>Hadith B</h4>
      <div class="diff-text" dir="rtl">
        {#each result.segments_b as seg}
          <span class="seg seg-{seg.kind.toLowerCase()}">{seg.text} </span>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .diff-similarity { font-size: 0.9rem; color: var(--accent); margin-bottom: 12px; font-weight: 600; }
  .diff-panels { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .diff-panel { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 16px; }
  .diff-panel h4 { margin-bottom: 10px; color: var(--text-secondary); font-size: 0.85rem; }
  .diff-text { line-height: 2; font-size: 1.1rem; }
  .seg-unchanged { color: var(--text-primary); }
  .seg-missing { color: var(--error); background: var(--bg-hover); border-radius: 2px; padding: 1px 2px; }
  .seg-added { color: var(--success); background: var(--bg-hover); border-radius: 2px; padding: 1px 2px; }

  @media (max-width: 768px) {
    .diff-panels { grid-template-columns: 1fr; }
  }
</style>
