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

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  function closeSidebar() {
    sidebarOpen = false;
  }

  // Close sidebar on navigation
  $effect(() => {
    page.url.pathname;
    sidebarOpen = false;
  });

  onMount(() => {
    const unsub = preferences.subscribe(p => {
      document.documentElement.dataset.theme = p.theme;
    });
    return unsub;
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
    <div class="sidebar-wrapper" class:open={sidebarOpen}>
      <Sidebar />
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
