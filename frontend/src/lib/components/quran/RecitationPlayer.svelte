<script lang="ts">
  import type { ApiReciter } from '$lib/types';
  import { getReciters } from '$lib/api';
  import { preferences } from '$lib/stores/preferences';
  import { onMount } from 'svelte';

  let {
    surahNumber,
    ayahCount,
    onayahchange,
  }: {
    surahNumber: number;
    ayahCount: number;
    onayahchange: (ayah: number) => void;
  } = $props();

  let reciters: ApiReciter[] = $state([]);
  let currentAyah = $state(1);
  let playing = $state(false);
  let progress = $state(0);
  let duration = $state(0);
  let audioEl: HTMLAudioElement | undefined = $state(undefined);
  let preloadEl: HTMLAudioElement | undefined = $state(undefined);

  let selectedFolder = $state($preferences.selectedReciter ?? 'Alafasy_128kbps');

  let selectedReciter = $derived(
    reciters.find((r) => r.folder_name === selectedFolder) ?? reciters[0]
  );

  function pad(n: number, len: number): string {
    return String(n).padStart(len, '0');
  }

  function audioUrl(folder: string, surah: number, ayah: number): string {
    return `https://everyayah.com/data/${folder}/${pad(surah, 3)}${pad(ayah, 3)}.mp3`;
  }

  let currentUrl = $derived(
    selectedReciter ? audioUrl(selectedReciter.folder_name, surahNumber, currentAyah) : ''
  );

  let nextUrl = $derived(
    selectedReciter && currentAyah < ayahCount
      ? audioUrl(selectedReciter.folder_name, surahNumber, currentAyah + 1)
      : ''
  );

  onMount(() => {
    getReciters().then((r) => {
      reciters = r;
      // Restore from preferences if valid
      const saved = $preferences.selectedReciter;
      if (saved && r.find((x) => x.folder_name === saved)) {
        selectedFolder = saved;
      } else if (r.length > 0) {
        selectedFolder = r[0].folder_name;
      }
    });
  });

  function handleTimeUpdate() {
    if (audioEl) {
      progress = audioEl.currentTime;
      duration = audioEl.duration || 0;
    }
  }

  function handleEnded() {
    if (currentAyah < ayahCount) {
      currentAyah += 1;
      onayahchange(currentAyah);
      playing = true;
    } else {
      playing = false;
    }
  }

  function togglePlay() {
    if (!audioEl) return;
    if (playing) {
      audioEl.pause();
      playing = false;
    } else {
      audioEl.play();
      playing = true;
    }
  }

  function prevAyah() {
    if (currentAyah > 1) {
      currentAyah -= 1;
      onayahchange(currentAyah);
      playing = true;
    }
  }

  function nextAyahFn() {
    if (currentAyah < ayahCount) {
      currentAyah += 1;
      onayahchange(currentAyah);
      playing = true;
    }
  }

  function seekTo(e: MouseEvent) {
    if (!audioEl || !duration) return;
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const pct = (e.clientX - rect.left) / rect.width;
    audioEl.currentTime = pct * duration;
  }

  function onReciterChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    selectedFolder = val;
    preferences.update((p) => ({ ...p, selectedReciter: val }));
    if (playing && audioEl) {
      // Will auto-play due to reactive src change + $effect below
    }
  }

  export function playAyah(ayah: number) {
    currentAyah = ayah;
    onayahchange(ayah);
    playing = true;
  }

  // When currentUrl changes and we should be playing, auto-play
  $effect(() => {
    if (currentUrl && audioEl) {
      audioEl.src = currentUrl;
      if (playing) {
        audioEl.play().catch(() => { playing = false; });
      }
    }
  });

  // Preload next ayah
  $effect(() => {
    if (nextUrl && preloadEl) {
      preloadEl.src = nextUrl;
    }
  });

  function formatTime(s: number): string {
    if (!s || isNaN(s)) return '0:00';
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, '0')}`;
  }
</script>

<div class="recitation-player">
  <audio
    bind:this={audioEl}
    ontimeupdate={handleTimeUpdate}
    onended={handleEnded}
    preload="auto"
  ></audio>
  <audio bind:this={preloadEl} preload="auto" style="display:none"></audio>

  <div class="player-controls">
    <select class="reciter-select" value={selectedFolder} onchange={onReciterChange}>
      {#each reciters as r}
        <option value={r.folder_name}>
          {r.name_en}{r.style ? ` (${r.style})` : ''}
        </option>
      {/each}
    </select>

    <div class="transport">
      <button class="btn-transport" onclick={prevAyah} disabled={currentAyah <= 1} aria-label="Previous ayah">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6 8.5 6V6z"/></svg>
      </button>
      <button class="btn-play" onclick={togglePlay} aria-label={playing ? 'Pause' : 'Play'}>
        {#if playing}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6zm8-14v14h4V5z"/></svg>
        {:else}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        {/if}
      </button>
      <button class="btn-transport" onclick={nextAyahFn} disabled={currentAyah >= ayahCount} aria-label="Next ayah">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6z"/></svg>
      </button>
    </div>

    <span class="ayah-indicator">{surahNumber}:{currentAyah}</span>

    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="progress-bar" onclick={seekTo}>
      <div class="progress-fill" style="width: {duration ? (progress / duration) * 100 : 0}%"></div>
    </div>

    <span class="time-display">{formatTime(progress)} / {formatTime(duration)}</span>
  </div>
</div>

<style>
  .recitation-player {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 56px;
    background: #1a1a2e;
    color: #e0e0e0;
    z-index: 100;
    display: flex;
    align-items: center;
    padding: 0 16px;
    box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.3);
  }
  .player-controls {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }
  .reciter-select {
    background: #16213e;
    color: #e0e0e0;
    border: 1px solid #334;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 0.75rem;
    max-width: 180px;
    cursor: pointer;
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .btn-transport, .btn-play {
    background: none;
    border: none;
    color: #e0e0e0;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .btn-transport:hover, .btn-play:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .btn-transport:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .btn-play {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.1);
  }
  .ayah-indicator {
    font-size: 0.75rem;
    font-family: var(--font-mono, monospace);
    color: #8899aa;
    white-space: nowrap;
  }
  .progress-bar {
    flex: 1;
    height: 6px;
    background: #334;
    border-radius: 3px;
    cursor: pointer;
    min-width: 60px;
  }
  .progress-fill {
    height: 100%;
    background: #5c7cfa;
    border-radius: 3px;
    transition: width 0.1s linear;
  }
  .time-display {
    font-size: 0.7rem;
    font-family: var(--font-mono, monospace);
    color: #8899aa;
    white-space: nowrap;
  }
  @media (max-width: 640px) {
    .recitation-player { padding: 0 8px; }
    .reciter-select { max-width: 120px; font-size: 0.7rem; }
    .time-display { display: none; }
    .ayah-indicator { font-size: 0.7rem; }
  }
</style>
