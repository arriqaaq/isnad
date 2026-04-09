<script lang="ts">
  let { query, position, onselect, onclose }: {
    query: string;
    position: { x: number; y: number };
    onselect: (command: string) => void;
    onclose: () => void;
  } = $props();

  let selectedIdx = $state(0);

  const COMMANDS = [
    { id: 'heading', label: 'Heading', desc: 'Large section heading', icon: 'H₂' },
    { id: 'subheading', label: 'Subheading', desc: 'Smaller heading', icon: 'H₃' },
    { id: 'quote', label: 'Quote', desc: 'Block quote', icon: '❝' },
    { id: 'bullet', label: 'Bullet List', desc: 'Unordered list', icon: '•' },
    { id: 'numbered', label: 'Numbered List', desc: 'Ordered list', icon: '1.' },
    { id: 'divider', label: 'Divider', desc: 'Horizontal rule', icon: '―' },
    { id: 'ayah', label: 'Quran Ayah', desc: 'Embed a Quran verse', icon: '﴾' },
    { id: 'hadith', label: 'Hadith', desc: 'Embed a hadith', icon: '☰' },
    { id: 'narrator', label: 'Narrator', desc: 'Mention a narrator', icon: '◎' },
  ];

  let filtered = $derived(
    query.length === 0
      ? COMMANDS
      : COMMANDS.filter(c =>
          c.label.toLowerCase().includes(query.toLowerCase()) ||
          c.id.includes(query.toLowerCase())
        )
  );

  // Reset selection when filtered list changes
  $effect(() => {
    if (filtered.length > 0 && selectedIdx >= filtered.length) {
      selectedIdx = 0;
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (filtered.length > 0) {
        onselect(filtered[selectedIdx].id);
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIdx = Math.min(selectedIdx + 1, filtered.length - 1);
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIdx = Math.max(selectedIdx - 1, 0);
    }
  }

  export { handleKeydown };
</script>

<div class="slash-palette" style="top: {position.y}px; left: {position.x}px;">
  {#if filtered.length === 0}
    <div class="slash-empty">No commands match "/{query}"</div>
  {:else}
    {#each filtered as cmd, i}
      <button
        class="slash-item"
        class:selected={i === selectedIdx}
        onmousedown={(e) => { e.preventDefault(); onselect(cmd.id); }}
        onmouseenter={() => { selectedIdx = i; }}
      >
        <span class="slash-icon">{cmd.icon}</span>
        <span class="slash-info">
          <span class="slash-label">{cmd.label}</span>
          <span class="slash-desc">{cmd.desc}</span>
        </span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .slash-palette {
    position: fixed;
    background: var(--bg-surface);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xl);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
    z-index: 9999;
    max-height: 320px;
    min-width: 240px;
    max-width: 320px;
    overflow-y: auto;
    padding: 4px;
  }
  .slash-empty {
    padding: 16px;
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
    text-align: center;
  }
  .slash-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    text-align: left;
    padding: 10px 14px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: var(--radius);
    transition: background var(--transition);
  }
  .slash-item:hover,
  .slash-item.selected {
    background: var(--bg-hover);
  }
  .slash-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius);
    background: var(--accent-muted);
    color: var(--accent);
    font-size: 0.9rem;
    font-weight: 600;
    flex-shrink: 0;
  }
  .slash-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .slash-label {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .slash-desc {
    font-size: 0.7rem;
    color: var(--text-muted);
  }
</style>
