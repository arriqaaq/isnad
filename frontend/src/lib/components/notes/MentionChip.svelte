<script lang="ts">
  let { refType, refId, label }: {
    refType: 'ayah' | 'hadith' | 'narrator';
    refId: string;
    label?: string;
  } = $props();

  let href = $derived(
    refType === 'ayah'
      ? `/quran/${refId.split(':')[0]}`
      : refType === 'hadith'
        ? `/hadiths/${encodeURIComponent(refId)}`
        : `/narrators/${encodeURIComponent(refId)}`
  );

  let displayLabel = $derived(
    label ?? (refType === 'ayah' ? refId : refType === 'hadith' ? `#${refId}` : refId)
  );

  let icon = $derived(
    refType === 'ayah' ? '\u25C8' : refType === 'hadith' ? '\u2630' : '\u25CE'
  );
</script>

<a class="mention-chip mention-{refType}" {href}>
  <span class="mention-icon">{icon}</span>
  {displayLabel}
</a>

<style>
  .mention-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 8px;
    border-radius: 10px;
    font-size: 0.75rem;
    font-family: var(--font-mono);
    text-decoration: none;
    transition: all var(--transition);
    vertical-align: baseline;
  }
  .mention-ayah {
    background: var(--accent-muted);
    color: var(--accent);
  }
  .mention-hadith {
    background: var(--bg-hover);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }
  .mention-narrator {
    background: var(--accent-muted);
    color: var(--accent);
  }
  .mention-chip:hover {
    opacity: 0.8;
  }
  .mention-icon {
    font-size: 0.65rem;
  }
</style>
