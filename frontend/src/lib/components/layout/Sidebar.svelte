<script lang="ts">
  import { page } from '$app/state';

  let { collapsed = false, onToggle }: {
    collapsed?: boolean;
    onToggle?: () => void;
  } = $props();

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

<nav class="sidebar" class:collapsed>
  <div class="sidebar-header">
    <a href="/" class="logo-link" title="Ilm">
      <span class="logo">◆</span>
      {#if !collapsed}
        <span class="logo-text">Ilm</span>
      {/if}
    </a>
    <button class="collapse-toggle" onclick={onToggle} title={collapsed ? 'Expand sidebar (Ctrl+B)' : 'Collapse sidebar (Ctrl+B)'}>
      {collapsed ? '»' : '«'}
    </button>
  </div>

  <div class="nav-items">
    {#each groups as group}
      <div class="nav-group">
        {#if !collapsed}
          <span class="section-label">{group.label}</span>
        {/if}
        {#each group.items as item}
          <a
            href={item.path}
            class="nav-item"
            class:active={isActive(item.path)}
            title={collapsed ? item.label : ''}
          >
            <span class="nav-icon">{item.icon}</span>
            {#if !collapsed}
              <span class="nav-label">{item.label}</span>
            {/if}
          </a>
        {/each}
      </div>
    {/each}
  </div>

  <div class="sidebar-footer">
    {#if !collapsed}
      <span class="footer-text">Islamic Knowledge Platform</span>
    {/if}
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
    transition: width 200ms ease;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: var(--sidebar-collapsed-width);
  }

  .sidebar-header {
    padding: 12px 12px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--topbar-height);
    white-space: nowrap;
    overflow: hidden;
    flex-shrink: 0;
  }
  .collapsed .sidebar-header {
    justify-content: center;
    padding: 12px 4px;
  }

  .logo-link {
    display: flex;
    align-items: center;
    gap: 10px;
    text-decoration: none;
    color: inherit;
    transition: opacity var(--transition);
  }
  .logo-link:hover {
    opacity: 0.85;
    color: inherit;
  }

  .logo {
    color: var(--accent);
    font-size: 1.1rem;
    flex-shrink: 0;
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
    overflow-x: hidden;
  }
  .collapsed .nav-items {
    padding: 8px 4px;
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
    white-space: nowrap;
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
    white-space: nowrap;
    overflow: hidden;
  }
  .collapsed .nav-item {
    justify-content: center;
    padding: 8px 0;
    border-left: none;
    border-radius: var(--radius-sm);
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
  .collapsed .nav-item.active {
    border-left-color: transparent;
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
    padding: 10px 12px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .collapsed .sidebar-footer {
    justify-content: center;
    padding: 10px 4px;
  }

  .footer-text {
    font-family: var(--font-serif);
    font-size: 0.72rem;
    font-style: italic;
    color: var(--text-muted);
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
  }

  .collapse-toggle {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-secondary);
    font-size: 1.1rem;
    font-weight: 700;
    cursor: pointer;
    transition: all var(--transition);
    flex-shrink: 0;
  }
  .collapse-toggle:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }
  .collapsed .collapse-toggle {
    width: 100%;
    border: none;
    border-radius: 0;
  }
</style>
