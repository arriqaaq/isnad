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

  function getThemeColor(varName: string, fallback: string): string {
    if (typeof document === 'undefined') return fallback;
    return getComputedStyle(document.documentElement).getPropertyValue(varName).trim() || fallback;
  }

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

    const textColor = getThemeColor('--text-primary', '#1a1a2e');
    const surfaceColor = getThemeColor('--bg-surface', '#ffffff');
    const centerColor = getThemeColor('--graph-center', '#d63384');
    const teacherColor = getThemeColor('--graph-teacher', '#16a34a');
    const studentColor = getThemeColor('--graph-student', '#b8860b');
    const edgeColor = getThemeColor('--graph-edge', 'rgba(156, 163, 175, 0.35)');
    const edgeHoverColor = getThemeColor('--graph-edge-hover', '#d63384');
    const shadowCenter = getThemeColor('--graph-shadow-center', 'rgba(214, 51, 132, 0.3)');
    const shadowTeacher = getThemeColor('--graph-shadow-teacher', 'rgba(22, 163, 74, 0.3)');
    const shadowStudent = getThemeColor('--graph-shadow-student', 'rgba(184, 134, 11, 0.3)');

    const nodeCount = data.nodes.length;
    const chosenLayout = {
      name: 'dagre',
      rankDir: 'BT',
      nodeSep: nodeCount > 20 ? 40 : 80,
      rankSep: nodeCount > 20 ? 60 : 100,
      animate: false,
    };

    cy = cytoscape({
      container,
      elements,
      style: [
        // ── Default node ──
        {
          selector: 'node',
          style: {
            'background-color': teacherColor,
            'background-fill': 'radial-gradient',
            'background-gradient-stop-colors': `${lighten(teacherColor)} ${teacherColor}`,
            'background-gradient-stop-positions': '0% 100%',
            label: 'data(label)',
            color: textColor,
            'font-size': '11px',
            'text-valign': 'bottom',
            'text-margin-y': 8,
            'text-outline-width': 2,
            'text-outline-color': surfaceColor,
            'text-max-width': '90px',
            'text-wrap': 'ellipsis',
            width: 32,
            height: 32,
            'border-width': 1.5,
            'border-color': teacherColor,
            'border-opacity': 0.3,
            'shadow-blur': 12,
            'shadow-color': shadowTeacher,
            'shadow-opacity': 0.2,
            'shadow-offset-x': 0,
            'shadow-offset-y': 2,
            'overlay-padding': 6,
          } as any,
        },
        // ── Center node (the narrator being viewed) ──
        {
          selector: 'node[type="center"]',
          style: {
            'background-color': centerColor,
            'background-gradient-stop-colors': `${lighten(centerColor)} ${centerColor}`,
            width: 50,
            height: 50,
            'font-weight': 'bold',
            'font-size': '13px',
            'text-max-width': '120px',
            'border-color': centerColor,
            'shadow-color': shadowCenter,
            'shadow-opacity': 0.4,
            'shadow-blur': 22,
          } as any,
        },
        // ── Teacher nodes ──
        {
          selector: 'node[type="teacher"]',
          style: {
            'background-color': teacherColor,
            'background-gradient-stop-colors': `${lighten(teacherColor)} ${teacherColor}`,
            'border-color': teacherColor,
            'shadow-color': shadowTeacher,
          } as any,
        },
        // ── Student nodes ──
        {
          selector: 'node[type="student"]',
          style: {
            'background-color': studentColor,
            'background-gradient-stop-colors': `${lighten(studentColor)} ${studentColor}`,
            'border-color': studentColor,
            'shadow-color': shadowStudent,
          } as any,
        },
        // ── Edges ──
        {
          selector: 'edge',
          style: {
            width: 1.5,
            'line-color': edgeColor,
            'line-opacity': 0.5,
            'target-arrow-color': edgeColor,
            'target-arrow-shape': 'vee',
            'curve-style': 'bezier',
            'arrow-scale': 0.6,
          },
        },
        // ── Faded state (applied during hover spotlight) ──
        {
          selector: '.faded',
          style: {
            opacity: 0.1,
          } as any,
        },
        // ── Highlighted edge during hover ──
        {
          selector: '.highlighted-edge',
          style: {
            'line-color': edgeHoverColor,
            'line-opacity': 1,
            width: 2.5,
            'target-arrow-color': edgeHoverColor,
          } as any,
        },
      ],
      layout: chosenLayout as any,
      userZoomingEnabled: true,
      userPanningEnabled: true,
      boxSelectionEnabled: false,
    } as any);

    // Fit graph to container after layout
    cy.fit(undefined, 40);

    // ── Click to navigate ──
    cy.on('tap', 'node', (evt) => {
      const nodeData = evt.target.data();
      const id = nodeData.id;
      const key = id.includes(':') ? id.split(':')[1] : id;
      goto(`/narrators/${key}`);
    });

    // ── Spotlight hover: highlight connections, fade the rest ──
    cy.on('mouseover', 'node', (evt) => {
      const node = evt.target;
      const neighborhood = node.closedNeighborhood();

      // Fade everything
      cy!.elements().addClass('faded');

      // Un-fade the neighborhood
      neighborhood.removeClass('faded');

      // Highlight connected edges
      node.connectedEdges().addClass('highlighted-edge');

      // Enlarge hovered node slightly
      node.animate(
        { style: { width: node.numericStyle('width') * 1.15, height: node.numericStyle('height') * 1.15 } },
        { duration: 150 }
      );
    });

    cy.on('mouseout', 'node', (evt) => {
      const node = evt.target;

      // Restore all elements
      cy!.elements().removeClass('faded').removeClass('highlighted-edge');

      // Restore node size
      const isCenter = node.data('type') === 'center';
      const originalSize = isCenter ? 48 : 34;
      node.animate(
        { style: { width: originalSize, height: originalSize } },
        { duration: 150 }
      );
    });
  }

  /** Lighten a hex color for radial gradient center */
  function lighten(hex: string): string {
    const clean = hex.replace('#', '');
    if (clean.length !== 6) return hex;
    const r = Math.min(255, parseInt(clean.slice(0, 2), 16) + 60);
    const g = Math.min(255, parseInt(clean.slice(2, 4), 16) + 60);
    const b = Math.min(255, parseInt(clean.slice(4, 6), 16) + 60);
    return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
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
    <div class="graph-footer">
      <div class="graph-legend">
        <span class="legend-item"><span class="dot dot-center"></span> This narrator</span>
        <span class="legend-item"><span class="dot dot-teacher"></span> Teachers</span>
        <span class="legend-item"><span class="dot dot-student"></span> Students</span>
      </div>
      {#if data.total_teachers != null || data.total_students != null}
        {@const shownTeachers = data.nodes.filter(n => n.data.type === 'teacher').length}
        {@const shownStudents = data.nodes.filter(n => n.data.type === 'student').length}
        {#if (data.total_teachers && shownTeachers < data.total_teachers) || (data.total_students && shownStudents < data.total_students)}
          <div class="graph-truncated">
            Showing {shownTeachers} of {data.total_teachers} teachers, {shownStudents} of {data.total_students} students
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .graph-wrapper {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .graph-container {
    width: 100%;
    height: 500px;
    cursor: grab;
  }

  .graph-container:active {
    cursor: grabbing;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  .graph-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    flex-wrap: wrap;
    gap: 8px;
  }

  .graph-legend {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: inline-block;
  }

  .dot-center { background: var(--graph-center, #d63384); }
  .dot-teacher { background: var(--graph-teacher, #16a34a); }
  .dot-student { background: var(--graph-student, #b8860b); }

  .graph-truncated {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-style: italic;
  }

  @media (max-width: 768px) {
    .graph-container { height: 320px; }
  }
</style>
