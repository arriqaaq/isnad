<script lang="ts">
  import { getAnalysisStats } from '$lib/api';
  import type { AnalysisStatsResponse } from '$lib/types';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let stats: AnalysisStatsResponse | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    getAnalysisStats()
      .then((s) => { stats = s; })
      .catch((e) => console.error('Failed to load analysis stats:', e))
      .finally(() => { loading = false; });
  });
</script>

<div class="analysis-page">
  <h1>Transmission Analysis</h1>
  <p class="subtitle">CL/PCL analysis of hadith transmission chains</p>

  {#if loading}
    <LoadingSpinner />
  {:else if stats}
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-value">{stats.family_count}</div>
        <div class="stat-label">Hadith Families</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.candidate_count}</div>
        <div class="stat-label">CL/PCL Candidates</div>
      </div>
      <div class="stat-card accent">
        <div class="stat-value">{stats.cl_count}</div>
        <div class="stat-label">Common Links</div>
      </div>
      <div class="stat-card success">
        <div class="stat-value">{stats.supported_count}</div>
        <div class="stat-label">Supported</div>
      </div>
    </div>

    {#if stats.family_count === 0}
      <div class="instructions">
        <h3>Getting Started</h3>
        <ol>
          <li>Ingest hadith data: <code>hadith ingest</code></li>
          <li>Compute families: <code>hadith analyze --families</code></li>
          <li>View results on the <a href="/families">Families</a> page</li>
        </ol>
      </div>
    {:else}
      <div class="links">
        <a href="/families" class="link-card">
          <span class="link-icon">⬢</span>
          <div>
            <div class="link-title">Browse Families</div>
            <div class="link-desc">View hadith families and their variants</div>
          </div>
        </a>
      </div>
    {/if}

    <div class="methodology">
      <h3>Methodology</h3>
      <p>This system implements Juynboll's Common Link (CL) and Partial Common Link (PCL) methodology for identifying key transmission narrators in hadith isnad chains.</p>
      <p>For each narrator in a hadith family, 8 structural features are computed (fan-out, bundle coverage, collector diversity, pre-single-strand ratio, bypass ratio, chronology conflicts, matn coherence, provenance completeness) and combined using a weighted scoring formula.</p>
      <div class="outcome-legend">
        <div class="outcome-item"><Badge text="Supported" variant="success" /> Confidence >= 0.75</div>
        <div class="outcome-item"><Badge text="Contested" variant="warning" /> Confidence 0.55 - 0.75</div>
        <div class="outcome-item"><Badge text="Uncertain" variant="accent" /> Confidence 0.35 - 0.55</div>
        <div class="outcome-item"><Badge text="Likely Weak" /> Confidence &lt; 0.35</div>
      </div>
    </div>
  {/if}
</div>

<style>
  .analysis-page { padding: 24px; max-width: 900px; }
  .subtitle { color: var(--text-muted); font-size: 0.9rem; margin-bottom: 24px; }
  .stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-bottom: 32px; }
  .stat-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; text-align: center; }
  .stat-card.accent { border-color: var(--accent); }
  .stat-card.success { border-color: #22c55e; }
  .stat-value { font-size: 2rem; font-weight: 700; color: var(--text-primary); }
  .stat-card.accent .stat-value { color: var(--accent); }
  .stat-card.success .stat-value { color: #22c55e; }
  .stat-label { font-size: 0.8rem; color: var(--text-secondary); margin-top: 4px; text-transform: uppercase; letter-spacing: 0.5px; }
  .instructions { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 24px; margin-bottom: 24px; }
  .instructions h3 { margin-bottom: 12px; }
  .instructions ol { padding-left: 20px; }
  .instructions li { margin-bottom: 8px; color: var(--text-secondary); }
  code { background: var(--bg-primary); padding: 2px 6px; border-radius: 4px; font-size: 0.85rem; }
  .links { display: flex; gap: 12px; margin-bottom: 32px; }
  .link-card { display: flex; align-items: center; gap: 16px; padding: 20px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); flex: 1; transition: all var(--transition); }
  .link-card:hover { border-color: var(--accent); }
  .link-icon { font-size: 1.5rem; }
  .link-title { font-weight: 600; margin-bottom: 4px; }
  .link-desc { font-size: 0.85rem; color: var(--text-secondary); }
  .methodology { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 24px; }
  .methodology h3 { margin-bottom: 12px; }
  .methodology p { color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6; margin-bottom: 12px; }
  .outcome-legend { display: flex; flex-wrap: wrap; gap: 16px; margin-top: 16px; }
  .outcome-item { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-secondary); }
</style>
