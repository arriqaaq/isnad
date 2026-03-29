<script lang="ts">
  import { page } from '$app/state';

  const navItems = [
    { path: '/', label: 'Dashboard', icon: '⬡' },
    { path: '/hadiths', label: 'Hadiths', icon: '☰' },
    { path: '/narrators', label: 'Narrators', icon: '◎' },
    { path: '/books', label: 'Books', icon: '▤' },
    { path: '/families', label: 'Families', icon: '⬢' },
    { path: '/search', label: 'Search', icon: '⌕' },
    { path: '/ask', label: 'Ask', icon: '◇' },
    { path: '/analysis', label: 'Analysis', icon: '△' },
  ];

  function isActive(path: string): boolean {
    const current = page.url.pathname;
    if (path === '/') return current === '/';
    return current.startsWith(path);
  }
</script>

<nav class="sidebar">
  <div class="sidebar-header">
    <span class="logo">◆</span>
    <span class="logo-text">Hadith Explorer</span>
  </div>

  <div class="nav-items">
    {#each navItems as item}
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
    <span class="footer-text">Sahih al-Bukhari</span>
  </div>
</nav>

<style>
  .sidebar {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--topbar-height);
  }

  .logo {
    color: var(--accent);
    font-size: 1.3rem;
  }

  .logo-text {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--text-primary);
  }

  .nav-items {
    flex: 1;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius);
    color: var(--text-secondary);
    transition: all var(--transition);
    font-size: 0.9rem;
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .nav-icon {
    width: 20px;
    text-align: center;
    font-size: 1rem;
  }

  .sidebar-footer {
    padding: 16px;
    border-top: 1px solid var(--border);
  }

  .footer-text {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
