<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { MultiDirectedGraph } from 'graphology';
  import { Sigma } from 'sigma';
  // @ts-expect-error sigma re-exports not resolved by svelte-check
  import { EdgeArrowProgram } from 'sigma/rendering';
  import { createEdgeCurveProgram, indexParallelEdgesIndex } from '@sigma/edge-curve';
  import { createNodeBorderProgram } from '@sigma/node-border';
  import type { GraphData } from '$lib/types';
  import type { NarratorGraphNode, NarratorGraphEdge, NarratorGraph } from './types';
  import type { LayoutMode } from './layout';
  import { applyHierarchicalLayout, createFA2Supervisor } from './layout';
  import { drawLabel, drawHover } from './drawing';
  import { language } from '$lib/stores/language';

  let { data }: { data: GraphData | null } = $props();

  let container: HTMLDivElement = $state(null!);
  let sigmaInstance: Sigma<NarratorGraphNode, NarratorGraphEdge> | null = null;
  let universeGraph: NarratorGraph | null = null;
  let displayGraph: NarratorGraph | null = null;
  let supervisor: Awaited<ReturnType<typeof createFA2Supervisor>> | null = null;
  let mounted = false;
  let themeObserver: MutationObserver | null = null;

  // Sidebar state
  let showTeachers = $state(true);
  let showStudents = $state(true);
  let currentLayout: LayoutMode = $state('hierarchical');
  let straightEdges = $state(false);
  let forceRunning = $state(false);

  // Non-reactive hover state (used inside Sigma callbacks, not Svelte reactivity)
  let _hoveredNode = '';
  let _neighbors: Set<string> = new Set();

  // Computed stats
  let nodeCount = $state(0);
  let edgeCount = $state(0);
  let shownTeachers = $state(0);
  let shownStudents = $state(0);

  function getThemeColor(varName: string, fallback: string): string {
    if (typeof document === 'undefined') return fallback;
    return getComputedStyle(document.documentElement).getPropertyValue(varName).trim() || fallback;
  }

  function getNodeColor(type: string): string {
    if (type === 'center') return getThemeColor('--graph-center', '#c8a96a');
    if (type === 'student') return getThemeColor('--graph-student', '#b8860b');
    return getThemeColor('--graph-teacher', '#16a34a');
  }

  /** Re-read theme colors and push them into Sigma + both graphs */
  function refreshThemeColors() {
    if (!sigmaInstance || !displayGraph) return;

    const textColor = getThemeColor('--text-primary', '#1a1a2e');
    const edgeColor = getThemeColor('--graph-edge', '#dee2e6');

    sigmaInstance.setSetting('labelColor', { color: textColor });
    sigmaInstance.setSetting('defaultEdgeColor', edgeColor);

    // Update node colors on both graphs so universe stays in sync
    for (const graph of [universeGraph, displayGraph]) {
      if (!graph) continue;
      graph.forEachNode((node, attrs) => {
        graph.setNodeAttribute(node, 'color', getNodeColor(attrs.narratorType));
      });
    }

    sigmaInstance.refresh();
  }

  function buildUniverseGraph(graphData: GraphData): NarratorGraph {
    const graph = new MultiDirectedGraph<NarratorGraphNode, NarratorGraphEdge>();

    for (const n of graphData.nodes) {
      const d = n.data;
      // Skip duplicate nodes (a narrator can be both teacher and student)
      if (graph.hasNode(d.id)) continue;
      graph.addNode(d.id, {
        narratorId: d.id,
        narratorType: (d.type as 'center' | 'teacher' | 'student') || 'teacher',
        labelAr: d.label,
        labelEn: d.label_en,
        label: d.label,
        generation: d.generation,
        color: getNodeColor(d.type),
        x: 0,
        y: 0,
        size: d.type === 'center' ? 15 : 8,
      });
    }

    for (const e of graphData.edges) {
      const d = e.data;
      if (graph.hasNode(d.source) && graph.hasNode(d.target)) {
        graph.addDirectedEdgeWithKey(d.id, d.source, d.target, {
          weight: 1,
          label: d.label,
          size: 1.5,
        });
      }
    }

    return graph;
  }

  function synchronizeDisplayGraph() {
    if (!universeGraph || !displayGraph || !sigmaInstance) return;

    // Kill force supervisor before modifying graph structure
    const wasForce = currentLayout === 'force';
    if (supervisor) {
      supervisor.kill();
      supervisor = null;
      forceRunning = false;
    }

    // Collect nodes to remove (can't mutate during iteration)
    const toRemove: string[] = [];
    displayGraph.forEachNode((node, attrs) => {
      if (attrs.narratorType === 'teacher' && !showTeachers) toRemove.push(node);
      else if (attrs.narratorType === 'student' && !showStudents) toRemove.push(node);
    });
    for (const node of toRemove) {
      displayGraph.dropNode(node);
    }

    // Add nodes back from universe that should be shown
    universeGraph.forEachNode((node, attrs) => {
      const shouldShow =
        attrs.narratorType === 'center' ||
        (attrs.narratorType === 'teacher' && showTeachers) ||
        (attrs.narratorType === 'student' && showStudents);

      if (shouldShow && !displayGraph!.hasNode(node)) {
        displayGraph!.addNode(node, { ...attrs });
      }
    });

    // Sync edges
    const edgesToRemove: string[] = [];
    universeGraph.forEachEdge((edge, attrs, source, target) => {
      const sourceExists = displayGraph!.hasNode(source);
      const targetExists = displayGraph!.hasNode(target);

      if (sourceExists && targetExists && !displayGraph!.hasEdge(edge)) {
        displayGraph!.addDirectedEdgeWithKey(edge, source, target, { ...attrs });
      } else if ((!sourceExists || !targetExists) && displayGraph!.hasEdge(edge)) {
        edgesToRemove.push(edge);
      }
    });
    for (const edge of edgesToRemove) {
      if (displayGraph.hasEdge(edge)) displayGraph.dropEdge(edge);
    }

    applyEdgeTypes(displayGraph);

    // Only apply hierarchical layout if we're in hierarchical mode
    if (currentLayout === 'hierarchical') {
      applyHierarchicalLayout(displayGraph);
    }

    updateStats();
    sigmaInstance?.refresh();

    // Restart force supervisor if it was active
    if (wasForce) {
      createFA2Supervisor(displayGraph).then((s) => {
        supervisor = s;
        supervisor.start();
        forceRunning = true;
      });
    }
  }

  /** Apply edge type (straight vs curved) based on parallel index and straightEdges toggle */
  function applyEdgeTypes(graph: NarratorGraph) {
    indexParallelEdgesIndex(graph, {
      edgeIndexAttribute: 'parallelIndex',
      edgeMaxIndexAttribute: 'parallelMaxIndex',
    });
    graph.forEachEdge((edge, attrs) => {
      const maxIndex = attrs.parallelMaxIndex || 0;
      if (!straightEdges && maxIndex > 0) {
        graph.setEdgeAttribute(edge, 'type', 'curved');
        graph.setEdgeAttribute(edge, 'curvature', (attrs.parallelIndex || 0) / maxIndex);
      } else {
        graph.setEdgeAttribute(edge, 'type', 'straight');
      }
    });
  }

  function updateStats() {
    if (!displayGraph) return;
    nodeCount = displayGraph.order;
    edgeCount = displayGraph.size;
    let tc = 0, sc = 0;
    displayGraph.forEachNode((_node, attrs) => {
      if (attrs.narratorType === 'teacher') tc++;
      else if (attrs.narratorType === 'student') sc++;
    });
    shownTeachers = tc;
    shownStudents = sc;
  }

  function initGraph() {
    if (!container || !data || data.nodes.length === 0) return;

    // Ensure the container has been laid out with real dimensions.
    // On fresh mount the browser may not have completed layout yet.
    const rect = container.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {
      requestAnimationFrame(() => initGraph());
      return;
    }

    // Cleanup previous
    supervisor?.kill();
    supervisor = null;
    forceRunning = false;
    sigmaInstance?.kill();
    sigmaInstance = null;

    // Build universe graph and clone to display
    universeGraph = buildUniverseGraph(data);
    displayGraph = universeGraph.copy() as NarratorGraph;

    applyHierarchicalLayout(displayGraph);
    applyEdgeTypes(displayGraph);
    updateStats();

    // Instantiate Sigma — read colors live via getThemeColor in reducers
    sigmaInstance = new Sigma<NarratorGraphNode, NarratorGraphEdge>(displayGraph, container, {
      allowInvalidContainer: true,
      defaultEdgeColor: getThemeColor('--graph-edge', '#dee2e6'),
      defaultEdgeType: 'straight',
      renderEdgeLabels: false,
      labelFont: 'system-ui, sans-serif',
      labelColor: { color: getThemeColor('--text-primary', '#1a1a2e') },
      labelSize: 11,
      labelRenderedSizeThreshold: 8,
      stagePadding: 40,
      defaultDrawNodeLabel: drawLabel,
      defaultDrawNodeHover: drawHover,
      edgeProgramClasses: {
        straight: EdgeArrowProgram,
        curved: createEdgeCurveProgram({
          arrowHead: {
            widenessToThicknessRatio: 4,
            lengthToThicknessRatio: 5,
            extremity: 'target',
          },
        }),
      },
      nodeProgramClasses: {
        border: createNodeBorderProgram({
          borders: [
            { color: { attribute: 'color' }, size: { value: 0.1 } },
            { color: { transparent: true }, size: { value: 0.05 } },
            { color: { attribute: 'color' }, size: { value: 1.0 } },
          ],
        }),
      },
      defaultNodeType: 'border',
      nodeReducer: (node, attrs) => {
        // Guard against post-destroy calls
        if (!displayGraph) return attrs;
        const res = { ...attrs };
        if (_hoveredNode && _hoveredNode !== node && !_neighbors.has(node)) {
          res.label = '';
          // Read faded color live so theme changes take effect
          res.color = getThemeColor('--text-muted', 'rgba(150, 150, 150, 0.3)');
          res.zIndex = 0;
        }
        if (_hoveredNode === node || _neighbors.has(node)) {
          res.highlighted = true;
          res.zIndex = 1;
        }
        return res;
      },
      edgeReducer: (edge, attrs) => {
        if (!displayGraph) return attrs;
        const res = { ...attrs };
        if (_hoveredNode) {
          const extremities = displayGraph.extremities(edge);
          if (!extremities.includes(_hoveredNode)) {
            res.hidden = true;
          } else {
            res.color = getThemeColor('--graph-edge-hover', '#c8a96a');
            res.size = 2.5;
          }
        }
        return res;
      },
    });

    // Hover spotlight
    sigmaInstance.on('enterNode', ({ node }) => {
      _hoveredNode = node;
      if (displayGraph) _neighbors = new Set(displayGraph.neighbors(node));
      sigmaInstance?.refresh({ skipIndexation: true });
    });

    sigmaInstance.on('leaveNode', () => {
      _hoveredNode = '';
      _neighbors = new Set();
      sigmaInstance?.refresh({ skipIndexation: true });
    });

    // Click to navigate
    sigmaInstance.on('clickNode', ({ node }) => {
      const key = node.includes(':') ? node.split(':')[1] : node;
      goto(`/narrators/${key}`);
    });

    // Watch for theme changes (data-theme attribute on <html>)
    themeObserver?.disconnect();
    themeObserver = new MutationObserver(() => refreshThemeColors());
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme', 'class'],
    });
  }

  async function switchLayout(mode: LayoutMode) {
    if (!displayGraph || !sigmaInstance) return;

    // Kill existing supervisor first
    if (supervisor) {
      supervisor.kill();
      supervisor = null;
      forceRunning = false;
    }

    currentLayout = mode;

    if (mode === 'hierarchical') {
      applyHierarchicalLayout(displayGraph);
      sigmaInstance.refresh();
    } else {
      supervisor = await createFA2Supervisor(displayGraph);
      supervisor.start();
      forceRunning = true;
    }
  }

  function toggleForce() {
    if (!supervisor) return;
    if (forceRunning) {
      supervisor.stop();
      forceRunning = false;
    } else {
      supervisor.start();
      forceRunning = true;
    }
  }

  /** Update edge types when straightEdges changes */
  function updateEdgeStyle() {
    if (!displayGraph || !sigmaInstance) return;
    applyEdgeTypes(displayGraph);
    sigmaInstance.refresh();
  }

  // Track previous data reference to avoid re-init loops
  let prevData: GraphData | null = null;

  onMount(() => {
    mounted = true;
    initGraph();
    prevData = data;
  });

  // Re-init only when data actually changes (new prop reference)
  $effect(() => {
    const d = data;
    if (mounted && d && d !== prevData) {
      prevData = d;
      showTeachers = true;
      showStudents = true;
      currentLayout = 'hierarchical';
      tick().then(() => initGraph());
    }
  });

  // Sync display graph when visibility toggles change
  let visibilityInitialized = false;
  $effect(() => {
    const st = showTeachers;
    const ss = showStudents;
    if (!visibilityInitialized) {
      visibilityInitialized = true;
      return;
    }
    if (universeGraph && displayGraph && sigmaInstance) {
      synchronizeDisplayGraph();
    }
  });

  // Handle straightEdges toggle separately
  let edgeStyleInitialized = false;
  $effect(() => {
    const se = straightEdges;
    if (!edgeStyleInitialized) {
      edgeStyleInitialized = true;
      return;
    }
    updateEdgeStyle();
  });

  // Switch labels when language changes
  $effect(() => {
    const lang = $language;
    if (!displayGraph || !sigmaInstance) return;
    displayGraph.forEachNode((node, attrs) => {
      const newLabel = lang === 'en' && attrs.labelEn ? attrs.labelEn : attrs.labelAr;
      displayGraph!.setNodeAttribute(node, 'label', newLabel);
    });
  });

  onDestroy(() => {
    themeObserver?.disconnect();
    themeObserver = null;
    supervisor?.kill();
    supervisor = null;
    sigmaInstance?.kill();
    sigmaInstance = null;
    universeGraph = null;
    displayGraph = null;
  });
</script>

<div class="graph-panel">
  {#if !data || data.nodes.length === 0}
    <div class="empty">No graph data available</div>
  {:else}
    <div class="graph-canvas">
      <div bind:this={container} class="sigma-container"></div>
    </div>
    <aside class="graph-sidebar">
      <!-- Stats -->
      <div class="sidebar-section">
        <h4 class="sidebar-title">Stats</h4>
        <div class="stat-row">
          <span class="stat-label">Nodes</span>
          <span class="stat-value">{nodeCount}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Edges</span>
          <span class="stat-value">{edgeCount}</span>
        </div>
      </div>

      <!-- Visibility -->
      <div class="sidebar-section">
        <h4 class="sidebar-title">Visibility</h4>
        <button
          class="toggle-row"
          class:hidden-toggle={!showTeachers}
          onclick={() => { showTeachers = !showTeachers; }}
        >
          <span class="toggle-dot dot-teacher" class:faded={!showTeachers}></span>
          <span class="toggle-label">Teachers</span>
          <span class="toggle-count">{shownTeachers}{#if data.total_teachers} / {data.total_teachers}{/if}</span>
        </button>
        <button
          class="toggle-row"
          class:hidden-toggle={!showStudents}
          onclick={() => { showStudents = !showStudents; }}
        >
          <span class="toggle-dot dot-student" class:faded={!showStudents}></span>
          <span class="toggle-label">Students</span>
          <span class="toggle-count">{shownStudents}{#if data.total_students} / {data.total_students}{/if}</span>
        </button>
      </div>

      <!-- Appearance -->
      <div class="sidebar-section">
        <h4 class="sidebar-title">Appearance</h4>
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={straightEdges} />
          <span>Straight edges</span>
        </label>
      </div>

      <!-- Legend -->
      <div class="sidebar-section">
        <h4 class="sidebar-title">Legend</h4>
        <div class="legend-item"><span class="legend-dot dot-center"></span> This narrator</div>
        <div class="legend-item"><span class="legend-dot dot-teacher"></span> Teachers</div>
        <div class="legend-item"><span class="legend-dot dot-student"></span> Students</div>
      </div>
    </aside>
  {/if}
</div>

<style>
  .graph-panel {
    display: flex;
    height: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .graph-canvas {
    flex: 1;
    min-width: 0;
    position: relative;
  }

  .sigma-container {
    width: 100%;
    height: 100%;
    cursor: grab;
  }

  .sigma-container:active {
    cursor: grabbing;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    min-height: 400px;
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  /* Sidebar */
  .graph-sidebar {
    width: 240px;
    flex-shrink: 0;
    border-left: 1px solid var(--border);
    background: var(--bg-primary);
    overflow-y: auto;
    padding: 12px 0;
  }

  .sidebar-section {
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
  }

  .sidebar-section:last-child {
    border-bottom: none;
  }

  .sidebar-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 8px;
    font-weight: 600;
  }

  /* Stats */
  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 0;
    font-size: 0.8rem;
  }

  .stat-label { color: var(--text-secondary); }
  .stat-value { color: var(--text-primary); font-weight: 500; }

  /* Visibility toggles */
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    background: none;
    border-radius: var(--radius);
    cursor: pointer;
    transition: all 0.15s;
    font-size: 0.8rem;
    color: var(--text-primary);
    text-align: left;
  }

  .toggle-row:hover {
    background: var(--bg-surface);
  }

  .toggle-row.hidden-toggle {
    opacity: 0.5;
  }

  .toggle-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
    transition: background 0.15s;
  }

  .toggle-dot.faded {
    background: var(--text-muted) !important;
    opacity: 0.4;
  }

  .toggle-label { flex: 1; }

  .toggle-count {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .dot-center { background: var(--graph-center, #c8a96a); }
  .dot-teacher { background: var(--graph-teacher, #16a34a); }
  .dot-student { background: var(--graph-student, #b8860b); }

  /* Appearance */
  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8rem;
    color: var(--text-secondary);
    cursor: pointer;
    margin: 0;
  }

  .checkbox-row input {
    margin: 0;
    accent-color: var(--accent);
  }

  /* Legend */
  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
    color: var(--text-secondary);
    padding: 2px 0;
  }

  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  /* Mobile */
  @media (max-width: 768px) {
    .graph-panel {
      flex-direction: column;
    }

    .graph-canvas {
      min-height: 400px;
    }

    .graph-sidebar {
      width: 100%;
      border-left: none;
      border-top: 1px solid var(--border);
      display: flex;
      flex-wrap: wrap;
      padding: 8px;
      gap: 8px;
    }

    .sidebar-section {
      flex: 1;
      min-width: 120px;
      padding: 4px 8px;
      border-bottom: none;
    }
  }
</style>
