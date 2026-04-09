<script lang="ts">
  import type { UserNote } from '$lib/types';
  import TagInput from './TagInput.svelte';
  import RichEditor from './RichEditor.svelte';

  let { note, startExpanded = false, initialContent = '', onsave, oncancel }: {
    note?: UserNote;
    startExpanded?: boolean;
    initialContent?: string;
    onsave: (data: { content: string; color: string; tags: string[] }) => void;
    oncancel?: () => void;
  } = $props();

  const COLORS = ['yellow', 'green', 'blue', 'pink', 'purple'] as const;

  let expanded = $state(!!note || startExpanded);
  let content = $state(note?.content ?? initialContent);
  let selectedColor = $state(note?.color ?? 'yellow');
  let tags = $state<string[]>(note?.tags ?? []);

  function handleColorClick(color: string) {
    selectedColor = color;
    if (!expanded) {
      onsave({ content: '', color, tags: [] });
    }
  }

  function handleSave() {
    onsave({ content, color: selectedColor, tags });
    if (!note) {
      content = '';
      tags = [];
      expanded = false;
    }
  }

  function handleCancel() {
    if (note) {
      content = note.content;
      selectedColor = note.color;
      tags = note.tags;
    } else {
      content = '';
      expanded = false;
    }
    oncancel?.();
  }
</script>

<div class="note-editor">
  {#if !expanded}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="editor-collapsed" onclick={() => { expanded = true; }}>
      <span class="placeholder-text">Write a note...</span>
      <div class="color-row">
        {#each COLORS as color}
          <button
            class="color-circle"
            class:selected={selectedColor === color}
            style="background: var(--note-{color})"
            onclick={(e) => { e.stopPropagation(); handleColorClick(color); }}
            aria-label="Highlight {color}"
          ></button>
        {/each}
      </div>
    </div>
  {:else}
    <div class="editor-expanded">
      <RichEditor
        value={content}
        onchange={(v) => { content = v; }}
        placeholder="Write your thoughts... Type @ to embed ayahs (@2:255), hadiths (@im_1), or narrators"
      />

      <div class="editor-toolbar">
        <div class="color-row">
          {#each COLORS as color}
            <button
              class="color-circle"
              class:selected={selectedColor === color}
              style="background: var(--note-{color})"
              onclick={() => { selectedColor = color; }}
              aria-label="{color}"
            ></button>
          {/each}
        </div>
        <div class="tag-row">
          <TagInput {tags} onchange={(t) => { tags = t; }} />
        </div>
        <div class="button-row">
          <button class="btn-save" onclick={handleSave}>Save</button>
          <button class="btn-cancel" onclick={handleCancel}>Cancel</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .note-editor {
    width: 100%;
  }
  .editor-collapsed {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 20px;
    background: var(--note-editor-bg);
    border: 1px solid transparent;
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-card);
    cursor: text;
    transition: all var(--transition);
  }
  .editor-collapsed:hover {
    border-color: var(--accent-muted);
    box-shadow: var(--shadow-card), 0 0 0 3px var(--accent-muted);
  }
  .placeholder-text {
    font-family: var(--font-serif);
    font-size: 0.95rem;
    font-style: italic;
    color: var(--text-muted);
  }
  .color-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .color-circle {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
  }
  .color-circle:hover {
    transform: scale(1.2);
  }
  .color-circle.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }
  .editor-expanded {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-editor);
    overflow: hidden;
  }
  .editor-toolbar {
    padding: 10px 16px;
    background: var(--bg-primary);
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .tag-row {
    flex: 1;
    min-width: 120px;
  }
  .button-row {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-left: auto;
  }
  .btn-save {
    padding: 5px 20px;
    font-size: 0.8rem;
    font-weight: 600;
    color: #fff;
    background: var(--accent);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    transition: background var(--transition);
  }
  .btn-save:hover {
    background: var(--accent-hover);
  }
  .btn-cancel {
    padding: 5px 14px;
    font-size: 0.8rem;
    color: var(--text-muted);
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: all var(--transition);
  }
  .btn-cancel:hover { color: var(--accent); border-color: var(--accent-muted); }
</style>
