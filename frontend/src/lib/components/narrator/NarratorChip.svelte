<script lang="ts">
  import type { ApiNarrator } from '$lib/types';

  let { narrator }: { narrator: ApiNarrator } = $props();
</script>

<a href="/narrators/{narrator.id}" class="chip">
  {narrator.name_ar || narrator.name_en || narrator.id}
</a>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    padding: 4px 12px;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 20px;
    font-size: 0.8rem;
    font-weight: 500;
    transition: all var(--transition);
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip:hover {
    background: var(--accent);
    color: var(--bg-primary);
  }
</style>
