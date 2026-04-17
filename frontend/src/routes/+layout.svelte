<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import TopBar from '$lib/components/layout/TopBar.svelte';
  import { preferences } from '$lib/stores/preferences';

  let { children } = $props();

  let isLanding = $derived(page.url.pathname === '/');
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  function closeSidebar() {
    sidebarOpen = false;
  }

  function toggleCollapse() {
    sidebarCollapsed = !sidebarCollapsed;
    preferences.update(p => ({ ...p, sidebarCollapsed }));
  }

  // Close mobile sidebar on navigation
  $effect(() => {
    page.url.pathname;
    sidebarOpen = false;
  });

  onMount(() => {
    // Sync theme + restore sidebar collapsed state
    const unsub = preferences.subscribe(p => {
      document.documentElement.dataset.theme = p.theme;
      sidebarCollapsed = p.sidebarCollapsed;
    });

    // Keyboard shortcut: Ctrl/Cmd+B to toggle sidebar collapse
    function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'b' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        toggleCollapse();
      }
    }
    window.addEventListener('keydown', handleKeydown);

    return () => {
      unsub();
      window.removeEventListener('keydown', handleKeydown);
    };
  });
</script>

<svelte:head>
  <title>Ilm</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</svelte:head>

{#if isLanding}
  {@render children()}
{:else}
  <div class="app-layout">
    <div class="sidebar-wrapper" class:open={sidebarOpen} class:collapsed={sidebarCollapsed}>
      <Sidebar collapsed={sidebarCollapsed} onToggle={toggleCollapse} />
    </div>
    {#if sidebarOpen}
      <button class="sidebar-backdrop" onclick={closeSidebar} aria-label="Close menu"></button>
    {/if}
    <div class="main-area">
      <TopBar onToggleSidebar={toggleSidebar} />
      <main class="content">
        {@render children()}
      </main>
    </div>
  </div>
{/if}

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar-wrapper {
    flex-shrink: 0;
    width: var(--sidebar-width);
    transition: width 200ms ease;
  }

  .sidebar-wrapper.collapsed {
    width: var(--sidebar-collapsed-width);
  }

  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .content {
    flex: 1;
    overflow-y: auto;
  }

  .sidebar-backdrop {
    display: none;
  }

  @media (max-width: 768px) {
    .sidebar-wrapper {
      display: none;
      position: fixed;
      top: 0;
      left: 0;
      height: 100vh;
      z-index: 50;
      width: var(--sidebar-width) !important;
    }
    .sidebar-wrapper.open {
      display: flex;
    }
    .sidebar-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 49;
      border: none;
      padding: 0;
      cursor: default;
    }
  }
</style>
