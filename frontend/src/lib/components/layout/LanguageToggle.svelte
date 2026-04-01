<script lang="ts">
  import { language } from '$lib/stores/language';

  function toggle() {
    language.update(l => l === 'ar' ? 'en' : 'ar');
  }
</script>

<button class="lang-toggle" onclick={toggle} title="Switch language">
  {$language === 'ar' ? 'عربي' : 'EN'}
</button>

<style>
  .lang-toggle {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 20px;
    font-size: 0.8rem;
    font-weight: 600;
    background: var(--bg-surface);
    color: var(--accent);
    cursor: pointer;
    transition: all var(--transition);
  }

  .lang-toggle:hover {
    background: var(--accent-muted);
    border-color: var(--accent);
  }
</style>
