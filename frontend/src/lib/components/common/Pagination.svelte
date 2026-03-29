<script lang="ts">
  let { page, hasMore, onPageChange }: {
    page: number;
    hasMore: boolean;
    onPageChange: (page: number) => void;
  } = $props();
</script>

<div class="pagination">
  <button
    class="page-btn"
    disabled={page <= 1}
    onclick={() => onPageChange(page - 1)}
  >
    Previous
  </button>
  <span class="page-info mono">Page {page}</span>
  <button
    class="page-btn"
    disabled={!hasMore}
    onclick={() => onPageChange(page + 1)}
  >
    Next
  </button>
</div>

<style>
  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 16px 0;
  }

  .page-btn {
    padding: 8px 16px;
    border-radius: var(--radius);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 0.85rem;
    transition: all var(--transition);
  }

  .page-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .page-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .page-info {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }
</style>
