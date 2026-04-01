<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import TopBar from '$lib/components/layout/TopBar.svelte';

  let { children } = $props();

  let isLanding = $derived(page.url.pathname === '/');
</script>

<svelte:head>
  <title>Ilm</title>
</svelte:head>

{#if isLanding}
  {@render children()}
{:else}
  <div class="app-layout">
    <Sidebar />
    <div class="main-area">
      <TopBar />
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

  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content {
    flex: 1;
    overflow-y: auto;
  }
</style>
