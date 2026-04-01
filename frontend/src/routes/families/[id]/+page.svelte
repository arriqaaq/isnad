<script lang="ts">
  import { page } from '$app/state';
  import { getFamily, getMatnDiff } from '$lib/api';
  import type { FamilyDetailResponse, ApiMatnDiff } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: FamilyDetailResponse | null = $state(null);
  let loading = $state(true);
  let activeTab: 'variants' | 'analysis' | 'diff' = $state('variants');
  let diffResult: ApiMatnDiff | null = $state(null);
  let diffA = $state('');
  let diffB = $state('');
  let diffLoading = $state(false);

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true;
    getFamily(id)
      .then((d) => { data = d; })
      .catch((e) => console.error('Failed to load family:', e))
      .finally(() => { loading = false; });
  });

  function confidenceColor(outcome: string): string {
    switch (outcome) {
      case 'supported': return 'success';
      case 'contested': return 'warning';
      case 'uncertain': return 'accent';
      default: return '';
    }
  }

  async function runDiff() {
    if (!diffA || !diffB || diffA === diffB) return;
    diffLoading = true;
    try {
      diffResult = await getMatnDiff(diffA, diffB);
    } catch (e) {
      console.error('Diff failed:', e);
    } finally {
      diffLoading = false;
    }
  }
</script>

<div class="family-view">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="view-header">
      <h1>{data.family.family_label ?? 'Hadith Family'}</h1>
      <div class="badges">
        <Badge text="{data.hadiths.length} variants" variant="accent" />
        {#if data.analysis.length > 0}
          <Badge text="{data.analysis.length} candidates" variant="success" />
        {/if}
      </div>
    </div>

    <div class="tabs">
      <button type="button" class="tab" class:active={activeTab === 'variants'} onclick={() => { activeTab = 'variants'; }}>Variants ({data.hadiths.length})</button>
      <button type="button" class="tab" class:active={activeTab === 'analysis'} onclick={() => { activeTab = 'analysis'; }}>Analysis</button>
      <button type="button" class="tab" class:active={activeTab === 'diff'} onclick={() => { activeTab = 'diff'; }}>Matn Diff</button>
    </div>

    <div class="tab-content">
      {#if activeTab === 'variants'}
        <div class="hadith-list">
          {#each data.hadiths as hadith (hadith.id)}
            <HadithCard {hadith} />
          {/each}
        </div>
      {:else if activeTab === 'analysis'}
        {#if data.analysis.length === 0}
          <div class="empty">
            <p>No CL/PCL analysis results yet.</p>
            <p class="hint">Run <code>hadith analyze --cl-pcl</code> after computing families.</p>
          </div>
        {:else}
          <div class="analysis-table">
            <table>
              <thead>
                <tr>
                  <th>Rank</th>
                  <th>Narrator</th>
                  <th>Type</th>
                  <th>Outcome</th>
                  <th>Confidence</th>
                  <th>Fan-out</th>
                  <th>Coverage</th>
                  <th>Diversity</th>
                </tr>
              </thead>
              <tbody>
                {#each data.analysis as c}
                  <tr>
                    <td class="rank">#{c.rank}</td>
                    <td><a href="/narrators/{c.narrator_id}">{c.narrator_id}</a></td>
                    <td><Badge text={c.candidate_type} variant={c.candidate_type === 'CL' ? 'success' : 'accent'} /></td>
                    <td><Badge text={c.outcome} variant={confidenceColor(c.outcome)} /></td>
                    <td class="mono">{c.final_confidence.toFixed(4)}</td>
                    <td>{c.fan_out}</td>
                    <td>{c.bundle_coverage.toFixed(2)}</td>
                    <td>{c.collector_diversity}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          {#if data.juynboll}
            <div class="juynboll-section">
              <h3>Transmission Integrity</h3>
              <div class="juynboll-grid">
                <div class="juynboll-card" class:positive={data.juynboll.has_reliable_bypass}>
                  <div class="label">Independent Paths</div>
                  <div class="value">{data.juynboll.has_reliable_bypass ? 'Yes' : 'No'}</div>
                  {#if data.juynboll.reliable_bypass_count > 0}
                    <div class="detail">{data.juynboll.reliable_bypass_count} path(s) through reliable narrators bypassing the convergence point</div>
                  {:else}
                    <div class="detail">No independent transmission paths detected through reliable narrators</div>
                  {/if}
                </div>
                <div class="juynboll-card" class:positive={data.juynboll.has_independent_cls}>
                  <div class="label">Independent Convergence</div>
                  <div class="value">{data.juynboll.has_independent_cls ? 'Yes' : 'No'}</div>
                  {#if data.juynboll.independent_cl_pairs > 0}
                    <div class="detail">{data.juynboll.independent_cl_pairs} unlinked convergence pair(s) among {data.juynboll.cl_count} CLs</div>
                  {:else}
                    <div class="detail">{data.juynboll.cl_count} convergence point(s) — all in same transmission chain</div>
                  {/if}
                </div>
                <div class="juynboll-card">
                  <div class="label">Chain Reliability</div>
                  <div class="value">{(data.juynboll.upstream_reliable_ratio * 100).toFixed(1)}%</div>
                  <div class="detail">Reliable narrators above convergence point, {data.juynboll.upstream_branching_points} branching point(s)</div>
                </div>
              </div>
            </div>
          {/if}
        {/if}
      {:else if activeTab === 'diff'}
        <div class="diff-controls">
          <label>
            <span>Hadith A</span>
            <select bind:value={diffA}>
              <option value="">Select...</option>
              {#each data.hadiths as h}
                <option value={h.id}>#{h.hadith_number} — {h.book_name ?? ''}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Hadith B</span>
            <select bind:value={diffB}>
              <option value="">Select...</option>
              {#each data.hadiths as h}
                <option value={h.id}>#{h.hadith_number} — {h.book_name ?? ''}</option>
              {/each}
            </select>
          </label>
          <button class="diff-btn" onclick={runDiff} disabled={!diffA || !diffB || diffA === diffB || diffLoading}>
            {diffLoading ? 'Computing...' : 'Compare'}
          </button>
        </div>
        {#if diffResult}
          <div class="diff-result">
            <div class="diff-similarity">Similarity: {(diffResult.similarity_ratio * 100).toFixed(1)}%</div>
            <div class="diff-panels">
              <div class="diff-panel">
                <h4>Hadith A</h4>
                <div class="diff-text" dir="rtl">
                  {#each diffResult.segments_a as seg}
                    <span class="seg seg-{seg.kind.toLowerCase()}">{seg.text} </span>
                  {/each}
                </div>
              </div>
              <div class="diff-panel">
                <h4>Hadith B</h4>
                <div class="diff-text" dir="rtl">
                  {#each diffResult.segments_b as seg}
                    <span class="seg seg-{seg.kind.toLowerCase()}">{seg.text} </span>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        {/if}
      {/if}
    </div>
  {:else}
    <div class="empty">Family not found.</div>
  {/if}
</div>

<style>
  .family-view { padding: 24px; max-width: 1100px; }
  .view-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px; }
  .badges { display: flex; gap: 8px; }
  .tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); margin-bottom: 20px; }
  .tab { padding: 10px 16px; font-size: 0.85rem; color: var(--text-secondary); border-bottom: 2px solid transparent; margin-bottom: -1px; cursor: pointer; }
  .tab:hover { color: var(--text-primary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .hadith-list { display: flex; flex-direction: column; gap: 12px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
  .hint { font-size: 0.85rem; }
  code { background: var(--bg-surface); padding: 2px 6px; border-radius: 4px; }

  /* Analysis table */
  .analysis-table { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
  th { text-align: left; padding: 10px 12px; border-bottom: 2px solid var(--border); color: var(--text-secondary); font-size: 0.8rem; text-transform: uppercase; }
  td { padding: 10px 12px; border-bottom: 1px solid var(--border); }
  td.rank { font-weight: 600; color: var(--accent); }
  td.mono { font-family: monospace; }
  td a { color: var(--accent); }
  td a:hover { text-decoration: underline; }

  /* Diff */
  .diff-controls { display: flex; gap: 12px; align-items: flex-end; margin-bottom: 20px; flex-wrap: wrap; }
  .diff-controls label { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 200px; }
  .diff-controls span { font-size: 0.8rem; color: var(--text-secondary); }
  .diff-controls select { padding: 8px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-primary); color: var(--text-primary); }
  .diff-btn { padding: 8px 20px; background: var(--accent); color: white; border: none; border-radius: var(--radius); cursor: pointer; white-space: nowrap; }
  .diff-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .diff-similarity { font-size: 0.9rem; color: var(--accent); margin-bottom: 12px; font-weight: 600; }
  .diff-panels { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .diff-panel { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 16px; }
  .diff-panel h4 { margin-bottom: 10px; color: var(--text-secondary); font-size: 0.85rem; }
  .diff-text { line-height: 2; font-size: 1.1rem; }
  .seg-unchanged { color: var(--text-primary); }
  .seg-missing { color: var(--error); background: var(--bg-hover); border-radius: 2px; padding: 1px 2px; }
  .seg-added { color: var(--success); background: var(--bg-hover); border-radius: 2px; padding: 1px 2px; }

  /* Juynboll falsifiability */
  .juynboll-section { margin-top: 24px; padding-top: 20px; border-top: 1px solid var(--border); }
  .juynboll-section h3 { font-size: 0.95rem; color: var(--text-secondary); margin-bottom: 12px; }
  .juynboll-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 12px; }
  .juynboll-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px; }
  .juynboll-card.positive { border-color: var(--success); background: var(--bg-hover); }
  .juynboll-card .label { font-size: 0.8rem; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 4px; }
  .juynboll-card .value { font-size: 1.2rem; font-weight: 600; }
  .juynboll-card.positive .value { color: var(--success); }
  .juynboll-card .detail { font-size: 0.8rem; color: var(--text-muted); margin-top: 4px; }

  @media (max-width: 768px) {
    .diff-panels { grid-template-columns: 1fr; }
  }
</style>
