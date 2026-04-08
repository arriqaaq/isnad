<script lang="ts">
  let { color, count = 1 }: { color: string; count?: number } = $props();
</script>

<span class="note-indicator" style="background: var(--note-{color})">
  {#if count > 1}
    <span class="count">{count}</span>
  {/if}
</span>

<style>
  .note-indicator {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .note-indicator:has(.count) {
    width: auto;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
  }
  .count {
    font-size: 0.6rem;
    color: #fff;
    font-weight: 700;
    font-family: var(--font-mono);
    line-height: 1;
  }
</style>
