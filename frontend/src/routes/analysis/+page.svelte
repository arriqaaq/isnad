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
  <p class="subtitle">Structural analysis of transmission chains with scholarly narrator assessments</p>

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
      <div class="stat-card">
        <div class="stat-value">{stats.evidence_count}</div>
        <div class="stat-label">Scholar Assessments</div>
      </div>
    </div>

    <h3>Transmission Breadth</h3>
    <div class="stats-grid stats-grid-4">
      <div class="stat-card">
        <div class="stat-value">{stats.mutawatir_count}</div>
        <div class="stat-label">Mutawatir</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.mashhur_count}</div>
        <div class="stat-label">Mashhur</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.aziz_count}</div>
        <div class="stat-label">'Aziz</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.gharib_count}</div>
        <div class="stat-label">Gharib</div>
      </div>
    </div>

    {#if stats.analyzed_count === 0}
      <div class="instructions">
        <h3>Getting Started</h3>
        <ol>
          <li>Ingest hadith data: <code>make hadith-ingest</code></li>
          <li>Compute families: <code>hadith analyze --families</code></li>
          <li>Run structural analysis: <code>hadith analyze --mustalah</code></li>
          <li>View results on the <a href="/families">Families</a> page</li>
        </ol>
      </div>
    {:else}
      <div class="links">
        <a href="/families" class="link-card">
          <span class="link-icon">&#x2B22;</span>
          <div>
            <div class="link-title">Browse Families</div>
            <div class="link-desc">View hadith families and their chain analysis</div>
          </div>
        </a>
      </div>
    {/if}

    <div class="methodology">
      <h3>Methodology</h3>
      <p>This tool displays <strong>structural analysis</strong> of transmission chains and <strong>scholarly assessments</strong> of narrators from classical rijal works. No algorithmic grades are computed &mdash; only observable facts about the chain and what scholars actually said.</p>
      <p>Each chain is analyzed for continuity (muttasil/munqati'/mursal). Families are classified by transmission breadth (mutawatir/mashhur/'aziz/gharib). Corroboration counts (mutaba'at and shawahid) show how many independent paths exist.</p>
      <p>Narrator assessments are sourced from:</p>
      <div class="outcome-legend">
        <div class="outcome-item"><Badge text="Taqrib" variant="accent" /> Ibn Hajar al-Asqalani, <em>Taqrib al-Tahdhib</em></div>
        <div class="outcome-item"><Badge text="Mizan" variant="default" /> al-Dhahabi, <em>Mizan al-I'tidal</em></div>
      </div>
    </div>
  {/if}
</div>

<style>
  .analysis-page { padding: 24px; max-width: 900px; }
  .subtitle { color: var(--text-muted); font-size: 0.9rem; margin-bottom: 24px; }
  h3 { margin-bottom: 12px; }
  .stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-bottom: 16px; }
  .stats-grid-4 { grid-template-columns: repeat(4, 1fr); margin-bottom: 32px; }
  .stat-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; text-align: center; }
  .stat-value { font-size: 2rem; font-weight: 700; color: var(--text-primary); }
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
  .methodology strong { color: var(--text-primary); }
  .methodology a { color: var(--accent); }
  .outcome-legend { display: flex; flex-wrap: wrap; gap: 16px; margin-top: 16px; }
  .outcome-item { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-secondary); }
  @media (max-width: 768px) { .stats-grid { grid-template-columns: repeat(2, 1fr); } .stats-grid-4 { grid-template-columns: repeat(2, 1fr); } }
</style>
