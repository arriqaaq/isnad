<script lang="ts">
  import type { ApiSurah } from '$lib/types';

  let { surah }: { surah: ApiSurah } = $props();
</script>

<div class="surah-header">
  <div class="header-card">
    <div class="header-top">
      <div class="surah-number-col">
        <span class="surah-num">{surah.surah_number}.</span>
      </div>
      <div class="surah-name-col">
        <div class="surah-name-ar" dir="rtl">{surah.name_ar}</div>
      </div>
      <div class="surah-info-col">
        <div class="surah-translit">{surah.name_translit}</div>
        <div class="surah-meaning">{surah.name_en}</div>
      </div>
    </div>

    <div class="header-meta">
      <span class="meta-badge">{surah.revelation_type}</span>
      <span class="meta-dot">·</span>
      <span class="meta-text">{surah.ayah_count} Ayahs</span>
    </div>
  </div>

  {#if surah.surah_number !== 9}
    <div class="bismillah-section">
      <div class="bismillah-ar" dir="rtl">بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ</div>
      <div class="bismillah-en">In the Name of Allah—the Most Compassionate, Most Merciful</div>
    </div>
  {/if}
</div>

<style>
  .surah-header {
    margin-bottom: 24px;
  }
  .header-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px 32px;
    text-align: center;
  }
  .header-top {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 24px;
    margin-bottom: 16px;
  }
  .surah-number-col {
    flex-shrink: 0;
  }
  .surah-num {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent);
  }
  .surah-name-col {
    flex-shrink: 0;
  }
  .surah-name-ar {
    font-size: 2.8rem;
    color: var(--text-primary);
    line-height: 1.2;
  }
  .surah-info-col {
    text-align: left;
  }
  .surah-translit {
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--text-primary);
  }
  .surah-meaning {
    font-size: 0.9rem;
    color: var(--text-muted);
    margin-top: 2px;
  }
  .header-meta {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .meta-badge {
    padding: 3px 12px;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 600;
  }
  .meta-dot { color: var(--text-muted); }
  .bismillah-section {
    text-align: center;
    padding: 24px 0 16px;
  }
  .bismillah-ar {
    font-size: 1.8rem;
    color: var(--text-primary);
    margin-bottom: 6px;
    line-height: 1.8;
    text-align: center;
  }
  .bismillah-en {
    font-size: 0.85rem;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
