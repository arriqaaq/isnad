<script lang="ts">
  import type { TafsirOption } from '$lib/types';

  let { options, selectedKey, onselect, onclose }: {
    options: TafsirOption[];
    selectedKey: string;
    onselect: (key: string) => void;
    onclose: () => void;
  } = $props();

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('picker-backdrop')) {
      onclose();
    }
  }

  function pick(key: string) {
    onselect(key);
    onclose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="picker-backdrop" onclick={handleBackdrop}>
  <div class="picker-dialog" role="dialog" aria-label="Select tafsir source">
    <div class="picker-header">
      <span>Choose tafsir</span>
      <button class="picker-close" onclick={onclose}>&times;</button>
    </div>
    <ul class="picker-list">
      {#each options as opt}
        <li>
          <button
            class="picker-item"
            class:selected={opt.key === selectedKey}
            onclick={() => pick(opt.key)}
          >
            <span class="radio" aria-hidden="true">{opt.key === selectedKey ? '●' : '○'}</span>
            <span class="item-text">
              <span class="item-label">{opt.label}</span>
              {#if opt.subtitle}
                <span class="item-subtitle">{opt.subtitle}</span>
              {/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 0.12s ease-out;
  }
  .picker-dialog {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 56px rgba(0, 0, 0, 0.35);
    width: min(420px, calc(100vw - 32px));
    max-height: min(70vh, 520px);
    display: flex;
    flex-direction: column;
    animation: slideUp 0.15s ease-out;
  }
  .picker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    font-weight: 600;
    color: var(--text-primary);
    font-size: 0.95rem;
  }
  .picker-close {
    font-size: 1.4rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }
  .picker-close:hover { color: var(--text-primary); }
  .picker-list {
    margin: 0;
    padding: 8px 0;
    list-style: none;
    overflow-y: auto;
  }
  .picker-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: none;
    border: none;
    text-align: left;
    color: var(--text-secondary);
    cursor: pointer;
    font: inherit;
    transition: background var(--transition);
  }
  .picker-item:hover { background: var(--bg-secondary); }
  .picker-item.selected {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }
  .radio {
    color: var(--accent);
    font-size: 1rem;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }
  .item-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .item-label {
    font-size: 0.9rem;
    font-weight: 500;
  }
  .item-subtitle {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideUp {
    from { transform: translateY(8px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>
