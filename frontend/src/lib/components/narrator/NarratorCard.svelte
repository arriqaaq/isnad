<script lang="ts">
  import type { ApiNarratorWithCount } from '$lib/types';
  import Badge from '$lib/components/common/Badge.svelte';
  import { language } from '$lib/stores/language';

  let { narrator }: { narrator: ApiNarratorWithCount } = $props();

  let displayName = $derived(
    $language === 'en' && narrator.name_en && narrator.name_en !== narrator.name_ar
      ? narrator.name_en
      : (narrator.name_ar || narrator.name_en)
  );
  let isArabic = $derived($language === 'ar' || !narrator.name_en || narrator.name_en === narrator.name_ar);
</script>

<a href="/narrators/{narrator.id}" class="narrator-card">
  <div class="card-header">
    <h3 class="name" class:arabic={isArabic} dir={isArabic ? 'rtl' : 'ltr'}>{displayName}</h3>
    {#if narrator.generation}
      <Badge text={narrator.generation} variant="accent" />
    {/if}
  </div>

  <div class="card-footer">
    <span class="hadith-count mono">{narrator.hadith_count} hadiths</span>
  </div>
</a>

<style>
  .narrator-card {
    display: block;
    padding: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: all var(--transition);
    color: var(--text-primary);
    overflow: hidden;
  }

  .narrator-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .name {
    font-size: 0.95rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .card-footer { margin-top: 8px; }

  .hadith-count {
    color: var(--text-muted);
    font-size: 0.8rem;
  }
</style>
