<script lang="ts">
  import type { ApiSurah } from '$lib/types';

  let { surah }: { surah: ApiSurah } = $props();
</script>

<a href="/quran/{surah.surah_number}" class="surah-row">
  <div class="surah-number">
    <span class="number-badge">{String(surah.surah_number).padStart(3, '0')}</span>
  </div>
  <div class="surah-info">
    <span class="surah-translit">{surah.name_translit}</span>
    <span class="surah-meaning">{surah.name_en}</span>
  </div>
  <div class="surah-meta">
    <span class="surah-arabic" dir="rtl">{surah.name_ar}</span>
    <span class="surah-details">{surah.ayah_count} Ayahs · {surah.revelation_type}</span>
  </div>
</a>

<style>
  .surah-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    transition: background var(--transition);
  }
  .surah-row:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .number-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .surah-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .surah-translit {
    font-weight: 600;
    font-size: 0.95rem;
  }
  .surah-meaning {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .surah-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  .surah-arabic {
    font-size: 1.1rem;
    color: var(--text-primary);
  }
  .surah-details {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
