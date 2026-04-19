<script lang="ts">
  import type { Snippet } from 'svelte';

  let { activeTab = 'content', content, chat }: {
    activeTab?: 'content' | 'chat';
    content: Snippet;
    chat: Snippet;
  } = $props();

  // svelte-ignore state_referenced_locally — prop seeds initial tab; component owns state after mount.
  let currentTab: 'content' | 'chat' = $state(activeTab);
</script>

<div class="sidebar-tabs-container">
  <div class="tab-bar">
    <button
      class="tab-btn"
      class:active={currentTab === 'content'}
      onclick={() => currentTab = 'content'}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="3" y1="6" x2="21" y2="6"/>
        <line x1="3" y1="12" x2="15" y2="12"/>
        <line x1="3" y1="18" x2="18" y2="18"/>
      </svg>
      Contents
    </button>
    <button
      class="tab-btn"
      class:active={currentTab === 'chat'}
      onclick={() => currentTab = 'chat'}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2C6.48 2 2 5.58 2 10c0 2.24 1.12 4.26 2.92 5.7L4 22l4.73-2.37C9.78 19.87 10.87 20 12 20c5.52 0 10-3.58 10-8s-4.48-8-10-8z"/>
      </svg>
      Ask AI
    </button>
  </div>

  <div class="tab-content">
    {#if currentTab === 'content'}
      {@render content()}
    {:else}
      {@render chat()}
    {/if}
  </div>
</div>

<style>
  .sidebar-tabs-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
    padding: 0 4px;
  }

  .tab-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 8px;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all var(--transition);
  }

  .tab-btn:hover {
    color: var(--text-secondary);
    background: var(--bg-hover, rgba(255,255,255,0.03));
  }

  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
