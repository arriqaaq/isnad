<script lang="ts">
  import { page } from '$app/state';
  import { getFamily, getMatnDiff, getMustalahFamily } from '$lib/api';
  import type { FamilyDetailResponse, ApiMatnDiff, MustalahFamilyResponse } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: FamilyDetailResponse | null = $state(null);
  let mustalah: MustalahFamilyResponse | null = $state(null);
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
    Promise.all([
      getFamily(id),
      getMustalahFamily(id).catch(() => null),
    ])
      .then(([d, m]) => { data = d; mustalah = m; })
      .catch((e) => console.error('Failed to load family:', e))
      .finally(() => { loading = false; });
  });

  function gradeColor(grade: string | null): 'success' | 'accent' | 'warning' | 'default' {
    if (!grade) return 'default';
    if (grade.startsWith('sahih')) return 'success';
    if (grade.startsWith('hasan')) return 'accent';
    if (grade.startsWith('daif') || grade === 'mawdu') return 'warning';
    return 'default';
  }

  function gradeLabel(grade: string | null): string {
    if (!grade) return '—';
    const labels: Record<string, string> = {
      sahih: 'Sahih',
      sahihlighayrihi: 'Sahih li-Ghayrihi',
      hasan: 'Hasan',
      hasanlighayrihi: 'Hasan li-Ghayrihi',
      daif: "Da'eef",
      daifjiddan: "Da'eef Jiddan",
      mawdu: "Mawdu'",
    };
    return labels[grade] ?? grade;
  }

  function breadthLabel(b: string | null): string {
    if (!b) return '—';
    const labels: Record<string, string> = {
      mutawatir: 'Mutawatir',
      mashhur: 'Mashhur',
      aziz: "'Aziz",
      gharib: 'Gharib',
    };
    return labels[b] ?? b;
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
        {#if mustalah?.analysis?.composite_grade}
          <Badge text={gradeLabel(mustalah.analysis.composite_grade)} variant={gradeColor(mustalah.analysis.composite_grade)} />
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
        {#if !mustalah?.analysis}
          <div class="empty">
            <p>No mustalah analysis results yet.</p>
            <p class="hint">Run <code>hadith analyze --mustalah</code> after computing families.</p>
          </div>
        {:else}
          {@const a = mustalah.analysis}
          <!-- Composite Grade Banner -->
          <div class="grade-banner grade-{a.composite_grade}">
            <div class="grade-label">Composite Grade</div>
            <div class="grade-value">{gradeLabel(a.composite_grade)}</div>
            <div class="grade-detail">Best chain: {gradeLabel(a.best_chain_grade)} &middot; {a.chain_count} chain(s)</div>
          </div>

          <!-- Stats Grid -->
          <div class="mustalah-grid">
            <div class="m-card">
              <div class="label">Transmission Breadth</div>
              <div class="value">{breadthLabel(a.breadth_class)}</div>
              <div class="detail">Min {a.min_breadth} narrator(s) at tabaqah {a.bottleneck_tabaqah ?? '?'}</div>
            </div>
            <div class="m-card">
              <div class="label">Corroboration</div>
              <div class="value">{a.corroboration_strength ?? 'None'}</div>
              <div class="detail">{a.mutabaat_count} mutaba'at, {a.shawahid_count} shawahid ({a.reliable_mutabaat_count} reliable)</div>
            </div>
            <div class="m-card">
              <div class="label">Chain Grades</div>
              <div class="value">{a.sahih_chain_count}S / {a.hasan_chain_count}H / {a.daif_chain_count}D</div>
              <div class="detail">Sahih / Hasan / Da'eef chains</div>
            </div>
            <div class="m-card">
              <div class="label">Sahabah</div>
              <div class="value">{a.sahabi_count}</div>
              <div class="detail">Distinct companion narrator(s)</div>
            </div>
          </div>

          <!-- Defect Flags -->
          {#if a.ilal_flags && a.ilal_flags.length > 0}
            <div class="ilal-section">
              <h3>'Ilal (Defect Flags)</h3>
              <ul>
                {#each a.ilal_flags as flag}
                  <li>{flag}</li>
                {/each}
              </ul>
            </div>
          {/if}

          <!-- Chain Assessments Table -->
          {#if mustalah.chains.length > 0}
            <div class="section-header">
              <h3>Chain Assessments</h3>
            </div>
            <div class="analysis-table">
              <table>
                <thead>
                  <tr>
                    <th>Variant</th>
                    <th>Continuity</th>
                    <th>Grade</th>
                    <th>Weakest Link</th>
                    <th>Narrators</th>
                  </tr>
                </thead>
                <tbody>
                  {#each mustalah.chains as c}
                    <tr>
                      <td><a href="/hadiths/{c.variant_id}">{c.variant_id}</a></td>
                      <td><Badge text={c.continuity} variant={c.continuity === 'muttasil' ? 'success' : 'warning'} /></td>
                      <td><Badge text={gradeLabel(c.chain_grade)} variant={gradeColor(c.chain_grade)} /></td>
                      <td>
                        {#if c.weakest_narrator_id}
                          <a href="/narrators/{c.weakest_narrator_id}">{c.weakest_rating ?? 'unknown'}</a>
                          {#if c.weakest_prior}<span class="mono">({c.weakest_prior.toFixed(2)})</span>{/if}
                        {:else}
                          —
                        {/if}
                      </td>
                      <td>{c.narrator_count}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}

          <!-- Pivot Narrators -->
          {#if mustalah.pivots.length > 0}
            <div class="section-header">
              <h3>Madar al-Isnad (Pivot Narrators)</h3>
            </div>
            <div class="analysis-table">
              <table>
                <thead>
                  <tr>
                    <th>Narrator</th>
                    <th>Coverage</th>
                    <th>Fan-out</th>
                    <th>Diversity</th>
                    <th>Bypass</th>
                    <th>Bottleneck</th>
                  </tr>
                </thead>
                <tbody>
                  {#each mustalah.pivots as p}
                    <tr>
                      <td><a href="/narrators/{p.narrator_id}">{p.narrator_id}</a></td>
                      <td class="mono">{(p.bundle_coverage ?? 0).toFixed(2)}</td>
                      <td>{p.fan_out}</td>
                      <td>{p.collector_diversity}</td>
                      <td>{p.bypass_count}</td>
                      <td>{#if p.is_bottleneck}<Badge text="gharabah" variant="warning" />{/if}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
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

  /* Mustalah analysis */
  .grade-banner { padding: 20px; border-radius: var(--radius); margin-bottom: 20px; text-align: center; border: 1px solid var(--border); }
  .grade-banner.grade-sahih, .grade-banner.grade-sahihlighayrihi { background: color-mix(in srgb, var(--success) 10%, transparent); border-color: var(--success); }
  .grade-banner.grade-hasan, .grade-banner.grade-hasanlighayrihi { background: color-mix(in srgb, var(--accent) 10%, transparent); border-color: var(--accent); }
  .grade-banner.grade-daif, .grade-banner.grade-daifjiddan, .grade-banner.grade-mawdu { background: color-mix(in srgb, var(--warning) 10%, transparent); border-color: var(--warning); }
  .grade-label { font-size: 0.8rem; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 4px; }
  .grade-value { font-size: 1.5rem; font-weight: 700; }
  .grade-detail { font-size: 0.85rem; color: var(--text-muted); margin-top: 4px; }
  .mustalah-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; margin-bottom: 20px; }
  .m-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px; }
  .m-card .label { font-size: 0.75rem; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 4px; }
  .m-card .value { font-size: 1.1rem; font-weight: 600; }
  .m-card .detail { font-size: 0.78rem; color: var(--text-muted); margin-top: 4px; }
  .section-header { margin-top: 20px; margin-bottom: 10px; }
  .section-header h3 { font-size: 0.95rem; color: var(--text-secondary); }
  .ilal-section { margin-top: 16px; padding: 12px; background: color-mix(in srgb, var(--warning) 8%, transparent); border: 1px solid var(--warning); border-radius: var(--radius); }
  .ilal-section h3 { font-size: 0.85rem; color: var(--warning); margin-bottom: 8px; }
  .ilal-section ul { margin: 0; padding-left: 20px; font-size: 0.85rem; color: var(--text-secondary); }
  .ilal-section li { margin-bottom: 4px; }

  @media (max-width: 768px) {
    .diff-panels { grid-template-columns: 1fr; }
  }
</style>
