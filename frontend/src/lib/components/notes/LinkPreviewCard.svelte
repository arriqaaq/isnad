<script lang="ts">
  import { fetchLinkPreview } from '$lib/api';
  import type { LinkPreview } from '$lib/types';

  let { url }: { url: string } = $props();

  let preview: LinkPreview | null = $state(null);
  let loading = $state(true);
  let failed = $state(false);

  let domain = $derived(
    url.replace(/^https?:\/\//, '').split('/')[0]
  );

  $effect(() => {
    loading = true;
    failed = false;
    fetchLinkPreview(url)
      .then(p => { preview = p; })
      .catch(() => { failed = true; })
      .finally(() => { loading = false; });
  });
</script>

{#if failed || (!loading && !preview?.title)}
  <a href={url} target="_blank" rel="noopener" class="plain-link">{url}</a>
{:else}
  <a href={url} target="_blank" rel="noopener" class="preview-card">
    {#if preview?.image}
      <img src={preview.image} alt="" class="preview-image" />
    {/if}
    <div class="preview-body">
      <div class="preview-title">{preview?.title ?? domain}</div>
      {#if preview?.description}
        <div class="preview-desc">{preview.description}</div>
      {/if}
      <div class="preview-domain">{preview?.domain ?? domain}</div>
    </div>
  </a>
{/if}

<style>
  .plain-link {
    color: var(--accent);
    font-size: 0.8rem;
    word-break: break-all;
  }
  .preview-card {
    display: flex;
    gap: 12px;
    padding: 12px 16px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xl);
    background: var(--bg-surface);
    text-decoration: none;
    color: inherit;
    box-shadow: var(--shadow-card);
    transition: box-shadow var(--transition), border-color var(--transition);
    margin: 8px 0;
  }
  .preview-card:hover {
    border-color: var(--border);
    box-shadow: var(--shadow-card-hover);
  }
  .preview-image {
    width: 64px;
    height: 64px;
    object-fit: cover;
    border-radius: var(--radius);
    flex-shrink: 0;
  }
  .preview-body {
    min-width: 0;
    flex: 1;
  }
  .preview-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .preview-desc {
    font-size: 0.75rem;
    color: var(--text-secondary);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin-top: 2px;
  }
  .preview-domain {
    font-size: 0.65rem;
    color: var(--text-muted);
    margin-top: 4px;
  }
</style>
