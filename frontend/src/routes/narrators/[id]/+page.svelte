<script lang="ts">
  import { page } from '$app/state';
  import { getNarrator, getNarratorGraph, updateNarrator, getNarratorClStatus } from '$lib/api';
  import type { NarratorDetailResponse, GraphData, NarratorClStatus } from '$lib/types';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import GraphView from '$lib/components/graph/GraphView.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: NarratorDetailResponse | null = $state(null);
  let graphData: GraphData | null = $state(null);
  let clStatus: NarratorClStatus | null = $state(null);
  let loading = $state(true);
  let activeTab: 'network' | 'hadiths' | 'connections' | 'details' = $state('network');
  let saving = $state(false);
  let saveMsg = $state('');

  // Editable fields
  let editGender = $state('');
  let editGeneration = $state('');
  let editBio = $state('');
  let editKunya = $state('');
  let editBirthYear = $state('');
  let editBirthCalendar = $state('hijri');
  let editDeathYear = $state('');
  let editDeathCalendar = $state('hijri');
  let editLocations = $state('');
  let editTags = $state('');
  let editReliabilityRating = $state('');
  let editReliabilitySource = $state('');

  let id = $derived(page.params.id);

  function populateForm() {
    if (!data) return;
    const n = data.narrator;
    editGender = n.gender ?? '';
    editGeneration = n.generation ?? '';
    editBio = n.bio ?? '';
    editKunya = n.kunya ?? '';
    editBirthYear = n.birth_year?.toString() ?? '';
    editBirthCalendar = n.birth_calendar ?? 'hijri';
    editDeathYear = n.death_year?.toString() ?? '';
    editDeathCalendar = n.death_calendar ?? 'hijri';
    editLocations = n.locations?.join(', ') ?? '';
    editTags = n.tags?.join(', ') ?? '';
    editReliabilityRating = n.reliability_rating ?? '';
    editReliabilitySource = n.reliability_source ?? '';
  }

  $effect(() => {
    if (!id) return;
    loading = true;
    activeTab = 'network';
    Promise.all([getNarrator(id), getNarratorGraph(id), getNarratorClStatus(id).catch(() => null)])
      .then(([d, g, cls]) => {
        const seen = new Set<string>();
        d.hadiths = d.hadiths.filter(h => {
          if (seen.has(h.id)) return false;
          seen.add(h.id);
          return true;
        });
        data = d;
        graphData = g;
        clStatus = cls;
        populateForm();
      })
      .catch((e) => console.error('Failed to load narrator:', e))
      .finally(() => { loading = false; });
  });

  const RATINGS = ['', 'thiqah', 'saduq', 'majhul', 'daif', 'matruk', 'accused_fabrication'];

  function ratingColor(rating: string | null): string {
    if (!rating) return '';
    const map: Record<string, string> = {
      thiqah: 'success', saduq: 'accent', majhul: '', daif: 'warning', matruk: 'warning', accused_fabrication: 'warning'
    };
    return map[rating] ?? '';
  }

  async function handleSave() {
    if (!data) return;
    saving = true;
    saveMsg = '';
    const payload: Record<string, unknown> = {};

    if (editGender) payload.gender = editGender;
    if (editGeneration) payload.generation = editGeneration;
    if (editBio) payload.bio = editBio;
    if (editKunya) payload.kunya = editKunya;
    if (editBirthYear) payload.birth_year = parseInt(editBirthYear);
    if (editBirthCalendar) payload.birth_calendar = editBirthCalendar;
    if (editDeathYear) payload.death_year = parseInt(editDeathYear);
    if (editDeathCalendar) payload.death_calendar = editDeathCalendar;
    if (editLocations.trim()) payload.locations = editLocations.split(',').map(s => s.trim()).filter(Boolean);
    if (editTags.trim()) payload.tags = editTags.split(',').map(s => s.trim()).filter(Boolean);
    if (editReliabilityRating) payload.reliability_rating = editReliabilityRating;
    if (editReliabilitySource) payload.reliability_source = editReliabilitySource;

    // Map rating to prior
    const priorMap: Record<string, number> = { thiqah: 0.75, saduq: 0.65, majhul: 0.50, daif: 0.35, matruk: 0.20, accused_fabrication: 0.20 };
    if (editReliabilityRating && priorMap[editReliabilityRating] !== undefined) {
      payload.reliability_prior = priorMap[editReliabilityRating];
    }

    try {
      await updateNarrator(data.narrator.id, payload);
      saveMsg = 'Saved';
      // Refresh data
      const d = await getNarrator(id);
      const seen = new Set<string>();
      d.hadiths = d.hadiths.filter(h => { if (seen.has(h.id)) return false; seen.add(h.id); return true; });
      data = d;
      populateForm();
    } catch (e) {
      saveMsg = 'Error saving';
      console.error(e);
    } finally {
      saving = false;
      setTimeout(() => { saveMsg = ''; }, 3000);
    }
  }
</script>

<div class="narrator-view">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="view-header">
      <div>
        <h1>
          {data.narrator.name_ar || data.narrator.name_en || data.narrator.id}
          {#if data.narrator.kunya}
            <span class="kunya">({data.narrator.kunya})</span>
          {/if}
        </h1>
        {#if data.narrator.birth_year || data.narrator.death_year}
          <p class="dates">
            {#if data.narrator.birth_year}{data.narrator.birth_year}{/if}
            {#if data.narrator.birth_year && data.narrator.death_year}–{/if}
            {#if data.narrator.death_year}{data.narrator.death_year}{/if}
            {data.narrator.death_calendar === 'gregorian' ? 'CE' : 'AH'}
          </p>
        {/if}
      </div>
      <div class="badges">
        {#if data.narrator.reliability_rating}
          <Badge text={data.narrator.reliability_rating} variant={ratingColor(data.narrator.reliability_rating)} />
        {/if}
        {#if data.narrator.generation}
          <Badge text={data.narrator.generation} variant="accent" />
        {/if}
        {#if clStatus && clStatus.cl_family_count > 0}
          <Badge text="CL in {clStatus.cl_family_count} {clStatus.cl_family_count === 1 ? 'family' : 'families'}" variant="success" />
        {/if}
        {#if clStatus && clStatus.pcl_family_count > 0 && clStatus.cl_family_count === 0}
          <Badge text="PCL in {clStatus.pcl_family_count} {clStatus.pcl_family_count === 1 ? 'family' : 'families'}" variant="accent" />
        {/if}
        {#if data.narrator.gender}
          <Badge text={data.narrator.gender} />
        {/if}
      </div>
    </div>

    {#if data.narrator.locations && data.narrator.locations.length > 0}
      <div class="location-tags">
        {#each data.narrator.locations as loc}
          <span class="location-tag">{loc}</span>
        {/each}
      </div>
    {/if}

    {#if data.narrator.bio}
      <div class="bio truncated">{data.narrator.bio}</div>
    {/if}

    {#if data.narrator.reliability_source}
      <p class="reliability-source">Source: {data.narrator.reliability_source}</p>
    {/if}

    <div class="tabs">
      <button type="button" class="tab" class:active={activeTab === 'network'} onclick={() => { activeTab = 'network'; }}>Network</button>
      <button type="button" class="tab" class:active={activeTab === 'hadiths'} onclick={() => { activeTab = 'hadiths'; }}>Hadiths ({data.hadiths.length})</button>
      <button type="button" class="tab" class:active={activeTab === 'connections'} onclick={() => { activeTab = 'connections'; }}>Connections</button>
      <button type="button" class="tab" class:active={activeTab === 'details'} onclick={() => { activeTab = 'details'; }}>Details</button>
    </div>

    <div class="tab-content">
      {#if activeTab === 'network'}
        <GraphView data={graphData} />
      {:else if activeTab === 'hadiths'}
        <div class="hadith-list">
          {#each data.hadiths as hadith (hadith.id)}
            <HadithCard {hadith} />
          {/each}
          {#if data.hadiths.length === 0}
            <div class="empty">No hadiths linked to this narrator.</div>
          {/if}
        </div>
      {:else if activeTab === 'connections'}
        {#if data.teachers.length > 0}
          <div class="connection-group">
            <h3>Teachers (heard from)</h3>
            <div class="chips">{#each data.teachers as teacher}<NarratorChip narrator={teacher} />{/each}</div>
          </div>
        {/if}
        {#if data.students.length > 0}
          <div class="connection-group">
            <h3>Students (narrated to)</h3>
            <div class="chips">{#each data.students as student}<NarratorChip narrator={student} />{/each}</div>
          </div>
        {/if}
        {#if data.teachers.length === 0 && data.students.length === 0}
          <div class="empty">No connections found.</div>
        {/if}
      {:else if activeTab === 'details'}
        <form class="details-form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
          <div class="form-section">
            <h3>Classification</h3>
            <div class="form-row">
              <label>
                <span>Reliability Rating</span>
                <select bind:value={editReliabilityRating}>
                  {#each RATINGS as r}
                    <option value={r}>{r || '— not set —'}</option>
                  {/each}
                </select>
              </label>
              <label>
                <span>Source</span>
                <input type="text" bind:value={editReliabilitySource} placeholder="e.g., Taqrib al-Tahdhib" />
              </label>
            </div>
            <div class="form-row">
              <label>
                <span>Generation</span>
                <input type="text" bind:value={editGeneration} placeholder="e.g., Sahabi, Tabi'i" />
              </label>
              <label>
                <span>Gender</span>
                <input type="text" bind:value={editGender} placeholder="Male / Female" />
              </label>
            </div>
          </div>

          <div class="form-section">
            <h3>Biography</h3>
            <div class="form-row">
              <label>
                <span>Kunya</span>
                <input type="text" bind:value={editKunya} placeholder="e.g., Abu Huraira" />
              </label>
            </div>
            <div class="form-row">
              <label class="half">
                <span>Birth Year</span>
                <input type="number" bind:value={editBirthYear} placeholder="Year" />
              </label>
              <label class="quarter">
                <span>Calendar</span>
                <select bind:value={editBirthCalendar}>
                  <option value="hijri">Hijri</option>
                  <option value="gregorian">Gregorian</option>
                </select>
              </label>
              <label class="half">
                <span>Death Year</span>
                <input type="number" bind:value={editDeathYear} placeholder="Year" />
              </label>
              <label class="quarter">
                <span>Calendar</span>
                <select bind:value={editDeathCalendar}>
                  <option value="hijri">Hijri</option>
                  <option value="gregorian">Gregorian</option>
                </select>
              </label>
            </div>
            <label>
              <span>Locations (comma-separated)</span>
              <input type="text" bind:value={editLocations} placeholder="e.g., Madinah, Makkah, Kufa" />
            </label>
            <label>
              <span>Tags (comma-separated)</span>
              <input type="text" bind:value={editTags} placeholder="e.g., thiqah, hafiz, mujtahid" />
            </label>
            <label>
              <span>Bio</span>
              <textarea bind:value={editBio} rows="4" placeholder="Biographical notes..."></textarea>
            </label>
          </div>

          <div class="form-actions">
            <button type="submit" class="save-btn" disabled={saving}>
              {saving ? 'Saving...' : 'Save Changes'}
            </button>
            {#if saveMsg}
              <span class="save-msg" class:error={saveMsg === 'Error saving'}>{saveMsg}</span>
            {/if}
          </div>
        </form>
      {/if}
    </div>
  {:else}
    <div class="empty">Narrator not found.</div>
  {/if}
</div>

<style>
  .narrator-view { padding: 24px; max-width: 900px; }
  .view-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px; }
  .name-ar { color: var(--text-secondary); font-size: 1.2rem; margin-top: 4px; }
  .kunya { color: var(--text-muted); font-weight: normal; font-size: 0.85em; }
  .dates { color: var(--text-muted); font-size: 0.85rem; margin-top: 2px; }
  .badges { display: flex; gap: 8px; flex-wrap: wrap; }
  .location-tags { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 12px; }
  .location-tag { font-size: 0.75rem; padding: 2px 8px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 12px; color: var(--text-secondary); }
  .reliability-source { font-size: 0.8rem; color: var(--text-muted); margin-bottom: 16px; font-style: italic; }
  .bio { color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6; padding: 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); margin-bottom: 20px; max-height: 200px; overflow: hidden; text-overflow: ellipsis; }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 20px;
    position: sticky;
    top: 0;
    background: var(--bg-primary);
    z-index: 10;
    padding-top: 4px;
  }
  .tab {
    padding: 10px 16px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 2px solid transparent;
    transition: all var(--transition);
    margin-bottom: -1px;
    cursor: pointer;
  }
  .tab:hover { color: var(--text-primary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .hadith-list { display: flex; flex-direction: column; gap: 12px; }
  .connection-group { margin-bottom: 20px; }
  .connection-group h3 { margin-bottom: 10px; color: var(--text-secondary); font-size: 0.9rem; text-transform: uppercase; letter-spacing: 0.5px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }

  /* Details form */
  .details-form { display: flex; flex-direction: column; gap: 20px; }
  .form-section { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; }
  .form-section h3 { font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-secondary); margin-bottom: 16px; }
  .form-row { display: flex; gap: 12px; margin-bottom: 12px; }
  .form-row label { flex: 1; }
  .form-row label.half { flex: 2; }
  .form-row label.quarter { flex: 1; }
  label { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; }
  label span { font-size: 0.8rem; color: var(--text-secondary); }
  input, select, textarea {
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.9rem;
    font-family: inherit;
  }
  input:focus, select:focus, textarea:focus { border-color: var(--accent); outline: none; }
  textarea { resize: vertical; }
  .form-actions { display: flex; align-items: center; gap: 12px; }
  .save-btn {
    padding: 10px 24px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.9rem;
  }
  .save-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .save-msg { font-size: 0.85rem; color: var(--accent); }
  .save-msg.error { color: #ef4444; }
</style>
