<script lang="ts">
  import { page } from '$app/state';
  import { getManuscript } from '$lib/api';
  import type { ApiManuscript } from '$lib/types';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let manuscript: ApiManuscript | null = $state(null);
  let loading = $state(true);
  let error = $state('');

  let manuscriptId = $derived(page.params.id);

  $effect(() => {
    loading = true;
    error = '';
    getManuscript(manuscriptId!)
      .then((ms) => {
        manuscript = ms;
        loading = false;
      })
      .catch((e) => {
        error = e.message;
        loading = false;
      });
  });
</script>

<div class="manuscript-detail">
  <a href="/quran/manuscripts" class="back-link">All Manuscripts</a>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="error">{error}</div>
  {:else if manuscript}
    <h1 class="ms-title">{manuscript.name}</h1>

    <div class="ms-fields">
      {#if manuscript.repository}
        <div class="field">
          <span class="field-label">Repository</span>
          <span class="field-value">{manuscript.repository}</span>
        </div>
      {/if}
      {#if manuscript.location}
        <div class="field">
          <span class="field-label">Location</span>
          <span class="field-value">{manuscript.location}</span>
        </div>
      {/if}
      {#if manuscript.date_range}
        <div class="field">
          <span class="field-label">Date Range</span>
          <span class="field-value">{manuscript.date_range}</span>
        </div>
      {/if}
      {#if manuscript.material}
        <div class="field">
          <span class="field-label">Material</span>
          <span class="field-value">{manuscript.material}</span>
        </div>
      {/if}
      {#if manuscript.script_type}
        <div class="field">
          <span class="field-label">Script Type</span>
          <span class="field-value">{manuscript.script_type}</span>
        </div>
      {/if}
      {#if manuscript.description}
        <div class="field">
          <span class="field-label">Description</span>
          <span class="field-value">{manuscript.description}</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .manuscript-detail {
    padding: 24px;
    max-width: 800px;
    margin: 0 auto;
  }
  .back-link {
    font-size: 0.85rem;
    color: var(--accent);
    display: inline-block;
    margin-bottom: 16px;
  }
  .back-link:hover {
    text-decoration: underline;
  }
  .ms-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 24px;
  }
  .ms-fields {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .field-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }
  .field-value {
    font-size: 0.9rem;
    color: var(--text-primary);
    line-height: 1.5;
  }
  .error {
    color: var(--error, #d50000);
    padding: 16px;
    text-align: center;
  }
  @media (max-width: 640px) {
    .manuscript-detail { padding: 12px; }
  }
</style>
