<script lang="ts">
  import type { ApiManuscript } from '$lib/types';

  let { manuscript }: { manuscript: ApiManuscript } = $props();
</script>

<a href="/quran/manuscripts/{manuscript.id}" class="manuscript-card">
  <div class="ms-name">{manuscript.name}</div>
  <div class="ms-details">
    {#if manuscript.repository}
      <span class="ms-detail"><span class="label">Repository:</span> {manuscript.repository}</span>
    {/if}
    {#if manuscript.location}
      <span class="ms-detail"><span class="label">Location:</span> {manuscript.location}</span>
    {/if}
    {#if manuscript.date_range}
      <span class="ms-detail"><span class="label">Date:</span> {manuscript.date_range}</span>
    {/if}
    {#if manuscript.material}
      <span class="ms-detail"><span class="label">Material:</span> {manuscript.material}</span>
    {/if}
  </div>
</a>

<style>
  .manuscript-card {
    display: block;
    padding: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-decoration: none;
    transition: all var(--transition);
  }
  .manuscript-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .ms-name {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
  }
  .ms-details {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  .ms-detail {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .label {
    font-weight: 500;
    color: var(--text-muted);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
</style>
