<script lang="ts">
  import type { GraphData } from '$lib/types';

  let { data }: { data: GraphData | null } = $props();

  // Build ordered chain from graph data (top = closest to Prophet, bottom = closest to compiler)
  // The graph has nodes + edges where source->target means "heard from"
  // We reverse it so the top card is the source (Prophet/Companion end)
  interface ChainNode {
    id: string;
    label: string;
    type: string;
  }

  let chain: ChainNode[] = $derived.by(() => {
    if (!data || data.nodes.length === 0) return [];

    const nodes = new Map(data.nodes.map(n => [n.data.id, n.data]));
    const edges = data.edges.map(e => ({ from: e.data.source, to: e.data.target }));

    if (edges.length === 0) {
      // No edges — just return nodes as a flat list
      return data.nodes.map(n => ({
        id: n.data.id,
        label: n.data.label,
        type: n.data.type,
      }));
    }

    // Build adjacency: who did each node hear from?
    const heardFrom = new Map<string, string>(); // node -> teacher
    for (const e of edges) {
      heardFrom.set(e.from, e.to);
    }

    // Find the start (node that no one heard from — the compiler's end)
    const targets = new Set(edges.map(e => e.to));
    const sources = new Set(edges.map(e => e.from));
    let start = '';
    for (const s of sources) {
      if (!targets.has(s)) {
        start = s;
        break;
      }
    }
    if (!start && sources.size > 0) start = [...sources][0];

    // Walk the chain from start (compiler end) to end (Prophet end)
    const ordered: ChainNode[] = [];
    const visited = new Set<string>();
    let current = start;
    while (current && !visited.has(current)) {
      visited.add(current);
      const node = nodes.get(current);
      if (node) {
        ordered.push({ id: current, label: node.label, type: node.type });
      }
      current = heardFrom.get(current) || '';
    }

    // Reverse so top = Prophet/Companion, bottom = compiler's teacher
    ordered.reverse();

    // Add any unlinked nodes at the bottom
    for (const n of data.nodes) {
      if (!visited.has(n.data.id)) {
        ordered.push({ id: n.data.id, label: n.data.label, type: n.data.type });
      }
    }

    return ordered;
  });

  function narratorKey(id: string): string {
    return id.includes(':') ? id.split(':')[1] : id;
  }
</script>

<div class="chain-view">
  {#if chain.length === 0}
    <div class="empty">No chain data available</div>
  {:else}
    <div class="chain">
      {#each chain as node, i}
        <a href="/narrators/{narratorKey(node.id)}" class="chain-card" class:first={i === 0}>
          <span class="position">{i + 1}</span>
          <div class="names">
            <span class="name-ar arabic">{node.label}</span>
          </div>
        </a>
        {#if i < chain.length - 1}
          <div class="connector">
            <div class="line"></div>
            <div class="arrow">▼</div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .chain-view {
    padding: 16px 0;
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: 40px;
    font-size: 0.85rem;
  }

  .chain {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
  }

  .chain-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 20px;
    background: var(--bg-surface);
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    min-width: 240px;
    max-width: 400px;
    color: var(--text-primary);
    transition: all var(--transition);
    text-decoration: none;
  }

  .chain-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  }

  .chain-card.first {
    border-color: var(--accent-secondary);
    border-width: 2px;
  }

  .position {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent-muted);
    color: var(--accent);
    font-size: 0.7rem;
    font-weight: 700;
    flex-shrink: 0;
  }

  .chain-card.first .position {
    background: rgba(184, 134, 11, 0.15);
    color: var(--accent-secondary);
  }

  .names {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .name-ar {
    font-size: 1rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .connector {
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 28px;
    color: var(--border);
  }

  .line {
    width: 1.5px;
    flex: 1;
    background: var(--border);
  }

  .arrow {
    font-size: 0.5rem;
    line-height: 1;
    color: var(--text-muted);
  }
</style>
