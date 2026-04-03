<script lang="ts">
  import type { ApiVariantReading } from '$lib/types';

  let { variants }: { variants: ApiVariantReading[] } = $props();
</script>

{#if variants.length > 0}
  <div class="variant-table-wrap">
    <table class="variant-table">
      <thead>
        <tr>
          <th>Reader</th>
          <th class="ar-col">Variant Reading</th>
          <th class="ar-col">Standard Text</th>
          <th>Source</th>
        </tr>
      </thead>
      <tbody>
        {#each variants as v}
          <tr>
            <td class="reader-name">{v.reader_name}</td>
            <td class="ar-col" dir="rtl">{v.reading_ar}</td>
            <td class="ar-col" dir="rtl">{v.standard_ar ?? '—'}</td>
            <td class="source">{v.source ?? ''}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else}
  <div class="empty">No variant readings found for this verse.</div>
{/if}

<style>
  .variant-table-wrap {
    overflow-x: auto;
  }
  .variant-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .variant-table th {
    text-align: left;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
  }
  .variant-table td {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--text-secondary);
  }
  .variant-table tr:last-child td {
    border-bottom: none;
  }
  .ar-col {
    font-size: 1rem;
    line-height: 1.8;
    color: var(--text-primary);
  }
  .reader-name {
    font-weight: 500;
    white-space: nowrap;
    color: var(--text-primary);
  }
  .source {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
