<script lang="ts">
  import { AYAH_COUNTS, parseVerseRef } from '$lib/constants/ayahCounts';

  let { surah, ayah, onsubmit }: {
    surah: number;
    ayah: number;
    onsubmit: (v: { surah: number; ayah: number }) => void;
  } = $props();

  // svelte-ignore state_referenced_locally — the $effect below re-syncs on prop change.
  let input = $state(`${surah}:${ayah}`);
  let error: string | null = $state(null);

  $effect(() => {
    input = `${surah}:${ayah}`;
  });

  function handleSubmit(e: Event) {
    e.preventDefault();
    const parsed = parseVerseRef(input);
    if (!parsed) {
      error = 'Enter a verse like "2:255" (surah 1–114, ayah within bounds).';
      return;
    }
    error = null;
    onsubmit(parsed);
  }

  function stepAyah(delta: 1 | -1) {
    const max = AYAH_COUNTS[surah] ?? 1;
    const next = ayah + delta;
    if (next < 1 || next > max) return;
    onsubmit({ surah, ayah: next });
  }
</script>

<form class="verse-picker" onsubmit={handleSubmit}>
  <div class="picker-row">
    <label class="picker-label" for="verse-input">Verse</label>
    <input
      id="verse-input"
      class="verse-input"
      type="text"
      inputmode="numeric"
      placeholder="e.g. 2:255"
      bind:value={input}
      autocomplete="off"
      spellcheck="false"
    />
    <button class="go-btn" type="submit">Go</button>

    <div class="nav-spacer" aria-hidden="true"></div>

    <button
      class="step-btn"
      type="button"
      onclick={() => stepAyah(-1)}
      disabled={ayah <= 1}
      title="Previous ayah"
    >‹</button>
    <span class="ref-chip">{surah}:{ayah}</span>
    <button
      class="step-btn"
      type="button"
      onclick={() => stepAyah(1)}
      disabled={ayah >= (AYAH_COUNTS[surah] ?? 1)}
      title="Next ayah"
    >›</button>
  </div>
  {#if error}
    <div class="picker-error">{error}</div>
  {/if}
</form>

<style>
  .verse-picker {
    padding: 14px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }
  .picker-row {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .picker-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .verse-input {
    flex: 0 1 140px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.9rem;
    font-family: var(--font-mono);
  }
  .verse-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .go-btn {
    padding: 6px 14px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--bg-primary);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity var(--transition);
  }
  .go-btn:hover { opacity: 0.9; }

  .nav-spacer { flex: 1; min-width: 12px; }

  .step-btn {
    width: 28px;
    height: 28px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-secondary);
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
    transition: all var(--transition);
  }
  .step-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .step-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .ref-chip {
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: var(--accent);
    background: var(--accent-muted);
    padding: 3px 10px;
    border-radius: 10px;
  }
  .picker-error {
    margin-top: 8px;
    font-size: 0.8rem;
    color: var(--danger, #c33);
  }
</style>
