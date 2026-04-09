<script lang="ts">
  import type { CCManuscript } from '$lib/types';
  let { manuscript }: { manuscript: CCManuscript } = $props();
  let showModal = $state(false);
  let imageUrl = $derived(
    manuscript.pages?.[0]?.images?.[0]?.image_url ?? ''
  );
  let folioLabel = $derived(
    manuscript.pages?.[0] ? `${manuscript.pages[0].folio}${manuscript.pages[0].side}` : ''
  );
</script>

{#if imageUrl}
  <button class="ms-card" onclick={() => showModal = true}>
    <img class="ms-thumb" src={imageUrl} alt={manuscript.title} loading="lazy" />
    <div class="ms-info">
      <div class="ms-title" title={manuscript.title}>{manuscript.title}</div>
      <div class="ms-archive">{manuscript.archive?.city} — {manuscript.archive?.name}</div>
      {#if folioLabel}
        <span class="ms-folio">{folioLabel}</span>
      {/if}
    </div>
  </button>
{/if}

{#if showModal}
  <div class="modal-overlay" onclick={() => showModal = false} role="dialog" aria-modal="true">
    <div class="modal-content" onclick={(e) => e.stopPropagation()}>
      <button class="modal-close" onclick={() => showModal = false}>&#10005;</button>
      <img class="modal-img" src={imageUrl} alt={manuscript.title} />
      <div class="modal-caption">
        <div class="modal-title">{manuscript.title}</div>
        <div class="modal-archive">{manuscript.archive?.city} — {manuscript.archive?.name}</div>
      </div>
    </div>
  </div>
{/if}

<style>
  .ms-card {
    width: 200px;
    flex-shrink: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
    cursor: pointer;
    box-shadow: var(--shadow-card);
    transition: all var(--transition);
    text-align: left;
    padding: 0;
  }
  .ms-card:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-card-hover);
    transform: translateY(-2px);
  }
  .ms-thumb {
    width: 100%;
    height: 180px;
    object-fit: cover;
  }
  .ms-info {
    padding: 10px 12px;
  }
  .ms-title {
    font-family: var(--font-serif);
    font-size: 0.75rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ms-archive {
    font-size: 0.6rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ms-folio {
    display: inline-block;
    margin-top: 4px;
    font-size: 0.55rem;
    font-family: var(--font-mono);
    color: var(--accent);
    background: var(--accent-muted);
    border: none;
    border-radius: var(--radius-sm);
    padding: 2px 8px;
  }

  /* Modal overlay */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal-content {
    position: relative;
    max-width: 90vw;
    max-height: 90vh;
    background: var(--bg-surface);
    border-radius: var(--radius-2xl);
    box-shadow: 0 16px 64px rgba(0, 0, 0, 0.3);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .modal-close {
    position: absolute;
    top: 8px;
    right: 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 1rem;
    width: 28px;
    height: 28px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1;
  }
  .modal-close:hover {
    background: var(--bg-hover);
  }
  .modal-img {
    max-width: 100%;
    max-height: 80vh;
    object-fit: contain;
  }
  .modal-caption {
    padding: 16px 20px;
  }
  .modal-title {
    font-family: var(--font-serif);
    font-size: 0.95rem;
    color: var(--text-primary);
    font-weight: 600;
  }
  .modal-archive {
    font-family: var(--font-serif);
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
