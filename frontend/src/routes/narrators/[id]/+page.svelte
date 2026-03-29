<script lang="ts">
  import { page } from '$app/state';
  import { getNarrator, getNarratorGraph } from '$lib/api';
  import type { NarratorDetailResponse, GraphData } from '$lib/types';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import GraphView from '$lib/components/graph/GraphView.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: NarratorDetailResponse | null = $state(null);
  let graphData: GraphData | null = $state(null);
  let loading = $state(true);
  let activeTab: 'network' | 'hadiths' | 'connections' = $state('network');

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true;
    activeTab = 'network';
    Promise.all([getNarrator(id), getNarratorGraph(id)])
      .then(([d, g]) => {
        // Deduplicate hadiths (can have dupes from multiple narrates edges)
        const seen = new Set<string>();
        d.hadiths = d.hadiths.filter(h => {
          if (seen.has(h.id)) return false;
          seen.add(h.id);
          return true;
        });
        data = d;
        graphData = g;
      })
      .catch((e) => console.error('Failed to load narrator:', e))
      .finally(() => { loading = false; });
  });
</script>

<div class="narrator-view">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="view-header">
      <div>
        <h1>{data.narrator.name_en}</h1>
        {#if data.narrator.name_ar}
          <p class="name-ar arabic" dir="rtl">{data.narrator.name_ar}</p>
        {/if}
      </div>
      <div class="badges">
        {#if data.narrator.generation}
          <Badge text={data.narrator.generation} variant="accent" />
        {/if}
        {#if data.narrator.gender}
          <Badge text={data.narrator.gender} />
        {/if}
      </div>
    </div>

    {#if data.narrator.bio}
      <div class="bio">{data.narrator.bio}</div>
    {/if}

    <div class="tabs">
      <button type="button" class="tab" class:active={activeTab === 'network'} onclick={() => { activeTab = 'network'; }}>Network</button>
      <button type="button" class="tab" class:active={activeTab === 'hadiths'} onclick={() => { activeTab = 'hadiths'; }}>Hadiths ({data.hadiths.length})</button>
      <button type="button" class="tab" class:active={activeTab === 'connections'} onclick={() => { activeTab = 'connections'; }}>Connections</button>
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
  .badges { display: flex; gap: 8px; }
  .bio { color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6; padding: 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); margin-bottom: 20px; }
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
</style>
