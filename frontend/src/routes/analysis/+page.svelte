<script lang="ts">
  import { getAnalysisStats, getJuynbollSummary } from '$lib/api';
  import type { AnalysisStatsResponse, JuynbollSummaryResponse } from '$lib/types';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let stats: AnalysisStatsResponse | null = $state(null);
  let juynboll: JuynbollSummaryResponse | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    Promise.all([
      getAnalysisStats(),
      getJuynbollSummary().catch(() => null),
    ])
      .then(([s, j]) => { stats = s; juynboll = j; })
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
          <li>Run transmission analysis: <code>hadith analyze --juynboll</code></li>
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

    {#if juynboll && juynboll.families_analyzed > 0}
      <div class="transmission-section">
        <h3>Transmission Integrity</h3>
        <p class="section-desc">Structural tests evaluating whether transmission networks show evidence of independent, reliable narration — countering claims that convergence points indicate fabrication.</p>

        <div class="stats-grid stats-grid-3">
          <div class="stat-card">
            <div class="stat-value">{juynboll.families_analyzed}</div>
            <div class="stat-label">Families Analyzed</div>
          </div>
          <div class="stat-card" class:positive={juynboll.families_with_reliable_bypass > 0}>
            <div class="stat-value">{juynboll.families_with_reliable_bypass}</div>
            <div class="stat-label">Independent Paths</div>
            {#if juynboll.families_analyzed > 0}
              <div class="stat-detail">{(juynboll.families_with_reliable_bypass * 100 / juynboll.families_analyzed).toFixed(0)}% of families</div>
            {/if}
          </div>
          <div class="stat-card" class:positive={juynboll.families_with_independent_cls > 0}>
            <div class="stat-value">{juynboll.families_with_independent_cls}</div>
            <div class="stat-label">Independent Convergence</div>
            <div class="stat-detail">Families with unlinked CL pairs</div>
          </div>
        </div>

        {#if juynboll.cross_family_narrators.length > 0}
          <div class="cross-family">
            <h4>Cross-Family Convergence Points</h4>
            <p class="section-desc">Narrators who are convergence points (CLs) in multiple hadith families. High classical reliability combined with broad teaching activity is consistent with genuine transmission, not fabrication.</p>
            <table>
              <thead>
                <tr>
                  <th>Narrator</th>
                  <th>Families</th>
                  <th>Reliability</th>
                </tr>
              </thead>
              <tbody>
                {#each juynboll.cross_family_narrators as n}
                  <tr>
                    <td><a href="/narrators/{n.narrator_id}">{n.narrator_id}</a></td>
                    <td>{n.cl_family_count}</td>
                    <td>
                      {#if n.reliability_rating}
                        <Badge text={n.reliability_rating} variant={n.reliability_rating === 'thiqah' ? 'success' : n.reliability_rating === 'saduq' ? 'accent' : 'default'} />
                      {:else}
                        <span class="unknown">unknown</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    {/if}

    <div class="methodology">
      <h3>Methodology</h3>
      <p>This system uses graph-theoretic analysis to identify Common Link (CL) and Partial Common Link (PCL) narrators — key convergence points in hadith transmission networks. The structural analysis technique originates from Juynboll's framework, but this tool explicitly rejects his fabrication assumption. See the <a href="https://github.com">full methodology</a> for detailed scholarly context.</p>
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
  .stats-grid-3 { grid-template-columns: repeat(3, 1fr); }
  .stat-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; text-align: center; }
  .stat-card.accent { border-color: var(--accent); }
  .stat-card.success { border-color: #22c55e; }
  .stat-card.positive { border-color: #22c55e; background: rgba(34,197,94,0.05); }
  .stat-value { font-size: 2rem; font-weight: 700; color: var(--text-primary); }
  .stat-card.accent .stat-value { color: var(--accent); }
  .stat-card.success .stat-value, .stat-card.positive .stat-value { color: #22c55e; }
  .stat-label { font-size: 0.8rem; color: var(--text-secondary); margin-top: 4px; text-transform: uppercase; letter-spacing: 0.5px; }
  .stat-detail { font-size: 0.75rem; color: var(--text-muted); margin-top: 4px; }
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

  /* Transmission integrity */
  .transmission-section { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 24px; margin-bottom: 24px; }
  .transmission-section h3 { margin-bottom: 8px; }
  .section-desc { color: var(--text-secondary); font-size: 0.85rem; line-height: 1.5; margin-bottom: 16px; }
  .cross-family { margin-top: 20px; }
  .cross-family h4 { font-size: 0.9rem; color: var(--text-secondary); margin-bottom: 8px; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; margin-top: 12px; }
  th { text-align: left; padding: 8px 12px; border-bottom: 2px solid var(--border); color: var(--text-secondary); font-size: 0.8rem; text-transform: uppercase; }
  td { padding: 8px 12px; border-bottom: 1px solid var(--border); }
  td a { color: var(--accent); }
  td a:hover { text-decoration: underline; }
  .unknown { color: var(--text-muted); font-style: italic; font-size: 0.85rem; }

  .methodology { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 24px; }
  .methodology h3 { margin-bottom: 12px; }
  .methodology p { color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6; margin-bottom: 12px; }
  .methodology a { color: var(--accent); }
  .outcome-legend { display: flex; flex-wrap: wrap; gap: 16px; margin-top: 16px; }
  .outcome-item { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-secondary); }
</style>
