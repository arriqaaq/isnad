<script lang="ts">
  import { page } from '$app/state';

  const featuredItems = [
    { path: '/explore', label: 'Explore', icon: '✦' },
    { path: '/ask', label: 'Ask', icon: '◇' },
  ];

  const hadithItems = [
    { path: '/hadiths', label: 'Hadiths', icon: '☰' },
    { path: '/narrators', label: 'Narrators', icon: '◎' },
    { path: '/books', label: 'Books', icon: '▤' },
    { path: '/families', label: 'Families', icon: '⬢' },
    { path: '/search', label: 'Search', icon: '⌕' },
    { path: '/analysis', label: 'Analysis', icon: '△' },
  ];

  const quranItems = [
    { path: '/quran', label: 'Quran', icon: '◈' },
    { path: '/quran/search', label: 'Quran Search', icon: '⌕' },
  ];

  const noteItems = [
    { path: '/notes', label: 'Notes', icon: '✎' },
  ];

  function isActive(path: string): boolean {
    const current = page.url.pathname;
    if (path === '/') return current === '/';
    return current === path || current.startsWith(path + '/');
  }
</script>

<nav class="sidebar">
  <a href="/" class="sidebar-header">
    <span class="logo">◆</span>
    <span class="logo-text">Ilm</span>
  </a>

  <div class="nav-items">
    {#each featuredItems as item}
      <a
        href={item.path}
        class="nav-item featured"
        class:active={isActive(item.path)}
      >
        <span class="nav-icon">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
      </a>
    {/each}

    <div class="nav-divider"></div>

    {#each quranItems as item}
      <a
        href={item.path}
        class="nav-item"
        class:active={isActive(item.path)}
      >
        <span class="nav-icon">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
      </a>
    {/each}

    <div class="nav-divider"></div>

    {#each noteItems as item}
      <a
        href={item.path}
        class="nav-item"
        class:active={isActive(item.path)}
      >
        <span class="nav-icon">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
      </a>
    {/each}

    <div class="nav-divider"></div>

    {#each hadithItems as item}
      <a
        href={item.path}
        class="nav-item"
        class:active={isActive(item.path)}
      >
        <span class="nav-icon">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
      </a>
    {/each}
  </div>

  <div class="sidebar-footer">
    <span class="footer-text">Islamic Knowledge Platform</span>
  </div>
</nav>

<style>
  .sidebar {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--bg-primary);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 20px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--topbar-height);
    text-decoration: none;
    color: inherit;
    transition: opacity var(--transition);
  }
  .sidebar-header:hover {
    opacity: 0.85;
    color: inherit;
  }

  .logo {
    color: var(--accent);
    font-size: 1.1rem;
  }

  .logo-text {
    font-family: var(--font-serif);
    font-weight: 600;
    font-size: 1.15rem;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .nav-items {
    flex: 1;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
  }

  .nav-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 8px 12px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius);
    color: var(--text-secondary);
    transition: all var(--transition);
    font-family: var(--font-serif);
    font-size: 0.95rem;
    text-decoration: none;
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--accent-muted);
    color: var(--accent);
    font-weight: 600;
  }

  .nav-item.featured {
    font-weight: 600;
    color: var(--text-primary);
  }

  .nav-icon {
    width: 20px;
    text-align: center;
    font-size: 0.95rem;
  }

  .sidebar-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .footer-text {
    font-family: var(--font-serif);
    font-size: 0.75rem;
    font-style: italic;
    color: var(--text-muted);
  }
</style>
