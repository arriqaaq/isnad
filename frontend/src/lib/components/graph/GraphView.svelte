<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import cytoscape from 'cytoscape';
  import dagre from 'cytoscape-dagre';
  import type { GraphData } from '$lib/types';
  import { language } from '$lib/stores/language';

  let {
    data,
    layout: layoutName = 'dagre',
  }: {
    data: GraphData | null;
    layout?: string;
  } = $props();

  let container: HTMLDivElement = $state(null!);
  let cy: cytoscape.Core | null = null;
  let registered = false;

  function initGraph() {
    if (!container || !data || data.nodes.length === 0) return;

    if (!registered) {
      cytoscape.use(dagre);
      registered = true;
    }

    cy?.destroy();

    const elements: cytoscape.ElementDefinition[] = [
      ...data.nodes.map((n) => ({ data: n.data })),
      ...data.edges.map((e) => ({ data: e.data })),
    ];

    cy = cytoscape({
      container,
      elements,
      style: [
        {
          selector: 'node',
          style: {
            'background-color': '#2d8f4e',
            label: 'data(label)',
            color: '#1a1a2e',
            'font-size': '11px',
            'text-valign': 'bottom',
            'text-margin-y': 6,
            width: 30,
            height: 30,
            'border-width': 2,
            'border-color': '#e5e7eb',
          },
        },
        {
          selector: 'node[type="center"]',
          style: {
            'background-color': '#b8860b',
            width: 40,
            height: 40,
            'font-weight': 'bold',
          },
        },
        {
          selector: 'node[type="teacher"]',
          style: { 'background-color': '#16a34a' },
        },
        {
          selector: 'node[type="student"]',
          style: { 'background-color': '#2563eb' },
        },
        {
          selector: 'edge',
          style: {
            width: 2,
            'line-color': '#d1d5db',
            'target-arrow-color': '#9ca3af',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'arrow-scale': 0.8,
          },
        },
      ],
      layout: {
        name: layoutName,
        rankDir: 'BT',
        nodeSep: 60,
        rankSep: 80,
        animate: false,
      } as any,
      userZoomingEnabled: true,
      userPanningEnabled: true,
      boxSelectionEnabled: false,
    });

    cy.on('tap', 'node', (evt) => {
      const nodeData = evt.target.data();
      const id = nodeData.id;
      const key = id.includes(':') ? id.split(':')[1] : id;
      goto(`/narrators/${key}`);
    });
  }

  onMount(() => { initGraph(); });

  $effect(() => {
    if (data) initGraph();
  });

  // Switch labels when language changes
  $effect(() => {
    const lang = $language;
    if (!cy) return;
    cy.nodes().forEach(node => {
      const labelEn = node.data('label_en');
      const labelAr = node.data('label');
      if (lang === 'en' && labelEn && labelEn !== labelAr) {
        node.style('label', labelEn);
      } else {
        node.style('label', labelAr);
      }
    });
  });

  onDestroy(() => {
    cy?.destroy();
    cy = null;
  });
</script>

<div class="graph-wrapper">
  {#if !data || data.nodes.length === 0}
    <div class="empty">No graph data available</div>
  {:else}
    <div bind:this={container} class="graph-container"></div>
  {/if}
</div>

<style>
  .graph-wrapper {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .graph-container {
    width: 100%;
    height: 400px;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--text-muted);
    font-size: 0.85rem;
  }
</style>
