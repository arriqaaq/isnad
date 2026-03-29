<script lang="ts">
  let { text, variant = 'default' }: {
    text: string;
    variant?: 'default' | 'accent' | 'success' | 'warning';
  } = $props();
</script>

<span class="badge {variant}">{text}</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    font-weight: 500;
    line-height: 1.4;
  }

  .default {
    background: var(--bg-surface);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .accent {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .success {
    background: rgba(74, 222, 128, 0.15);
    color: var(--success);
  }

  .warning {
    background: rgba(251, 191, 36, 0.15);
    color: var(--warning);
  }
</style>
