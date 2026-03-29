<script lang="ts">
  import { goto } from '$app/navigation';

  let searchQuery = $state('');

  function handleSearch(e: Event) {
    e.preventDefault();
    if (searchQuery.trim()) {
      goto(`/search?q=${encodeURIComponent(searchQuery.trim())}`);
    }
  }
</script>

<header class="topbar">
  <div class="breadcrumb"></div>

  <form class="search-form" onsubmit={handleSearch}>
    <span class="search-icon">⌕</span>
    <input
      type="text"
      placeholder="Search hadiths, narrators..."
      bind:value={searchQuery}
      class="search-input"
    />
  </form>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 20px;
    flex-shrink: 0;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .search-form {
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0 12px;
    width: 320px;
    transition: border-color var(--transition);
  }

  .search-form:focus-within {
    border-color: var(--accent);
  }

  .search-icon {
    color: var(--text-muted);
    font-size: 1rem;
    margin-right: 8px;
  }

  .search-input {
    border: none;
    background: transparent;
    padding: 8px 0;
    width: 100%;
    font-size: 0.85rem;
  }

  .search-input:focus {
    border-color: transparent;
  }
</style>
