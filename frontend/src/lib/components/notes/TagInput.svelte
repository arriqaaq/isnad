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
    gap: 6px;
    align-items: center;
    padding: 6px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    background: var(--bg-surface);
    min-height: 36px;
    transition: border-color var(--transition), box-shadow var(--transition);
  }
  .tags-row:focus-within {
    border-color: var(--gold-accent-muted);
    box-shadow: 0 0 0 3px var(--gold-accent-muted);
  }
  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 3px 10px;
    background: var(--gold-accent-muted);
    color: var(--gold-accent);
    border-radius: 12px;
    font-size: 0.72rem;
    font-weight: 600;
  }
  .tag-remove {
    background: none;
    border: none;
    color: var(--gold-accent);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0 2px;
    line-height: 1;
    opacity: 0.6;
    transition: opacity var(--transition);
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
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgba(0,0,0,0.12);
    z-index: 100;
    margin-top: 4px;
    overflow: hidden;
  }
  .suggestion-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 14px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.8rem;
    cursor: pointer;
    transition: background var(--transition);
  }
  .suggestion-item:hover {
    background: var(--bg-hover);
  }
</style>
