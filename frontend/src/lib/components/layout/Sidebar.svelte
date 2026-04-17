<script lang="ts">
  import { page } from '$app/state';

  interface NavItem {
    path: string;
    label: string;
    icon: string;
  }

  interface NavGroup {
    label: string;
    items: NavItem[];
  }

  const groups: NavGroup[] = [
    {
      label: 'Browse',
      items: [
        { path: '/explore', label: 'Explore', icon: '◈' },
        { path: '/ask', label: 'Ask', icon: '◇' },
      ],
    },
    {
      label: 'Quran',
      items: [
        { path: '/quran', label: 'Quran', icon: '❐' },
        { path: '/quran/search', label: 'Search', icon: '⌕' },
      ],
    },
    {
      label: 'Notes',
      items: [
        { path: '/notes', label: 'Notes', icon: '✎' },
      ],
    },
    {
      label: 'Hadith',
      items: [
        { path: '/hadiths', label: 'Hadiths', icon: '☰' },
        { path: '/narrators', label: 'Narrators', icon: '◎' },
        { path: '/books', label: 'Books', icon: '▤' },
        { path: '/families', label: 'Families', icon: '⬡' },
        { path: '/search', label: 'Search', icon: '⌕' },
        { path: '/analysis', label: 'Analysis', icon: '△' },
      ],
    },
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
    {#each groups as group}
      <div class="nav-group">
        <span class="section-label">{group.label}</span>
        {#each group.items as item}
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
    padding: 20px 16px;
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
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 0;
    overflow-y: auto;
  }

  .nav-group {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .section-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    padding: 16px 12px 5px;
    font-weight: 600;
    font-family: var(--font-sans);
    user-select: none;
  }

  .nav-group:first-child .section-label {
    padding-top: 8px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px;
    border-radius: var(--radius-sm);
    border-left: 2px solid transparent;
    color: var(--text-secondary);
    transition: all var(--transition);
    font-family: var(--font-sans);
    font-size: 0.88rem;
    text-decoration: none;
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-left-color: var(--accent);
    font-weight: 500;
  }

  .nav-icon {
    width: 18px;
    text-align: center;
    font-size: 0.82rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .nav-item.active .nav-icon {
    color: var(--accent);
  }

  .sidebar-footer {
    padding: 14px 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .footer-text {
    font-family: var(--font-serif);
    font-size: 0.72rem;
    font-style: italic;
    color: var(--text-muted);
  }
</style>
