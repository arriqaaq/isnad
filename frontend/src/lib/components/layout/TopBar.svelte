<script lang="ts">
  import { goto } from '$app/navigation';
  import LanguageToggle from './LanguageToggle.svelte';
  import QuranSettings from './QuranSettings.svelte';

  let { onToggleSidebar }: { onToggleSidebar?: () => void } = $props();

  let searchQuery = $state('');

  function handleSearch(e: Event) {
    e.preventDefault();
    if (searchQuery.trim()) {
      goto(`/search?q=${encodeURIComponent(searchQuery.trim())}`);
    }
  }
</script>

<header class="topbar">
  <div class="topbar-left">
    {#if onToggleSidebar}
      <button class="hamburger" onclick={onToggleSidebar} aria-label="Menu">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <line x1="3" y1="6" x2="21" y2="6"/>
          <line x1="3" y1="12" x2="21" y2="12"/>
          <line x1="3" y1="18" x2="21" y2="18"/>
        </svg>
      </button>
    {/if}
    <a href="/" class="home-btn" title="Home">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
        <polyline points="9 22 9 12 15 12 15 22"/>
      </svg>
    </a>
  </div>

  <div class="topbar-right">
    <form class="search-form" onsubmit={handleSearch}>
      <span class="search-icon">&#x2315;</span>
      <input
        type="text"
        placeholder="Search hadiths, narrators..."
        bind:value={searchQuery}
        class="search-input"
      />
    </form>
    <QuranSettings />
    <LanguageToggle />
  </div>
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
    gap: 12px;
  }

  .topbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .hamburger {
    display: none;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    border-radius: var(--radius);
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    transition: all var(--transition);
  }
  .hamburger:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .home-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    color: var(--text-secondary);
    transition: all var(--transition);
  }
  .home-btn:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    flex: 1;
    justify-content: flex-end;
  }

  .search-form {
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0 12px;
    flex: 1;
    max-width: 320px;
    min-width: 0;
    transition: border-color var(--transition);
  }

  .search-form:focus-within {
    border-color: var(--accent);
  }

  .search-icon {
    color: var(--text-muted);
    font-size: 1rem;
    margin-right: 8px;
    flex-shrink: 0;
  }

  .search-input {
    border: none;
    background: transparent;
    padding: 8px 0;
    width: 100%;
    font-size: 0.85rem;
    min-width: 0;
  }

  .search-input:focus {
    border-color: transparent;
  }

  @media (max-width: 768px) {
    .topbar { padding: 0 12px; }
    .hamburger { display: flex; }
    .home-btn { display: none; }
    .search-form { display: none; }
  }
</style>
