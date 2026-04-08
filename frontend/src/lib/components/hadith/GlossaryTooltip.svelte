<script lang="ts">
  import type { GlossaryTerm } from '$lib/types';
  import glossaryData from '$lib/data/mustalah-glossary.json';

  let { termId, children }: { termId: string; children: any } = $props();

  let open = $state(false);

  const term: GlossaryTerm | undefined = (glossaryData as GlossaryTerm[]).find(
    (t) => t.id === termId
  );

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    open = !open;
  }

  function close() {
    open = false;
  }

  const categoryColors: Record<string, string> = {
    grade: 'var(--success)',
    breadth: 'var(--accent)',
    chain: 'var(--warning)',
    narrator: 'var(--error)',
    defect: 'var(--error)',
    corroboration: 'var(--success)',
    ascription: 'var(--accent-secondary)',
  };
</script>

<svelte:window on:click={close} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="glossary-wrapper" onclick={toggle}>
  <span class="glossary-trigger">{@render children()}</span>

  {#if open && term}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="glossary-popup" onclick={(e) => e.stopPropagation()}>
      <button class="glossary-close" onclick={close}>&times;</button>

      <div class="glossary-header">
        <span class="glossary-ar">{term.term_ar}</span>
        <span class="glossary-en">{term.term_en}</span>
        <span class="glossary-cat" style="color: {categoryColors[term.category] ?? 'var(--text-muted)'}">
          {term.category}
        </span>
      </div>

      {#if term.literal_meaning}
        <div class="glossary-row">
          <span class="glossary-label">Literally</span>
          <span>{term.literal_meaning}</span>
        </div>
      {/if}

      <div class="glossary-row">
        <span class="glossary-label">Definition</span>
        <span>{term.technical_definition}</span>
      </div>

      {#if term.conditions && term.conditions.length > 0}
        <div class="glossary-row">
          <span class="glossary-label">Conditions</span>
          <ul class="glossary-conditions">
            {#each term.conditions as condition}
              <li>{condition}</li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if term.ruling}
        <div class="glossary-row">
          <span class="glossary-label">Ruling</span>
          <span class="glossary-ruling">{term.ruling}</span>
        </div>
      {/if}

      {#if term.page}
        <div class="glossary-footer">
          Ref: p.{term.page} — <em>Mustalah al-Hadeeth Made Easy</em>
        </div>
      {/if}
    </div>
  {/if}
</span>

<style>
  .glossary-wrapper {
    position: relative;
    display: inline;
  }
  .glossary-trigger {
    cursor: help;
    border-bottom: 1px dotted var(--accent);
  }
  .glossary-popup {
    position: absolute;
    z-index: 100;
    top: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
    width: 340px;
    max-width: 90vw;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.12);
  }
  .glossary-close {
    position: absolute;
    top: 8px;
    right: 10px;
    font-size: 1.1rem;
    color: var(--text-muted);
    cursor: pointer;
    background: none;
    border: none;
    line-height: 1;
  }
  .glossary-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 10px;
    flex-wrap: wrap;
  }
  .glossary-ar {
    font-family: var(--font-arabic);
    font-size: 1.2rem;
    color: var(--accent);
    font-weight: 600;
  }
  .glossary-en {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .glossary-cat {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 700;
  }
  .glossary-row {
    margin-bottom: 8px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .glossary-label {
    display: block;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 2px;
  }
  .glossary-conditions {
    margin: 4px 0 0 16px;
    padding: 0;
    font-size: 0.82rem;
  }
  .glossary-conditions li {
    margin-bottom: 2px;
  }
  .glossary-ruling {
    font-style: italic;
  }
  .glossary-footer {
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
