<script lang="ts">
  import { getSurah, getHadith, getNarrator } from '$lib/api';
  import { language } from '$lib/stores/language';
  import { stripHtml } from '$lib/utils';

  let { refType, refId }: {
    refType: 'ayah' | 'hadith' | 'narrator';
    refId: string;
  } = $props();

  let textAr = $state('');
  let textEn = $state('');
  let label = $state('');
  let loading = $state(true);
  let failed = $state(false);

  let href = $derived(
    refType === 'ayah'
      ? `/quran/${refId.split(':')[0]}`
      : refType === 'hadith'
        ? `/hadiths/${encodeURIComponent(refId)}`
        : `/narrators/${encodeURIComponent(refId)}`
  );

  let typeLabel = $derived(
    refType === 'ayah' ? 'Quran' : refType === 'hadith' ? 'Hadith' : 'Narrator'
  );

  // Show text based on language preference
  let displayText = $derived.by(() => {
    const lang = $language;
    if (lang === 'ar') return textAr || textEn;
    return textEn || textAr;
  });

  let isArabicDisplay = $derived.by(() => {
    const lang = $language;
    if (lang === 'ar') return true;
    return !textEn && !!textAr;
  });

  function decodeEntities(text: string): string {
    return text
      .replace(/&quot;/g, '"')
      .replace(/&amp;/g, '&')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&#39;/g, "'")
      .replace(/&#x27;/g, "'");
  }

  function truncate(text: string, max: number): string {
    if (text.length <= max) return text;
    return text.slice(0, max).trimEnd() + '...';
  }

  $effect(() => {
    loading = true;
    failed = false;
    textAr = '';
    textEn = '';

    if (refType === 'ayah') {
      const [s, a] = refId.split(':').map(Number);
      label = `${s}:${a}`;
      getSurah(s).then(res => {
        const ayah = res.ayahs.find((ay: any) => ay.ayah_number === a);
        if (ayah) {
          textAr = ayah.text_ar ?? '';
          textEn = ayah.text_en ?? '';
        } else {
          failed = true;
        }
      }).catch(() => { failed = true; })
        .finally(() => { loading = false; });
    } else if (refType === 'hadith') {
      label = refId;
      getHadith(refId).then(res => {
        const h = res.hadith;
        // Use matn if available, otherwise truncate full text; decode HTML entities
        const rawAr = h.matn ?? h.text_ar ?? '';
        const rawEn = h.text_en ?? '';
        textAr = truncate(decodeEntities(stripHtml(rawAr)), 250);
        textEn = truncate(decodeEntities(stripHtml(rawEn)), 250);
        label = `${h.book_name ?? 'Hadith'} #${h.hadith_number}`;
      }).catch(() => { failed = true; })
        .finally(() => { loading = false; });
    } else if (refType === 'narrator') {
      label = refId;
      getNarrator(refId).then(res => {
        label = res.narrator.name_en ?? res.narrator.name_ar ?? refId;
        textEn = res.narrator.bio ?? '';
      }).catch(() => { failed = true; })
        .finally(() => { loading = false; });
    } else {
      label = refId;
      loading = false;
    }
  });
</script>

<div class="embedded-ref">
  <a {href} class="ref-header">
    <span class="ref-badge {refType}">{typeLabel}</span>
    <span class="ref-label">{label}</span>
    <span class="ref-arrow">&rarr;</span>
  </a>

  {#if loading}
    <div class="ref-body loading-body">Loading...</div>
  {:else if failed}
    <div class="ref-body muted">Could not load reference</div>
  {:else if displayText}
    <div class="ref-body" class:rtl={isArabicDisplay} dir={isArabicDisplay ? 'rtl' : 'ltr'}>
      {displayText}
    </div>
  {/if}
</div>

<style>
  .embedded-ref {
    margin: 8px 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
    background: var(--bg-surface);
    box-shadow: var(--shadow-card);
    transition: box-shadow var(--transition);
  }
  .embedded-ref:hover {
    box-shadow: var(--shadow-card-hover);
  }
  .ref-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    background: var(--bg-hover);
    text-decoration: none;
    color: inherit;
    border-bottom: 1px solid var(--border-subtle);
    transition: background var(--transition);
  }
  .ref-header:hover {
    background: var(--gold-accent-muted);
  }
  .ref-badge {
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 10px;
    border-radius: 10px;
  }
  .ref-badge.ayah {
    background: var(--gold-accent-muted);
    color: var(--gold-accent);
  }
  .ref-badge.hadith {
    background: var(--bg-active);
    color: var(--text-primary);
  }
  .ref-badge.narrator {
    background: var(--accent-muted);
    color: var(--accent);
  }
  .ref-label {
    font-size: 0.75rem;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }
  .ref-arrow {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 0.75rem;
    transition: transform var(--transition);
  }
  .ref-header:hover .ref-arrow {
    transform: translateX(2px);
  }
  .ref-body {
    padding: 12px 16px;
    font-family: var(--font-serif);
    font-size: 0.9rem;
    line-height: 1.7;
    color: var(--text-secondary);
  }
  .ref-body.rtl {
    font-family: var(--font-arabic-text, 'Noto Naskh Arabic', serif);
    font-size: 1.05rem;
    line-height: 2;
    text-align: right;
    color: var(--text-primary);
  }
  .loading-body {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-style: italic;
  }
  .muted {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
