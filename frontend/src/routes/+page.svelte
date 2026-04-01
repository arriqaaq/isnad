<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { inview } from '$lib/actions/inview';
  import { getStats, getQuranStats } from '$lib/api';
  import type { StatsResponse, QuranStatsResponse } from '$lib/types';

  let hadithStats: StatsResponse | null = $state(null);
  let quranStats: QuranStatsResponse | null = $state(null);
  let searchQuery = $state('');

  onMount(async () => {
    try {
      [hadithStats, quranStats] = await Promise.all([getStats(), getQuranStats()]);
    } catch (e) {
      console.error('Failed to load stats:', e);
    }
  });

  function handleSearch(e: Event) {
    e.preventDefault();
    if (searchQuery.trim()) {
      goto(`/explore?q=${encodeURIComponent(searchQuery.trim())}&type=semantic`);
    }
  }
</script>

<div class="landing">
  <!-- Hero -->
  <section class="hero">
    <div class="hero-bg"></div>
    <div class="hero-content">
      <h1 class="hero-title">
        <span class="title-en">Ilm</span>
        <span class="title-ar" dir="rtl">عِلْم</span>
      </h1>
      <p class="hero-subtitle">Search the <strong>Quran</strong> and <strong>Sunnah</strong> by meaning</p>
    </div>

    <!-- Google-style Search Bar -->
    <form class="hero-search" onsubmit={handleSearch}>
      <div class="hero-search-bar">
        <span class="hero-search-icon">&#x2315;</span>
        <input
          type="text"
          placeholder="Search Quran & Sunnah..."
          bind:value={searchQuery}
          class="hero-search-input"
        />
      </div>
    </form>

    <!-- Quick Access Cards -->
    <div class="quick-access">
      <div class="access-card quran-card">
        <div class="card-icon">◈</div>
        <h2>Read Quran</h2>
        {#if quranStats}
          <div class="card-stats">
            <span class="stat">{quranStats.surah_count} Surahs</span>
            <span class="stat-dot">·</span>
            <span class="stat">{quranStats.ayah_count.toLocaleString()} Ayahs</span>
          </div>
        {/if}
        <div class="card-links">
          <a href="/quran">Browse Surahs</a>
          <a href="/quran/search">Search</a>
        </div>
      </div>

      <div class="access-card hadith-card">
        <div class="card-icon">☰</div>
        <h2>Explore Hadith</h2>
        {#if hadithStats}
          <div class="card-stats">
            <span class="stat">{hadithStats.hadith_count.toLocaleString()} Hadiths</span>
            <span class="stat-dot">·</span>
            <span class="stat">{hadithStats.narrator_count.toLocaleString()} Narrators</span>
          </div>
        {/if}
        <div class="card-links">
          <a href="/hadiths">Browse Hadiths</a>
          <a href="/search">Search</a>
          <a href="/narrators">Narrators</a>
        </div>
      </div>
    </div>
  </section>

  <!-- Features -->
  <section class="section features" use:inview>
    <div class="section-label animate-on-scroll" use:inview>Features</div>
    <h2 class="section-title animate-on-scroll" use:inview>What is Ilm?</h2>
    <div class="feature-grid">
      <div class="feature-card animate-on-scroll stagger-1" use:inview>
        <div class="feature-highlight">Semantic</div>
        <h3>Search by Meaning</h3>
        <p>Find verses and hadiths by <strong>what they mean</strong>, not just keywords. Hybrid search fuses <strong>BM25</strong> with <strong>384-dim vectors</strong>.</p>
      </div>
      <div class="feature-card animate-on-scroll stagger-2" use:inview>
        <div class="feature-highlight">114 Surahs</div>
        <h3>Quran with Tafsir</h3>
        <p><strong>Tajweed-colored</strong> Arabic, Sahih International translation, and expandable <strong>Tafsir Ibn Kathir</strong> commentary per ayah.</p>
      </div>
      <div class="feature-card animate-on-scroll stagger-3" use:inview>
        <div class="feature-highlight">Interactive</div>
        <h3>Narrator Graphs</h3>
        <p>Visualize <strong>isnad chains</strong> as graphs. Trace <strong>transmission paths</strong> and explore <strong>teacher-student</strong> networks.</p>
      </div>
    </div>
  </section>

  <!-- Data Sources -->
  <section class="section data-sources" use:inview>
    <div class="section-label animate-on-scroll" use:inview>Provenance</div>
    <h2 class="section-title animate-on-scroll" use:inview>Data Sources</h2>
    <div class="source-grid">
      <div class="source-card animate-on-scroll stagger-1" use:inview>
        <div class="source-stat">368K</div>
        <h4>Sanadset 650K</h4>
        <p>Hadith records with pre-parsed narrator chains from 926 books</p>
      </div>
      <div class="source-card animate-on-scroll stagger-2" use:inview>
        <div class="source-stat">6,236</div>
        <h4>Tanzil.net</h4>
        <p>Quranic verses in Uthmani Arabic + Sahih International English</p>
      </div>
      <div class="source-card animate-on-scroll stagger-3" use:inview>
        <div class="source-stat">114</div>
        <h4>Tafsir Ibn Kathir</h4>
        <p>Surahs with classical commentary for scholarly context</p>
      </div>
      <div class="source-card animate-on-scroll stagger-1" use:inview>
        <div class="source-stat">33K</div>
        <h4>Sunnah.com</h4>
        <p>Human-verified English translations across 6 canonical collections</p>
      </div>
      <div class="source-card animate-on-scroll stagger-2" use:inview>
        <div class="source-stat">18K</div>
        <h4>AR-Sanad</h4>
        <p>Narrators with Ibn Hajar's reliability classifications</p>
      </div>
      <div class="source-card animate-on-scroll stagger-3" use:inview>
        <div class="source-stat">&#x2726;</div>
        <h4>Quran.com API</h4>
        <p>Tajweed-annotated Arabic with color-coded recitation rules</p>
      </div>
    </div>
  </section>

  <!-- Architecture & Training — dark section -->
  <section class="hood-section" use:inview>
    <div class="hood-glow"></div>
    <div class="hood-inner">
      <div class="section-label animate-on-scroll" use:inview>Technical</div>
      <h2 class="hood-title animate-on-scroll" use:inview>Under the Hood</h2>

      <div class="hood-columns">
        <!-- Architecture Stack -->
        <div class="hood-col">
          <h3 class="hood-subtitle animate-on-scroll" use:inview>Architecture</h3>
          <div class="arch-stack">
            <div class="glass-card animate-on-scroll stagger-1" use:inview>
              <div class="layer-label">Frontend</div>
              <strong>SvelteKit</strong>
              <p>Quran · Hadith · Search · Narrators · Graphs</p>
            </div>
            <div class="arch-line animate-on-scroll stagger-1" use:inview>
              <div class="line-pulse"></div>
              <span class="line-label">JSON API</span>
            </div>
            <div class="glass-card animate-on-scroll stagger-2" use:inview>
              <div class="layer-label">Backend</div>
              <strong>Rust / Axum</strong>
              <div class="sub-cards">
                <div class="sub-chip">Search</div>
                <div class="sub-chip">Ingest</div>
                <div class="sub-chip">RAG</div>
              </div>
            </div>
            <div class="arch-line animate-on-scroll stagger-2" use:inview>
              <div class="line-pulse"></div>
            </div>
            <div class="glass-card animate-on-scroll stagger-3" use:inview>
              <div class="layer-label">Database</div>
              <strong>SurrealDB</strong>
              <p>Graph · HNSW vectors · BM25</p>
            </div>
            <div class="arch-line animate-on-scroll stagger-3" use:inview>
              <div class="line-pulse"></div>
            </div>
            <div class="glass-card animate-on-scroll stagger-4" use:inview>
              <div class="layer-label">Embeddings</div>
              <strong>FastEmbed</strong>
              <p>multilingual-e5-small · 384-dim</p>
            </div>
          </div>
        </div>

        <!-- Training Pipeline -->
        <div class="hood-col">
          <h3 class="hood-subtitle animate-on-scroll" use:inview>Training Pipeline</h3>
          <div class="pipeline-grid">
            {#each [
              { n: '1', title: 'Raw Data', desc: 'Sanadset · Tanzil · Sunnah.com' },
              { n: '2', title: 'Parse & Enrich', desc: 'Quran + Tafsir + Narrator bios' },
              { n: '3', title: 'Format as QA', desc: 'Question-answer pairs with isnad context' },
              { n: '4', title: 'Fine-tune LoRA', desc: 'Qwen / Llama with domain knowledge' },
              { n: '5', title: 'Deploy', desc: 'Ollama local serve' },
            ] as step, i}
              <div class="pipe-card animate-on-scroll stagger-{Math.min(i + 1, 4)}" use:inview>
                <div class="pipe-num">{step.n}</div>
                <div class="pipe-text">
                  <strong>{step.title}</strong>
                  <span>{step.desc}</span>
                </div>
              </div>
              {#if i < 4}
                <div class="pipe-line animate-on-scroll stagger-{Math.min(i + 1, 4)}" use:inview>
                  <div class="line-pulse"></div>
                </div>
              {/if}
            {/each}
          </div>
        </div>
      </div>
    </div>
  </section>

  <!-- Footer -->
  <footer class="landing-footer">
    <p>Ilm — Islamic Knowledge Platform</p>
  </footer>
</div>

<style>
  .landing {
    min-height: 100vh;
    background: var(--bg-primary);
    overflow-x: hidden;
  }

  /* Hero */
  .hero {
    position: relative;
    padding: 60px 24px 40px;
    text-align: center;
    min-height: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 40px;
  }
  .hero-bg {
    position: absolute;
    inset: 0;
    background: radial-gradient(ellipse at 50% 0%, rgba(214,51,132,0.08) 0%, transparent 70%);
    animation: pulse-glow 4s ease-in-out infinite alternate;
    pointer-events: none;
  }
  @keyframes pulse-glow {
    from { opacity: 0.4; }
    to { opacity: 1; }
  }
  .hero-content {
    position: relative;
    z-index: 1;
    animation: fade-in-up 0.8s ease forwards;
  }
  @keyframes fade-in-up {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .hero-title {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 16px;
    margin-bottom: 16px;
  }
  .title-en {
    font-size: 5rem;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -2px;
  }
  .title-ar {
    font-family: var(--font-arabic);
    font-size: 3.5rem;
    color: var(--text-muted);
    font-weight: 400;
  }
  .hero-subtitle {
    font-size: 1.15rem;
    color: var(--text-secondary);
    max-width: 600px;
    margin: 0 auto;
    line-height: 1.7;
    font-weight: 400;
  }
  .hero-subtitle :global(strong) {
    color: var(--accent);
    font-weight: 600;
  }

  /* Hero Search Bar */
  .hero-search {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 580px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    animation: fade-in-up 0.8s 0.2s ease both;
  }
  .hero-search-bar {
    width: 100%;
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 2px solid var(--border);
    border-radius: 28px;
    padding: 0 20px;
    box-shadow: 0 2px 12px rgba(0,0,0,0.06);
    transition: all 0.2s ease;
  }
  .hero-search-bar:focus-within {
    border-color: var(--accent);
    box-shadow: 0 4px 20px rgba(214,51,132,0.12);
  }
  .hero-search-icon {
    color: var(--text-muted);
    font-size: 1.2rem;
    margin-right: 12px;
    flex-shrink: 0;
  }
  .hero-search-input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 16px 0;
    font-size: 1.05rem;
    color: var(--text-primary);
    outline: none;
  }
  .hero-search-input::placeholder {
    color: var(--text-muted);
  }
  /* Quick Access Cards */
  .quick-access {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 20px;
    max-width: 700px;
    width: 100%;
    animation: fade-in-up 0.8s 0.3s ease both;
  }
  .access-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 28px 24px;
    text-align: center;
    color: var(--text-primary);
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .access-card:hover {
    border-color: var(--accent);
    box-shadow: 0 4px 20px rgba(214,51,132,0.08);
    transform: translateY(-2px);
    color: var(--text-primary);
  }
  .card-icon {
    font-size: 1.8rem;
    color: var(--accent);
    margin-bottom: 4px;
  }
  .access-card h2 {
    font-size: 1.2rem;
    font-weight: 700;
  }
  .card-stats {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 0.8rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .stat-dot { color: var(--border); }
  .card-links {
    display: flex;
    gap: 12px;
    margin-top: 8px;
    font-size: 0.8rem;
  }
  .card-links a {
    padding: 4px 12px;
    background: var(--accent-muted);
    border-radius: 16px;
    color: var(--accent);
    font-weight: 500;
    transition: all var(--transition);
  }
  .card-links a:hover {
    background: var(--accent);
    color: white;
  }

  /* Sections */
  .section {
    padding: 48px 24px;
    max-width: 900px;
    margin: 0 auto;
  }
  .section-label {
    text-align: center;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 2px;
    color: var(--accent);
    font-weight: 700;
    margin-bottom: 8px;
  }
  .section-title {
    text-align: center;
    font-size: 1.6rem;
    margin-bottom: 36px;
    color: var(--text-primary);
  }

  /* Feature cards */
  .feature-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 20px;
  }
  .feature-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 32px 24px;
    text-align: center;
    transition: all 0.3s ease;
  }
  .feature-card:hover {
    border-color: var(--accent);
    box-shadow: 0 6px 28px rgba(214,51,132,0.08);
    transform: translateY(-3px);
  }
  .feature-highlight {
    font-size: 1.6rem;
    font-weight: 700;
    color: var(--accent);
    margin-bottom: 12px;
    line-height: 1;
  }
  .feature-card h3 {
    font-size: 1.05rem;
    margin-bottom: 10px;
  }
  .feature-card p {
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.6;
  }
  .feature-card p :global(strong) {
    color: var(--text-primary);
    font-weight: 600;
  }

  /* Under the Hood — light pink section */
  .hood-section {
    position: relative;
    background: linear-gradient(180deg,
      var(--bg-primary) 0%,
      rgba(214,51,132,0.06) 15%,
      rgba(214,51,132,0.10) 50%,
      rgba(214,51,132,0.06) 85%,
      var(--bg-primary) 100%
    );
    padding: 80px 24px;
    overflow: hidden;
  }
  .hood-glow {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 700px;
    height: 500px;
    background: radial-gradient(ellipse, rgba(214,51,132,0.08) 0%, transparent 70%);
    filter: blur(80px);
    animation: glow-breathe 4s ease-in-out infinite alternate;
    pointer-events: none;
  }
  @keyframes glow-breathe {
    from { opacity: 0.6; transform: translate(-50%, -50%) scale(0.95); }
    to { opacity: 1; transform: translate(-50%, -50%) scale(1.05); }
  }
  .hood-inner {
    position: relative;
    z-index: 1;
    max-width: 900px;
    margin: 0 auto;
  }
  .hood-title {
    text-align: center;
    font-size: 1.6rem;
    color: var(--text-primary);
    margin-bottom: 48px;
  }
  .hood-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 72px;
    align-items: stretch;
  }
  .hood-col {
    display: flex;
    flex-direction: column;
  }
  .hood-subtitle {
    text-align: center;
    font-size: 1.05rem;
    margin-bottom: 8px;
    color: var(--accent);
    font-weight: 600;
  }
  .hood-desc {
    text-align: center;
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-bottom: 20px;
  }

  /* Architecture Stack */
  .arch-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
    flex: 1;
    justify-content: space-between;
  }
  .glass-card {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 18px 20px;
    text-align: center;
    transition: all 0.3s ease;
    box-shadow: 0 2px 8px rgba(214,51,132,0.04);
  }
  .glass-card:hover {
    border-color: var(--accent);
    box-shadow: 0 4px 24px rgba(214,51,132,0.1);
    transform: translateY(-2px);
  }
  .layer-label {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--accent);
    font-weight: 700;
    margin-bottom: 4px;
  }
  .glass-card strong {
    display: block;
    font-size: 1rem;
    color: var(--text-primary);
    margin-bottom: 4px;
  }
  .glass-card p {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .sub-cards {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .sub-chip {
    flex: 1;
    padding: 6px 0;
    background: var(--accent-muted);
    border: 1px solid rgba(214,51,132,0.15);
    border-radius: 6px;
    font-size: 0.7rem;
    color: var(--text-secondary);
    text-align: center;
  }

  /* Animated connecting lines */
  .arch-line, .pipe-line {
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 0;
    transition: height 0.5s ease;
    position: relative;
    overflow: visible;
  }
  .arch-line.in-view, .pipe-line.in-view {
    height: 28px;
  }
  .line-pulse {
    width: 2px;
    height: 100%;
    background: linear-gradient(180deg, var(--accent), rgba(214,51,132,0.2));
    position: relative;
  }
  .line-pulse::after {
    content: '';
    position: absolute;
    top: 0;
    left: -2px;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    box-shadow: 0 0 8px rgba(214,51,132,0.4);
    animation: dot-travel 2s ease-in-out infinite;
  }
  @keyframes dot-travel {
    0% { top: 0; opacity: 0; }
    20% { opacity: 1; }
    80% { opacity: 1; }
    100% { top: calc(100% - 6px); opacity: 0; }
  }
  .line-label {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.6rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  /* Training Pipeline */
  .pipeline-grid {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
    flex: 1;
    justify-content: space-between;
  }
  .pipe-card {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    transition: all 0.3s ease;
    box-shadow: 0 2px 8px rgba(214,51,132,0.04);
  }
  .pipe-card:hover {
    border-color: var(--accent);
    box-shadow: 0 4px 20px rgba(214,51,132,0.1);
    transform: translateY(-1px);
  }
  .pipe-num {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    border-radius: 50%;
    font-weight: 700;
    font-size: 0.75rem;
    box-shadow: 0 0 10px rgba(214,51,132,0.2);
  }
  .pipe-text strong {
    display: block;
    font-size: 0.85rem;
    color: var(--text-primary);
  }
  .pipe-text span {
    font-size: 0.7rem;
    color: var(--text-muted);
    line-height: 1.4;
  }

  /* Data Sources */
  .source-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }
  .source-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 24px 20px;
    text-align: center;
    transition: all 0.3s ease;
  }
  .source-card:hover {
    border-color: var(--accent);
    box-shadow: 0 4px 20px rgba(214,51,132,0.08);
    transform: translateY(-2px);
  }
  .source-stat {
    font-size: 1.8rem;
    font-weight: 700;
    color: var(--accent);
    font-family: var(--font-mono);
    margin-bottom: 8px;
    line-height: 1;
  }
  .source-card h4 {
    font-size: 0.9rem;
    margin-bottom: 6px;
    color: var(--text-primary);
  }
  .source-card p {
    font-size: 0.78rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  /* Footer */
  .landing-footer {
    text-align: center;
    padding: 40px 24px;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  /* Responsive */
  @media (max-width: 700px) {
    .quick-access { grid-template-columns: 1fr; }
    .feature-grid { grid-template-columns: 1fr; }
    .source-grid { grid-template-columns: 1fr; }
    .title-en { font-size: 2.5rem; }
    .title-ar { font-size: 2rem; }
    .sub-cards { flex-direction: column; }
    .hood-columns { grid-template-columns: 1fr; }
  }
</style>
