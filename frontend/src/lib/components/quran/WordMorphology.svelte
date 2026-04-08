<script lang="ts">
  import type { ApiQuranWord } from '$lib/types';

  let { word, onclose }: {
    word: ApiQuranWord;
    onclose: () => void;
  } = $props();

  const POS_LABELS: Record<string, string> = {
    N: 'Noun', V: 'Verb', P: 'Particle',
  };

  const FEATURE_LABELS: Record<string, Record<string, string>> = {
    gender: { M: 'Masculine', F: 'Feminine' },
    number: { S: 'Singular', D: 'Dual', P: 'Plural' },
    case: { NOM: 'Nominative', GEN: 'Genitive', ACC: 'Accusative' },
    aspect: { PERF: 'Perfect', IMPF: 'Imperfect', IMPV: 'Imperative' },
    voice: { PASS: 'Passive' },
    derivation: { ACT_PCPL: 'Active Participle', PASS_PCPL: 'Passive Participle' },
    type: { ADJ: 'Adjective', PN: 'Proper Noun', PRON: 'Pronoun', REL: 'Relative', DEM: 'Demonstrative', CONJ: 'Conjunction', DET: 'Determiner', NV: 'Verbal Noun' },
  };

  function featureLabel(key: string, val: string): string {
    return FEATURE_LABELS[key]?.[val] ?? val;
  }

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('morph-backdrop')) {
      onclose();
    }
  }

  let segments: { pos: string; text?: string; affix?: string; root?: string }[] = $derived(
    word.segments ?? []
  );
</script>

<div class="morph-inline">
  <div class="morph-popup">
    <button class="morph-close" onclick={onclose}>&times;</button>

    <div class="morph-word" dir="rtl">{word.text_ar}</div>

    {#if word.translation}
      <div class="morph-meaning">{word.translation}</div>
    {/if}

    {#if word.transliteration}
      <div class="morph-translit">{word.transliteration}</div>
    {/if}

    <div class="morph-pos">{POS_LABELS[word.pos] ?? word.pos}</div>

    <div class="morph-details">
      {#if word.root}
        <div class="morph-row">
          <span class="morph-label">Root</span>
          <a class="morph-value root-link" href="/quran/root/{encodeURIComponent(word.root)}">{word.root}</a>
        </div>
      {/if}
      {#if word.lemma}
        <div class="morph-row">
          <span class="morph-label">Lemma</span>
          <span class="morph-value" dir="rtl">{word.lemma}</span>
        </div>
      {/if}

      {#if word.features}
        {#each Object.entries(word.features).filter(([k]) => k !== 'type' || word.pos !== 'P') as [key, val]}
          <div class="morph-row">
            <span class="morph-label">{key}</span>
            <span class="morph-value">{featureLabel(key, val)}</span>
          </div>
        {/each}
      {/if}
    </div>

    {#if segments.length > 1}
      <div class="morph-segments">
        <div class="morph-seg-label">Morphemes</div>
        <div class="morph-seg-list" dir="rtl">
          {#each segments as seg}
            <span class="morph-seg" class:prefix={seg.affix === 'PREF'} class:suffix={seg.affix === 'SUFF'}>
              <span class="seg-text">{seg.text ?? ''}</span>
              <span class="seg-pos">{seg.pos}{seg.affix ? ` (${seg.affix})` : ''}</span>
            </span>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .morph-inline {
    padding: 8px;
  }
  .morph-popup {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 24px;
    position: relative;
    box-shadow: 0 2px 12px rgba(0,0,0,0.08);
  }
  .morph-close {
    position: absolute;
    top: 8px;
    right: 12px;
    font-size: 1.4rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .morph-word {
    font-size: 2.4rem;
    text-align: center;
    color: var(--text-primary);
    margin-bottom: 4px;
    line-height: 1.4;
  }
  .morph-meaning {
    text-align: center;
    font-size: 0.95rem;
    color: var(--text-secondary);
    margin-bottom: 2px;
  }
  .morph-translit {
    text-align: center;
    font-size: 0.85rem;
    color: var(--text-muted);
    font-style: italic;
    margin-bottom: 12px;
  }
  .morph-pos {
    text-align: center;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    margin-bottom: 16px;
  }
  .morph-details {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .morph-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
  }
  .morph-label {
    color: var(--text-muted);
    text-transform: capitalize;
  }
  .morph-value {
    color: var(--text-primary);
    font-weight: 500;
  }
  .root-link {
    color: var(--accent);
    text-decoration: none;
    font-size: 1.1rem;
  }
  .root-link:hover {
    text-decoration: underline;
  }
  .morph-segments {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .morph-seg-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 8px;
  }
  .morph-seg-list {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .morph-seg {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: var(--bg-hover);
    border: 1px solid var(--border);
  }
  .morph-seg.prefix {
    border-color: var(--accent);
    background: var(--accent-muted);
  }
  .morph-seg.suffix {
    border-color: var(--success);
    background: rgba(76, 175, 80, 0.1);
  }
  .seg-text {
    font-size: 1.2rem;
    color: var(--text-primary);
  }
  .seg-pos {
    font-size: 0.65rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
</style>
