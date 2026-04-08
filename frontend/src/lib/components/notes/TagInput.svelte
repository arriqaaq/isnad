<script lang="ts">
  import { fetchNoteTags } from '$lib/api';

  let { tags = [], onchange }: {
    tags?: string[];
    onchange: (tags: string[]) => void;
  } = $props();

  let input = $state('');
  let suggestions: string[] = $state([]);
  let allTags: string[] = $state([]);
  let showSuggestions = $state(false);

  $effect(() => {
    fetchNoteTags().then(t => { allTags = t; }).catch(() => {});
  });

  function addTag(tag: string) {
    const t = tag.trim().toLowerCase();
    if (t && !tags.includes(t)) {
      const updated = [...tags, t];
      onchange(updated);
    }
    input = '';
    showSuggestions = false;
  }

  function removeTag(tag: string) {
    onchange(tags.filter(t => t !== tag));
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.key === 'Enter' || e.key === ',') && input.trim()) {
      e.preventDefault();
      addTag(input);
    }
    if (e.key === 'Backspace' && !input && tags.length > 0) {
      removeTag(tags[tags.length - 1]);
    }
  }

  function handleInput() {
    if (input.trim()) {
      suggestions = allTags
        .filter(t => t.includes(input.toLowerCase()) && !tags.includes(t))
        .slice(0, 5);
      showSuggestions = suggestions.length > 0;
    } else {
      showSuggestions = false;
    }
  }
</script>

<div class="tag-input-wrapper">
  <div class="tags-row">
    {#each tags as tag}
      <span class="tag-pill">
        {tag}
        <button class="tag-remove" onclick={() => removeTag(tag)}>&times;</button>
      </span>
    {/each}
    <input
      type="text"
      class="tag-text-input"
      placeholder={tags.length === 0 ? 'Add tags...' : ''}
      bind:value={input}
      oninput={handleInput}
      onkeydown={handleKeydown}
      onfocus={() => handleInput()}
      onblur={() => setTimeout(() => { showSuggestions = false; }, 150)}
    />
  </div>
  {#if showSuggestions}
    <div class="suggestions">
      {#each suggestions as s}
        <button class="suggestion-item" onmousedown={() => addTag(s)}>{s}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tag-input-wrapper {
    position: relative;
  }
  .tags-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    min-height: 32px;
  }
  .tags-row:focus-within {
    border-color: var(--accent);
  }
  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px 8px;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 10px;
    font-size: 0.7rem;
    font-weight: 600;
  }
  .tag-remove {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0 2px;
    line-height: 1;
    opacity: 0.6;
  }
  .tag-remove:hover { opacity: 1; }
  .tag-text-input {
    flex: 1;
    min-width: 60px;
    border: none;
    outline: none;
    background: transparent;
    font-size: 0.8rem;
    color: var(--text-primary);
    padding: 2px 0;
  }
  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
    z-index: 100;
    margin-top: 2px;
  }
  .suggestion-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .suggestion-item:hover {
    background: var(--bg-hover);
  }
</style>
