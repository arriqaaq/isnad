<script lang="ts">
  import type { NoteRef } from '$lib/types';
  import EmbeddedRef from './EmbeddedRef.svelte';

  let { ref, onupdateannotation, onremove }: {
    ref: NoteRef;
    onupdateannotation?: (annotation: string) => void;
    onremove?: () => void;
  } = $props();

  let showAnnotation = $state(!!ref.annotation);
  let annotationText = $state(ref.annotation ?? '');

  function handleAnnotationSave() {
    onupdateannotation?.(annotationText);
  }
</script>

<div class="ref-card-wrapper">
  {#if onremove}
    <button class="remove-btn" onclick={onremove} aria-label="Remove">&times;</button>
  {/if}

  <EmbeddedRef refType={ref.ref_type} refId={ref.ref_id} />

  <div class="annotation-area">
    {#if showAnnotation}
      <textarea
        class="annotation-input"
        bind:value={annotationText}
        placeholder="Your thoughts on this..."
        rows="2"
        onblur={handleAnnotationSave}
      ></textarea>
    {:else}
      <button class="add-annotation-btn" onclick={() => { showAnnotation = true; }}>
        + Add a thought...
      </button>
    {/if}
  </div>
</div>

<style>
  .ref-card-wrapper {
    position: relative;
  }
  .remove-btn {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 1;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 50%;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.85rem;
    line-height: 1;
    opacity: 0;
    transition: all var(--transition);
  }
  .ref-card-wrapper:hover .remove-btn { opacity: 1; }
  .remove-btn:hover { color: var(--error); border-color: var(--error); }
  .annotation-area {
    padding: 0 12px 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 var(--radius) var(--radius);
    margin-top: -1px;
  }
  .add-annotation-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    padding: 6px 0;
    font-style: italic;
  }
  .add-annotation-btn:hover { color: var(--accent); }
  .annotation-input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 8px;
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--text-primary);
    background: var(--bg-primary);
    resize: vertical;
    outline: none;
    font-family: inherit;
    box-sizing: border-box;
    margin-top: 6px;
  }
  .annotation-input:focus { border-color: var(--accent); }
</style>
