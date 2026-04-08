<script lang="ts">
  import { getMustalahStats } from '$lib/api';
  import type { MustalahStatsResponse } from '$lib/types';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let stats: MustalahStatsResponse | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    getMustalahStats()
      .then((s) => { stats = s; })
      .catch((e) => console.error('Failed to load analysis stats:', e))
      .finally(() => { loading = false; });
  });
</script>

<div class="analysis-page">
  <h1>Isnad Analysis</h1>
  <p class="subtitle">Mustalah al-hadith analysis of transmission chains</p>

  {#if loading}
    <LoadingSpinner />
  {:else if stats}
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-value">{stats.family_count}</div>
        <div class="stat-label">Hadith Families</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.analyzed_count}</div>
        <div class="stat-label">Analyzed</div>
      </div>
      <div class="stat-card success">
        <div class="stat-value">{stats.sahih_count}</div>
        <div class="stat-label">Sahih</div>
      </div>
      <div class="stat-card accent">
        <div class="stat-value">{stats.hasan_count}</div>
        <div class="stat-label">Hasan</div>
      </div>
      <div class="stat-card warning">
        <div class="stat-value">{stats.daif_count}</div>
        <div class="stat-label">Da'eef</div>
      </div>
    </div>

    <div class="stats-grid stats-grid-2">
      <div class="stat-card">
        <div class="stat-value">{stats.mutawatir_count}</div>
        <div class="stat-label">Mutawatir</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.mashhur_count}</div>
        <div class="stat-label">Mashhur</div>
      </div>
    </div>

    {#if stats.analyzed_count === 0}
      <div class="instructions">
        <h3>Getting Started</h3>
        <ol>
          <li>Ingest hadith data: <code>make hadith-ingest</code></li>
          <li>Compute families: <code>hadith analyze --families</code></li>
          <li>Run mustalah analysis: <code>hadith analyze --mustalah</code></li>
          <li>View results on the <a href="/families">Families</a> page</li>
        </ol>
      </div>
    {:else}
      <div class="links">
        <a href="/families" class="link-card">
          <span class="link-icon">&#x2B22;</span>
          <div>
            <div class="link-title">Browse Families</div>
            <div class="link-desc">View hadith families and their isnad analysis</div>
          </div>
        </a>
      </div>
    {/if}

    <div class="methodology">
      <h3>Methodology</h3>
      <p>This tool uses proper mustalah al-hadith (hadith terminology) to assess transmission chains. Each chain is evaluated for continuity (muttasil/munqati'/mursal), narrator quality ('adaalah and dabt), and graded according to traditional criteria (sahih/hasan/da'eef).</p>
      <p>Families are classified by transmission breadth (mutawatir/mashhur/'aziz/gharib) and assessed for corroboration (mutaba'at and shawahid). Composite grades follow the textbook definitions from at-Tahhaan's <em>Tayseer Mustalah al-Hadeeth</em>.</p>
      <div class="outcome-legend">
        <div class="outcome-item"><Badge text="Sahih" variant="success" /> Connected chain, all narrators thiqah, no defects</div>
        <div class="outcome-item"><Badge text="Hasan" variant="accent" /> Connected chain, narrators at saduq level</div>
        <div class="outcome-item"><Badge text="Da'eef" variant="warning" /> Broken chain or weak narrator</div>
      </div>
    </div>
  {/if}
</div>

<style>
  .analysis-page { padding: 24px; max-width: 900px; }
  .subtitle { color: var(--text-muted); font-size: 0.9rem; margin-bottom: 24px; }
  .stats-grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 12px; margin-bottom: 16px; }
  .stats-grid-2 { grid-template-columns: repeat(2, 1fr); margin-bottom: 32px; }
  .stat-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; text-align: center; }
  .stat-card.accent { border-color: var(--accent); }
  .stat-card.success { border-color: #22c55e; }
  .stat-card.warning { border-color: var(--warning); }
  .stat-value { font-size: 2rem; font-weight: 700; color: var(--text-primary); }
  .stat-card.accent .stat-value { color: var(--accent); }
  .stat-card.success .stat-value { color: #22c55e; }
  .stat-card.warning .stat-value { color: var(--warning); }
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
  .methodology a { color: var(--accent); }
  .outcome-legend { display: flex; flex-wrap: wrap; gap: 16px; margin-top: 16px; }
  .outcome-item { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-secondary); }
  @media (max-width: 768px) { .stats-grid { grid-template-columns: repeat(2, 1fr); } }
</style>
