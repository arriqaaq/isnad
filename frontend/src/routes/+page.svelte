<script lang="ts">
  import { onMount } from 'svelte';
  import { inview } from '$lib/actions/inview';
  import { getStats, getQuranStats } from '$lib/api';
  import type { StatsResponse, QuranStatsResponse } from '$lib/types';

  let hadithStats: StatsResponse | null = $state(null);
  let quranStats: QuranStatsResponse | null = $state(null);

  onMount(async () => {
    try {
      [hadithStats, quranStats] = await Promise.all([getStats(), getQuranStats()]);
    } catch (e) {
      console.error('Failed to load stats:', e);
    }
  });
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
      <p class="hero-subtitle">Search the Quran and Hadith by meaning, explore narrator chains and transmission graphs, and study with Tafsir Ibn Kathir</p>
    </div>

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
    <h2 class="section-title animate-on-scroll" use:inview>What is Ilm?</h2>
    <div class="feature-grid">
      <div class="feature-card animate-on-scroll stagger-1" use:inview>
        <div class="feature-icon">⌕</div>
        <h3>Search Quran & Hadith</h3>
        <p>Search the primary sources by meaning, not just keywords. Hybrid search fuses BM25 keyword matching with vector similarity. Search within Tafsir Ibn Kathir too.</p>
      </div>
      <div class="feature-card animate-on-scroll stagger-2" use:inview>
        <div class="feature-icon">◈</div>
        <h3>Quran with Tafsir</h3>
        <p>Browse all 114 surahs with tajweed-colored Arabic, Sahih International translation, and expandable Tafsir Ibn Kathir commentary per ayah.</p>
      </div>
      <div class="feature-card animate-on-scroll stagger-3" use:inview>
        <div class="feature-icon">◎</div>
        <h3>Narrator Graphs</h3>
        <p>Visualize isnad chains and narrator networks as interactive graphs. See who narrated from whom, explore teacher-student relationships, and trace transmission paths.</p>
      </div>
    </div>
  </section>

  <!-- How It Works -->
  <section class="section how-it-works" use:inview>
    <h2 class="section-title animate-on-scroll" use:inview>How It Works</h2>
    <div class="steps">
      <div class="step animate-on-scroll stagger-1" use:inview>
        <div class="step-num">1</div>
        <div class="step-content">
          <h3>Ingest</h3>
          <p>368K+ hadiths from Sanadset 650K with isnad chains, plus 6,236 Quranic verses with translations and tafsir.</p>
        </div>
      </div>
      <div class="step animate-on-scroll stagger-2" use:inview>
        <div class="step-num">2</div>
        <div class="step-content">
          <h3>Embed</h3>
          <p>FastEmbed generates 384-dim vectors for every hadith and ayah, enabling semantic understanding across languages.</p>
        </div>
      </div>
      <div class="step animate-on-scroll stagger-3" use:inview>
        <div class="step-num">3</div>
        <div class="step-content">
          <h3>Search</h3>
          <p>Hybrid search fuses keyword matching (BM25) with meaning-based vectors (HNSW) for the best of both worlds.</p>
        </div>
      </div>
      <div class="step animate-on-scroll stagger-4" use:inview>
        <div class="step-num">4</div>
        <div class="step-content">
          <h3>Analyze</h3>
          <p>Explore narrator networks as interactive graphs, trace isnad chains, and study transmission integrity across hadith families.</p>
        </div>
      </div>
    </div>
  </section>

  <!-- Data Sources -->
  <section class="section data-sources" use:inview>
    <h2 class="section-title animate-on-scroll" use:inview>Data Sources</h2>
    <div class="source-grid">
      <div class="source-card animate-on-scroll stagger-1" use:inview>
        <h4>Sanadset 650K</h4>
        <p>368K hadith records with pre-parsed narrator chains from 926 books</p>
      </div>
      <div class="source-card animate-on-scroll stagger-2" use:inview>
        <h4>Tanzil.net</h4>
        <p>Uthmani Arabic text + Sahih International English for all 6,236 Quranic verses</p>
      </div>
      <div class="source-card animate-on-scroll stagger-3" use:inview>
        <h4>Tafsir Ibn Kathir</h4>
        <p>Classical Quranic commentary in English for scholarly context and AI grounding</p>
      </div>
      <div class="source-card animate-on-scroll stagger-1" use:inview>
        <h4>Sunnah.com</h4>
        <p>Human-verified English translations for the 6 canonical hadith collections</p>
      </div>
      <div class="source-card animate-on-scroll stagger-2" use:inview>
        <h4>AR-Sanad</h4>
        <p>18,298 narrators with Ibn Hajar's reliability classifications from Taqrib al-Tahdhib</p>
      </div>
      <div class="source-card animate-on-scroll stagger-3" use:inview>
        <h4>Quran.com API</h4>
        <p>Tajweed-annotated Arabic text with color-coded recitation rules</p>
      </div>
    </div>
  </section>

  <!-- Tech Stack -->
  <section class="section tech-stack" use:inview>
    <h2 class="section-title animate-on-scroll" use:inview>Built With</h2>
    <div class="tech-grid animate-on-scroll" use:inview>
      <div class="tech-badge"><strong>Rust</strong><span>Axum backend</span></div>
      <div class="tech-badge"><strong>SurrealDB</strong><span>Graph + vector DB</span></div>
      <div class="tech-badge"><strong>FastEmbed</strong><span>384-dim vectors</span></div>
      <div class="tech-badge"><strong>SvelteKit</strong><span>Frontend</span></div>
      <div class="tech-badge"><strong>Ollama</strong><span>Local LLM</span></div>
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
    padding: 80px 24px 60px;
    text-align: center;
    min-height: 70vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 40px;
  }
  .hero-bg {
    position: absolute;
    inset: 0;
    background: radial-gradient(ellipse at 50% 0%, rgba(45,143,78,0.08) 0%, transparent 70%);
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
    font-size: 4rem;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -1px;
  }
  .title-ar {
    font-family: var(--font-arabic);
    font-size: 3rem;
    color: var(--text-muted);
    font-weight: 400;
  }
  .hero-subtitle {
    font-size: 1.15rem;
    color: var(--text-secondary);
    max-width: 600px;
    margin: 0 auto;
    line-height: 1.7;
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
    box-shadow: 0 4px 20px rgba(45,143,78,0.08);
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
    padding: 64px 24px;
    max-width: 900px;
    margin: 0 auto;
  }
  .section-title {
    text-align: center;
    font-size: 1.6rem;
    margin-bottom: 40px;
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
    padding: 28px 20px;
    text-align: center;
  }
  .feature-icon {
    font-size: 1.6rem;
    color: var(--accent);
    margin-bottom: 12px;
  }
  .feature-card h3 {
    font-size: 1.05rem;
    margin-bottom: 8px;
  }
  .feature-card p {
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  /* Steps */
  .steps {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .step {
    display: flex;
    gap: 20px;
    align-items: flex-start;
  }
  .step-num {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    border-radius: 50%;
    font-weight: 700;
    font-size: 1rem;
  }
  .step-content h3 {
    font-size: 1.05rem;
    margin-bottom: 4px;
  }
  .step-content p {
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.6;
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
    border-radius: var(--radius);
    padding: 20px;
  }
  .source-card h4 {
    font-size: 0.95rem;
    margin-bottom: 6px;
    color: var(--accent);
  }
  .source-card p {
    font-size: 0.8rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  /* Tech Stack */
  .tech-grid {
    display: flex;
    justify-content: center;
    flex-wrap: wrap;
    gap: 16px;
  }
  .tech-badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 16px 24px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    min-width: 120px;
  }
  .tech-badge strong {
    font-size: 0.95rem;
    color: var(--text-primary);
  }
  .tech-badge span {
    font-size: 0.75rem;
    color: var(--text-muted);
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
  }
</style>
