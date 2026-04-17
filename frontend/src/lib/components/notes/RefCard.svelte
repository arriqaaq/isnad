<script lang="ts">
  import type { NoteRef } from '$lib/types';
  import EmbeddedRef from './EmbeddedRef.svelte';

  let { ref, onupdateannotation, onremove }: {
    ref: NoteRef;
    onupdateannotation?: (annotation: string) => void;
    onremove?: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally — intentional: one-shot init from prop
  let showAnnotation = $state(!!ref.annotation);
  // svelte-ignore state_referenced_locally
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
    padding: 0 16px 10px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-top: none;
    border-radius: 0 0 var(--radius-xl) var(--radius-xl);
    margin-top: -1px;
  }
  .add-annotation-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-size: 0.8rem;
    cursor: pointer;
    padding: 8px 0;
    font-style: italic;
    transition: color var(--transition);
  }
  .add-annotation-btn:hover { color: var(--accent); }
  .annotation-input {
    width: 100%;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: 10px 12px;
    font-family: var(--font-serif);
    font-size: 0.85rem;
    line-height: 1.6;
    color: var(--text-primary);
    background: var(--note-editor-bg);
    resize: vertical;
    outline: none;
    box-sizing: border-box;
    margin-top: 6px;
    transition: border-color var(--transition), box-shadow var(--transition);
  }
  .annotation-input:focus {
    border-color: var(--accent-muted);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }
</style>
