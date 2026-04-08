<script lang="ts">
  import type { ApiSurah } from '$lib/types';

  let { surah }: { surah: ApiSurah } = $props();
</script>

<div class="surah-header">
  <div class="header-card">
    <div class="ornament-top">&#xFD3E; &#xFD3F;</div>
    <div class="surah-name-ar" dir="rtl">{surah.name_ar}</div>
    <div class="surah-info">
      <span class="surah-translit">{surah.name_translit}</span>
      <span class="info-sep">&mdash;</span>
      <span class="surah-meaning">{surah.name_en}</span>
    </div>
    <div class="header-meta">
      <span class="meta-badge">{surah.revelation_type}</span>
      <span class="meta-dot">&middot;</span>
      <span class="meta-text">{surah.ayah_count} Ayahs</span>
      <span class="meta-dot">&middot;</span>
      <span class="meta-text">Surah {surah.surah_number}</span>
    </div>
    <div class="ornament-bottom">&#x2022; &#x2022; &#x2022;</div>
  </div>

  {#if surah.surah_number !== 9}
    <div class="bismillah-section">
      <div class="bismillah-ar" dir="rtl">بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ</div>
      <div class="bismillah-en">In the Name of Allah — the Most Compassionate, Most Merciful</div>
    </div>
  {/if}
</div>

<style>
  .surah-header {
    margin-bottom: 24px;
  }
  .header-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 28px 32px 20px;
    text-align: center;
    position: relative;
    overflow: hidden;
  }
  .ornament-top {
    font-size: 1.4rem;
    color: var(--accent)40;
    letter-spacing: 8px;
    margin-bottom: 8px;
  }
  .ornament-bottom {
    font-size: 0.6rem;
    color: var(--accent)30;
    letter-spacing: 12px;
    margin-top: 12px;
  }
  .surah-name-ar {
    font-family: 'Amiri Quran', 'Amiri', 'Noto Naskh Arabic', serif;
    font-size: 3.2rem;
    color: var(--accent);
    line-height: 1.6;
    margin-bottom: 8px;
    text-shadow: 0 1px 2px var(--accent-muted);
    text-align: center;
  }
  .surah-info {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .surah-translit {
    font-size: 1.2rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .info-sep {
    color: var(--text-muted);
    font-size: 0.8rem;
  }
  .surah-meaning {
    font-size: 0.95rem;
    color: var(--text-secondary);
    font-style: italic;
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
  .meta-text {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .meta-dot { color: var(--text-muted); }

  /* Bismillah */
  .bismillah-section {
    text-align: center;
    padding: 28px 0 16px;
  }
  .bismillah-ar {
    font-family: 'Amiri Quran', 'Amiri', 'Noto Naskh Arabic', serif;
    font-size: 2.2rem;
    color: var(--accent);
    margin-bottom: 8px;
    line-height: 2;
    text-shadow: 0 1px 2px var(--accent-muted);
    letter-spacing: 1px;
    text-align: center;
  }
  .bismillah-en {
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
    letter-spacing: 0.3px;
  }

  @media (max-width: 640px) {
    .header-card { padding: 20px 16px 16px; }
    .surah-name-ar { font-size: 2.4rem; }
    .surah-info { flex-direction: column; gap: 2px; }
    .info-sep { display: none; }
    .bismillah-ar { font-size: 1.8rem; }
  }
</style>
