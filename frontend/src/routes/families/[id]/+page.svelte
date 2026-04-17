<script lang="ts">
  import { page } from '$app/state';
  import { getFamily, getMatnDiff, getMustalahFamily, getNarratorAssessments } from '$lib/api';
  import type { FamilyDetailResponse, ApiMatnDiff, MustalahFamilyResponse, NarratorAssessment } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import GlossaryTooltip from '$lib/components/hadith/GlossaryTooltip.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import DiffViewer from '$lib/components/hadith/DiffViewer.svelte';

  let data: FamilyDetailResponse | null = $state(null);
  let mustalah: MustalahFamilyResponse | null = $state(null);
  let loading = $state(true);
  let activeTab: 'variants' | 'analysis' | 'diff' = $state('variants');
  let diffResult: ApiMatnDiff | null = $state(null);
  let diffA = $state('');
  let diffB = $state('');
  let diffLoading = $state(false);

  // Cache for narrator assessments (narrator_id → assessments[])
  let narratorAssessments: Record<string, NarratorAssessment[]> = $state({});
  let expandedChains: Set<number> = $state(new Set());

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

  /** Map analysis values to glossary term IDs */
  function glossaryId(value: string | null): string | null {
    if (!value) return null;
    const map: Record<string, string> = {
      mutawatir: 'mutawatir', mashhur: 'mashhur', aziz: 'aziz', gharib: 'gharib',
      muttasil: 'muttasil', munqati: 'munqati', mursal: 'mursal',
      muallaq: 'muallaq', mudal: 'mudal',
    };
    return map[value] ?? null;
  }

  async function toggleChain(idx: number, narratorIds: string[] | null) {
    if (expandedChains.has(idx)) {
      expandedChains = new Set([...expandedChains].filter(i => i !== idx));
      return;
    }
    expandedChains = new Set([...expandedChains, idx]);
    // Fetch assessments for any narrators we haven't loaded yet
    if (narratorIds) {
      for (const nid of narratorIds) {
        if (!(nid in narratorAssessments)) {
          try {
            const resp = await getNarratorAssessments(nid);
            narratorAssessments = { ...narratorAssessments, [nid]: resp.assessments };
          } catch {
            narratorAssessments = { ...narratorAssessments, [nid]: [] };
          }
        }
      }
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
        {#if mustalah?.analysis?.breadth_class}
          <Badge text={breadthLabel(mustalah.analysis.breadth_class)} variant="default" />
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
            <p>No structural analysis results yet.</p>
            <p class="hint">Run <code>hadith analyze --mustalah</code> after computing families.</p>
          </div>
        {:else}
          {@const a = mustalah.analysis}
          <!-- Stats Grid -->
          <div class="mustalah-grid">
            <div class="m-card">
              <div class="label">Transmission Breadth</div>
              <div class="value">{#if glossaryId(a.breadth_class)}<GlossaryTooltip termId={glossaryId(a.breadth_class) ?? ''}>{breadthLabel(a.breadth_class)}</GlossaryTooltip>{:else}{breadthLabel(a.breadth_class)}{/if}</div>
              <div class="detail">Min {a.min_breadth} narrator(s) at tabaqah {a.bottleneck_tabaqah ?? '?'}</div>
            </div>
            <div class="m-card">
              <div class="label">Corroboration</div>
              <div class="value">{a.mutabaat_count} / {a.shawahid_count}</div>
              <div class="detail">Mutaba'at / Shawahid</div>
            </div>
            <div class="m-card">
              <div class="label">Sahabah</div>
              <div class="value">{a.sahabi_count}</div>
              <div class="detail">Distinct companion narrator(s)</div>
            </div>
            <div class="m-card">
              <div class="label">Chains</div>
              <div class="value">{a.chain_count}</div>
              <div class="detail">Transmission chain(s)</div>
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

          <!-- Chain Assessments -->
          {#if mustalah.chains.length > 0}
            <div class="section-header">
              <h3>Chain Assessments</h3>
              <p class="section-hint">Click a chain to view narrator scholarly assessments</p>
            </div>
            {#each mustalah.chains as c, idx}
              <div class="chain-card">
                <button class="chain-header" onclick={() => toggleChain(idx, c.narrator_ids)}>
                  <div class="chain-info">
                    <a href="/hadiths/{c.variant_id}" onclick={(e: MouseEvent) => e.stopPropagation()}>{c.variant_id}</a>
                    <span class="chain-meta">
                      {#if glossaryId(c.continuity)}<GlossaryTooltip termId={glossaryId(c.continuity) ?? ''}><Badge text={c.continuity} variant={c.continuity === 'muttasil' ? 'success' : 'warning'} /></GlossaryTooltip>{:else}<Badge text={c.continuity} variant={c.continuity === 'muttasil' ? 'success' : 'warning'} />{/if}
                      <span class="narrator-count">{c.narrator_count} narrators</span>
                      {#if c.has_chronology_conflict}<Badge text="chronology issue" variant="warning" />{/if}
                    </span>
                  </div>
                  <span class="expand-icon">{expandedChains.has(idx) ? '▾' : '▸'}</span>
                </button>
                {#if expandedChains.has(idx) && c.narrator_ids}
                  <div class="chain-narrators">
                    <table>
                      <thead>
                        <tr>
                          <th>#</th>
                          <th>Narrator</th>
                          <th>Scholarly Assessments</th>
                        </tr>
                      </thead>
                      <tbody>
                        {#each c.narrator_ids as nid, nIdx}
                          <tr>
                            <td class="pos">{nIdx + 1}</td>
                            <td><a href="/narrators/{nid}">{nid}</a></td>
                            <td class="assessments-cell">
                              {#if narratorAssessments[nid]}
                                {#if narratorAssessments[nid].length === 0}
                                  <span class="no-data">No scholarly assessment</span>
                                {:else}
                                  {#each narratorAssessments[nid] as ev}
                                    <span class="assessment">
                                      <span class="scholar-name">{ev.scholar}:</span>
                                      <span class="citation-text" dir="rtl">{ev.citation_text}</span>
                                    </span>
                                  {/each}
                                {/if}
                              {:else}
                                <span class="loading-text">Loading...</span>
                              {/if}
                            </td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                {/if}
              </div>
            {/each}
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
          <DiffViewer result={diffResult} />
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
  td.mono { font-family: monospace; }
  td.pos { width: 30px; color: var(--text-muted); }
  td a { color: var(--accent); }
  td a:hover { text-decoration: underline; }

  /* Diff */
  .diff-controls { display: flex; gap: 12px; align-items: flex-end; margin-bottom: 20px; flex-wrap: wrap; }
  .diff-controls label { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 200px; }
  .diff-controls span { font-size: 0.8rem; color: var(--text-secondary); }
  .diff-controls select { padding: 8px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-primary); color: var(--text-primary); }
  .diff-btn { padding: 8px 20px; background: var(--accent); color: white; border: none; border-radius: var(--radius); cursor: pointer; white-space: nowrap; }
  .diff-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* Mustalah analysis */
  .mustalah-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; margin-bottom: 20px; }
  .m-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px; }
  .m-card .label { font-size: 0.75rem; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 4px; }
  .m-card .value { font-size: 1.1rem; font-weight: 600; }
  .m-card .detail { font-size: 0.78rem; color: var(--text-muted); margin-top: 4px; }
  .section-header { margin-top: 20px; margin-bottom: 10px; }
  .section-header h3 { font-size: 0.95rem; color: var(--text-secondary); }
  .section-hint { font-size: 0.8rem; color: var(--text-muted); margin-top: 4px; }
  .ilal-section { margin-top: 16px; margin-bottom: 16px; padding: 12px; background: color-mix(in srgb, var(--warning) 8%, transparent); border: 1px solid var(--warning); border-radius: var(--radius); }
  .ilal-section h3 { font-size: 0.85rem; color: var(--warning); margin-bottom: 8px; }
  .ilal-section ul { margin: 0; padding-left: 20px; font-size: 0.85rem; color: var(--text-secondary); }
  .ilal-section li { margin-bottom: 4px; }

  /* Chain cards */
  .chain-card { border: 1px solid var(--border); border-radius: var(--radius); margin-bottom: 8px; overflow: hidden; }
  .chain-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; background: var(--bg-surface); cursor: pointer; width: 100%; border: none; color: var(--text-primary); }
  .chain-header:hover { background: var(--bg-hover); }
  .chain-info { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .chain-info a { color: var(--accent); font-weight: 600; }
  .chain-meta { display: flex; align-items: center; gap: 8px; }
  .narrator-count { font-size: 0.8rem; color: var(--text-muted); }
  .expand-icon { font-size: 0.8rem; color: var(--text-muted); }
  .chain-narrators { border-top: 1px solid var(--border); }
  .chain-narrators table { font-size: 0.85rem; }
  .chain-narrators th { font-size: 0.75rem; }
  .assessments-cell { display: flex; flex-wrap: wrap; gap: 8px; }
  .assessment { display: inline-flex; align-items: baseline; gap: 4px; padding: 2px 8px; background: var(--bg-primary); border-radius: var(--radius); font-size: 0.82rem; }
  .scholar-name { color: var(--text-secondary); font-size: 0.75rem; white-space: nowrap; }
  .citation-text { color: var(--text-primary); font-family: 'Amiri', serif; }
  .no-data { color: var(--text-muted); font-size: 0.8rem; font-style: italic; }
  .loading-text { color: var(--text-muted); font-size: 0.8rem; }

  @media (max-width: 768px) {
  }
</style>
