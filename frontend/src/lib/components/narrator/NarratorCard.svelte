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

  {#if narrator.kunya}
    <div class="kunya" dir="rtl">{narrator.kunya}</div>
  {/if}

  <div class="card-footer">
    <span class="hadith-count mono">{narrator.hadith_count} hadiths</span>
    {#if narrator.death_year}
      <span class="death-year mono">d. {narrator.death_year} AH</span>
    {/if}
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

  .kunya {
    color: var(--text-secondary);
    font-size: 0.8rem;
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-footer {
    margin-top: 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .hadith-count {
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  .death-year {
    color: var(--text-muted);
    font-size: 0.75rem;
  }
</style>
