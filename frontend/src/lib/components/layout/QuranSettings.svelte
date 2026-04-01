<script lang="ts">
  import { preferences, stepSize, DEFAULTS, type Theme } from '$lib/stores/preferences';

  let open = $state(false);

  const themes: { key: Theme; label: string; color: string }[] = [
    { key: 'light', label: 'Light', color: '#ffffff' },
    { key: 'dark', label: 'Night', color: '#0d1117' },
    { key: 'brown', label: 'Sepia', color: '#f5ecd7' },
    { key: 'pink', label: 'Pink', color: '#fff0f5' },
  ];

  function setTheme(t: Theme) {
    preferences.update(p => ({ ...p, theme: t }));
  }
  function incArabic() {
    preferences.update(p => ({ ...p, arabicFontSize: stepSize(p.arabicFontSize, 1) }));
  }
  function decArabic() {
    preferences.update(p => ({ ...p, arabicFontSize: stepSize(p.arabicFontSize, -1) }));
  }
  function incEnglish() {
    preferences.update(p => ({ ...p, englishFontSize: stepSize(p.englishFontSize, 1) }));
  }
  function decEnglish() {
    preferences.update(p => ({ ...p, englishFontSize: stepSize(p.englishFontSize, -1) }));
  }
  function reset() {
    preferences.set({ ...DEFAULTS });
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.settings-wrapper')) {
      open = false;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="settings-wrapper">
  <button class="settings-btn" onclick={() => open = !open} title="Settings">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="3"/>
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
    </svg>
  </button>

  {#if open}
    <div class="dropdown">
      <div class="dropdown-title">Settings</div>

      <div class="control-row">
        <span class="control-label">Theme</span>
        <div class="theme-row">
          {#each themes as t}
            <button
              class="theme-dot"
              class:active={$preferences.theme === t.key}
              style="background: {t.color}; {t.key === 'dark' ? 'border-color: #30363d' : ''}"
              onclick={() => setTheme(t.key)}
              title={t.label}
            ></button>
          {/each}
        </div>
      </div>

      <div class="section-label">Font Size</div>

      <div class="control-row">
        <span class="control-label">Arabic</span>
        <div class="stepper">
          <button class="step-btn" onclick={decArabic}>-</button>
          <span class="step-value">{$preferences.arabicFontSize.toFixed(1)}</span>
          <button class="step-btn" onclick={incArabic}>+</button>
        </div>
      </div>

      <div class="control-row">
        <span class="control-label">English</span>
        <div class="stepper">
          <button class="step-btn" onclick={decEnglish}>-</button>
          <span class="step-value">{$preferences.englishFontSize.toFixed(1)}</span>
          <button class="step-btn" onclick={incEnglish}>+</button>
        </div>
      </div>

      <button class="reset-btn" onclick={reset}>Reset</button>
    </div>
  {/if}
</div>

<style>
  .settings-wrapper {
    position: relative;
  }
  .settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .settings-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .dropdown {
    position: absolute;
    top: 42px;
    right: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    min-width: 220px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
    z-index: 100;
  }
  .dropdown-title {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 12px;
  }
  .section-label {
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin: 10px 0 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border-subtle);
  }
  .control-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .control-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .theme-row {
    display: flex;
    gap: 8px;
  }
  .theme-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--border);
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
  }
  .theme-dot:hover {
    transform: scale(1.1);
  }
  .theme-dot.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .step-btn {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-hover);
    color: var(--text-primary);
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition);
  }
  .step-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .step-value {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    color: var(--text-primary);
    min-width: 28px;
    text-align: center;
  }
  .reset-btn {
    width: 100%;
    margin-top: 4px;
    padding: 4px 0;
    font-size: 0.7rem;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color var(--transition);
  }
  .reset-btn:hover {
    color: var(--accent);
  }
</style>
