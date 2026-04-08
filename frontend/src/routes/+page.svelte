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

  <!-- ════════ HERO ════════ -->
  <section class="hero">
    <div class="hero-glow"></div>
    <div class="hero-glow-2"></div>

    <p class="bismillah" dir="rtl">بِسۡمِ ٱللَّهِ ٱلرَّحۡمَـٰنِ ٱلرَّحِيمِ</p>

    <div class="hero-title-group">
      <h1 class="hero-title">
        <span class="title-en">Ilm</span>
        <span class="title-ar" dir="rtl">عِلْم</span>
      </h1>
      <p class="hero-tagline">Search the Quran & Sunnah. <em>Deeply.</em></p>
    </div>

    <p class="hero-desc">
      A complete semantic search platform for Islamic scholarship — explore the Quran with tafsir,
      hundreds of thousands of hadiths with narrator chains, and interactive isnad graphs.
      Powered by meaning, not just keywords.
    </p>

    <div class="hero-pills">
      <span class="pill">Free & Open Source</span>
      <span class="pill">Semantic Search</span>
      <span class="pill">Bilingual</span>
      <span class="pill">Open Data</span>
      <span class="pill">AI-Powered</span>
    </div>

    <form class="hero-search" onsubmit={handleSearch}>
      <div class="hero-search-bar">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
        </svg>
        <input
          type="text"
          placeholder="What does the Quran say about patience?"
          bind:value={searchQuery}
          class="hero-search-input"
        />
        <button type="submit" class="search-btn">Search</button>
      </div>
    </form>

    <div class="hero-ctas">
      <a href="/explore" class="cta cta-filled">Start Exploring</a>
      <a href="/quran" class="cta cta-outline">Browse Quran</a>
      <a href="/hadiths" class="cta cta-outline">Browse Hadith</a>
    </div>

    {#if hadithStats || quranStats}
      <div class="hero-stats">
        {#if quranStats}
          <div class="hero-stat">
            <span class="hero-stat-num">{quranStats.surah_count}</span>
            <span class="hero-stat-label">Surahs</span>
          </div>
          <span class="hero-stat-dot"></span>
          <div class="hero-stat">
            <span class="hero-stat-num">{quranStats.ayah_count.toLocaleString()}</span>
            <span class="hero-stat-label">Ayahs</span>
          </div>
        {/if}
        {#if hadithStats}
          <span class="hero-stat-dot"></span>
          <div class="hero-stat">
            <span class="hero-stat-num">{hadithStats.hadith_count.toLocaleString()}</span>
            <span class="hero-stat-label">Hadiths</span>
          </div>
          <span class="hero-stat-dot"></span>
          <div class="hero-stat">
            <span class="hero-stat-num">{hadithStats.narrator_count.toLocaleString()}</span>
            <span class="hero-stat-label">Narrators</span>
          </div>
        {/if}
      </div>
    {/if}
  </section>

  <!-- ════════ FEATURES ════════ -->
  <section class="features-section">
    <div class="section-header animate-on-scroll" use:inview>
      <span class="section-label">Features</span>
      <h2>Everything you need to study</h2>
    </div>

    <!-- Intelligent Search -->
    <div class="feat animate-on-scroll" use:inview>
      <div class="feat-visual">
        <div class="app-frame">
          <div class="frame-dots"><span></span><span></span><span></span></div>
          <img src="/img/feature-search.png" alt="Unified search across Quran and Hadith" />
        </div>
      </div>
      <div class="feat-text">
        <h3>Intelligent Search</h3>
        <p>Find verses and hadiths by what they mean, not just keywords. Hybrid search fuses BM25 full-text with 1024-dimension semantic vectors across the entire corpus.</p>
        <a href="/explore" class="feat-link">Try Search &rarr;</a>
      </div>
    </div>

    <!-- Quran Reader -->
    <div class="feat feat-reverse animate-on-scroll" use:inview>
      <div class="feat-visual">
        <div class="app-frame">
          <div class="frame-dots"><span></span><span></span><span></span></div>
          <img src="/img/feature-quran.png" alt="Quran reader with tajweed and tafsir" />
        </div>
      </div>
      <div class="feat-text">
        <h3>Quran Reader</h3>
        <p>114 Surahs with Tajweed-colored Arabic, Sahih International translation, and expandable Tafsir Ibn Kathir commentary — per ayah.</p>
        <a href="/quran" class="feat-link">Read Quran &rarr;</a>
      </div>
    </div>

    <!-- Hadith Explorer -->
    <div class="feat animate-on-scroll" use:inview>
      <div class="feat-visual">
        <div class="app-frame">
          <div class="frame-dots"><span></span><span></span><span></span></div>
          <img src="/img/feature-hadith.png" alt="Hadith browsing with narrator chains" />
        </div>
      </div>
      <div class="feat-text">
        <h3>Hadith Explorer</h3>
        <p>Browse hundreds of thousands of hadiths from 926 books across 6 canonical collections, each with full narrator chains and source attribution.</p>
        <a href="/hadiths" class="feat-link">Explore Hadiths &rarr;</a>
      </div>
    </div>

    <!-- Narrator Networks -->
    <div class="feat feat-reverse animate-on-scroll" use:inview>
      <div class="feat-visual">
        <div class="app-frame">
          <div class="frame-dots"><span></span><span></span><span></span></div>
          <img src="/img/feature-narrators.png" alt="Interactive narrator graph visualization" />
        </div>
      </div>
      <div class="feat-text">
        <h3>Narrator Networks</h3>
        <p>Interactive graph visualization of 18K+ narrators. Trace isnad chains, explore teacher-student relationships, and check Ibn Hajar reliability grades.</p>
        <a href="/narrators" class="feat-link">View Narrators &rarr;</a>
      </div>
    </div>

    <!-- Early Manuscripts -->
    <div class="feat animate-on-scroll" use:inview>
      <div class="feat-visual">
        <div class="app-frame">
          <div class="frame-dots"><span></span><span></span><span></span></div>
          <img src="/img/manuscript-sample.jpg" alt="Early Quranic manuscript — Berlin, Wetzstein II 1913" style="height: 340px; object-fit: cover;" />
        </div>
      </div>
      <div class="feat-text">
        <h3>Early Manuscripts</h3>
        <p>View high-resolution images of early Quranic manuscripts per ayah from the Corpus Coranicum archive — Berlin-Brandenburg Academy of Sciences. Click to zoom.</p>
        <a href="/quran/2?ayah=238" class="feat-link">View Example &rarr;</a>
      </div>
    </div>

    <!-- Personal Study Notes -->
    <div class="feat feat-reverse animate-on-scroll" use:inview>
      <div class="feat-visual">
        <div class="app-frame">
          <div class="frame-dots"><span></span><span></span><span></span></div>
          <img src="/img/feature-notes.svg" alt="Personal Study Notes — annotate ayahs and hadiths" style="height: 340px; object-fit: contain; background: var(--bg-surface); padding: 8px;" />
        </div>
      </div>
      <div class="feat-text">
        <h3>Personal Study Notes</h3>
        <p>Capture your thoughts while studying. Annotate any ayah or hadith, collect evidence by topic, and embed Quran verses and hadiths inline with @mentions. Your personal knowledge system for Islamic study.</p>
        <div class="feat-pills">
          <span class="feat-pill">@Mentions</span>
          <span class="feat-pill">Topic Collections</span>
          <span class="feat-pill">Tags</span>
          <span class="feat-pill">Color Highlights</span>
          <span class="feat-pill">Rich Embeds</span>
        </div>
        <a href="/notes" class="feat-link">Start Taking Notes &rarr;</a>
      </div>
    </div>
  </section>

  <!-- ════════ DATA SOURCES ════════ -->
  <section class="sources-section" use:inview>
    <div class="sources-inner">
      <div class="section-header animate-on-scroll" use:inview>
        <span class="section-label">Provenance</span>
        <h2>Data Sources</h2>
        <p>Built on open scholarly datasets. Every hadith, ayah, and narrator is traceable to its source.</p>
      </div>
      <div class="source-grid">
        <div class="source-card animate-on-scroll stagger-1" use:inview>
          <div class="source-num">34K</div>
          <h4>SemanticHadith KG</h4>
          <p>Hadiths with narrator chains and knowledge graph from 6 canonical collections</p>
        </div>
        <div class="source-card animate-on-scroll stagger-2" use:inview>
          <div class="source-num">6,236</div>
          <h4>QUL (Tarteel)</h4>
          <p>QPC Hafs Arabic + Sahih International English from Quranic Universal Library</p>
        </div>
        <div class="source-card animate-on-scroll stagger-3" use:inview>
          <div class="source-num">6,236</div>
          <h4>Tafsir Ibn Kathir</h4>
          <p>Classical exegesis per ayah via QUL</p>
        </div>
        <div class="source-card animate-on-scroll stagger-1" use:inview>
          <div class="source-num">33K</div>
          <h4>Sunnah.com</h4>
          <p>Human-verified English translations across 6 canonical collections</p>
        </div>
        <div class="source-card animate-on-scroll stagger-2" use:inview>
          <div class="source-num">18K</div>
          <h4>AR-Sanad</h4>
          <p>Narrators with Ibn Hajar's reliability classifications</p>
        </div>
        <div class="source-card animate-on-scroll stagger-3" use:inview>
          <div class="source-num">&#x2726;</div>
          <h4>Corpus Coranicum</h4>
          <p>Early manuscript images per ayah from Berlin-Brandenburg Academy</p>
        </div>
      </div>
    </div>
  </section>

  <!-- ════════ ARCHITECTURE ════════ -->
  <section class="hood-section" use:inview>
    <div class="hood-glow"></div>
    <div class="hood-inner">
      <div class="section-header animate-on-scroll" use:inview>
        <span class="section-label">Open Architecture</span>
        <h2>Under the Hood</h2>
        <p>A modern stack built for Islamic scholarship.</p>
      </div>

      <div class="hood-columns">
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
              <p>bge-m3 · 1024-dim</p>
            </div>
          </div>
        </div>

        <div class="hood-col">
          <h3 class="hood-subtitle animate-on-scroll" use:inview>Training Pipeline</h3>
          <div class="pipeline-grid">
            {#each [
              { n: '1', title: 'Raw Data', desc: 'SemanticHadith 34K · QUL · Sunnah.com' },
              { n: '2', title: 'Parse & Enrich', desc: 'Join translations + narrator bios + tafsir' },
              { n: '3', title: 'Generate QA', desc: 'ChatML pairs matching RAG prompt pattern' },
              { n: '4', title: 'LoRA Fine-tune', desc: 'MLX on Phi-4-mini / Command-R' },
              { n: '5', title: 'GGUF → Ollama', desc: 'Quantize Q4_K_M · ollama create · serve' },
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

  <!-- ════════ FOOTER ════════ -->
  <footer class="landing-footer">
    <div class="footer-inner">
      <div class="footer-brand">
        <span class="footer-logo">Ilm</span>
        <span class="footer-tagline">Islamic Knowledge Platform</span>
        <span class="footer-author">Built by <a href="https://github.com/arriqaaq" target="_blank" rel="noopener noreferrer">@arriqaaq</a></span>
      </div>
      <div class="footer-links">
        <div class="footer-col">
          <h4>Explore</h4>
          <a href="/explore">Unified Search</a>
          <a href="/ask">Ask AI</a>
        </div>
        <div class="footer-col">
          <h4>Quran</h4>
          <a href="/quran">Browse Surahs</a>
          <a href="/quran/search">Search Quran</a>
        </div>
        <div class="footer-col">
          <h4>Hadith</h4>
          <a href="/hadiths">Browse Hadiths</a>
          <a href="/narrators">Narrators</a>
          <a href="/books">Books</a>
        </div>
      </div>
    </div>
    <div class="footer-bottom">
      <p>Built with care for the Muslim community</p>
    </div>
  </footer>
</div>

<style>
  /* ══════════════════════════════════
     LANDING PAGE
     ══════════════════════════════════ */
  .landing {
    min-height: 100vh;
    background: var(--bg-primary);
    overflow-x: hidden;
  }

  /* ── Shared section header ── */
  .section-header {
    text-align: center;
    margin-bottom: 48px;
  }
  .section-label {
    display: inline-block;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 2.5px;
    color: var(--accent);
    font-weight: 700;
    margin-bottom: 12px;
    padding: 4px 16px;
    background: var(--accent-muted);
    border-radius: 20px;
  }
  .section-header h2 {
    font-size: 2rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 12px;
    letter-spacing: -0.5px;
  }
  .section-header p {
    font-size: 0.95rem;
    color: var(--text-secondary);
    max-width: 520px;
    margin: 0 auto;
    line-height: 1.6;
  }

  /* ══════════════════════════════════
     HERO
     ══════════════════════════════════ */
  .hero {
    position: relative;
    padding: 100px 24px 80px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 24px;
    overflow: hidden;
  }

  /* Shifting gradient background — pink shades, fades at edges */
  .hero-glow {
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg,
      #fff5f8, #fce4ec, #f9e0f0, #fdf2f8, #f5c6d8, #fff5f8
    ) 0 0 / 300% 300%;
    animation: hero-gradient 12s ease infinite;
    pointer-events: none;
    mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 70%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 70%, transparent 100%);
  }
  .hero-glow-2 { display: none; }

  :global([data-theme="dark"]) .hero-glow {
    background: linear-gradient(135deg,
      #1a1020, #2a1525, #1e1028, #251a2e, #3b2040, #1a1020
    ) 0 0 / 300% 300%;
    animation: hero-gradient 12s ease infinite;
  }
  :global([data-theme="brown"]) .hero-glow {
    background: linear-gradient(135deg,
      #f5ecd7, #f0dfc0, #efe4cb, #f5e8d0, #e8d5b0, #f5ecd7
    ) 0 0 / 300% 300%;
    animation: hero-gradient 12s ease infinite;
  }

  @keyframes hero-gradient {
    0% { background-position: 0% 0%; }
    50% { background-position: 100% 100%; }
    100% { background-position: 0% 0%; }
  }

  /* Bismillah */
  .bismillah {
    position: relative;
    z-index: 1;
    font-family: 'Scheherazade New', var(--font-arabic);
    font-size: 2.2rem;
    color: var(--accent);
    font-weight: 400;
    line-height: 2.4;
    letter-spacing: 2px;
    opacity: 0;
    animation: fade-in 1s 0.1s ease forwards;
  }

  @keyframes fade-in {
    to { opacity: 1; }
  }
  @keyframes fade-in-up {
    from { opacity: 0; transform: translateY(24px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Title group */
  .hero-title-group {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    opacity: 0;
    animation: fade-in-up 0.8s 0.3s ease forwards;
  }
  .hero-title {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 16px;
    margin: 0;
  }
  .title-en {
    font-size: 5rem;
    font-weight: 800;
    color: var(--accent);
    letter-spacing: -3px;
    line-height: 1;
  }
  .title-ar {
    font-family: var(--font-arabic);
    font-size: 3.2rem;
    color: var(--text-muted);
    font-weight: 400;
    line-height: 1;
  }
  .hero-tagline {
    font-size: 1.5rem;
    color: var(--text-primary);
    font-weight: 600;
    letter-spacing: -0.3px;
    margin: 0;
  }
  .hero-tagline em {
    font-family: 'EB Garamond', Georgia, serif;
    color: var(--accent);
    font-style: italic;
    font-size: 1.35em;
    font-weight: 600;
  }

  /* Description */
  .hero-desc {
    position: relative;
    z-index: 1;
    font-size: 1rem;
    color: var(--text-secondary);
    max-width: 580px;
    line-height: 1.8;
    opacity: 0;
    animation: fade-in-up 0.8s 0.5s ease forwards;
  }

  /* Pills */
  .hero-pills {
    position: relative;
    z-index: 1;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    opacity: 0;
    animation: fade-in-up 0.8s 0.6s ease forwards;
  }
  .pill {
    padding: 5px 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: 20px;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    transition: all 0.2s ease;
  }
  .pill:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }

  /* Search */
  .hero-search {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 600px;
    opacity: 0;
    animation: fade-in-up 0.8s 0.7s ease forwards;
  }
  .hero-search-bar {
    width: 100%;
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 1.5px solid var(--border);
    border-radius: 16px;
    padding: 4px 6px 4px 18px;
    box-shadow: 0 4px 24px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04);
    transition: all 0.25s ease;
    gap: 12px;
  }
  .hero-search-bar:focus-within {
    border-color: var(--accent);
    box-shadow: 0 4px 24px rgba(214,51,132,0.12), 0 0 0 3px rgba(214,51,132,0.06);
  }
  .search-icon {
    width: 20px;
    height: 20px;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .hero-search-input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 14px 0;
    font-size: 0.95rem;
    color: var(--text-primary);
    outline: none;
  }
  .hero-search-input::placeholder {
    color: var(--text-muted);
  }
  .search-btn {
    padding: 10px 24px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 12px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
  }
  .search-btn:hover {
    background: var(--accent-hover);
    box-shadow: 0 2px 12px rgba(214,51,132,0.3);
  }

  /* CTAs */
  .hero-ctas {
    position: relative;
    z-index: 1;
    display: flex;
    gap: 12px;
    opacity: 0;
    animation: fade-in-up 0.8s 0.8s ease forwards;
  }
  .cta {
    padding: 12px 28px;
    border-radius: 12px;
    font-size: 0.88rem;
    font-weight: 600;
    text-decoration: none;
    transition: all 0.2s ease;
  }
  .cta-filled {
    background: var(--accent);
    color: white;
  }
  .cta-filled:hover {
    background: var(--accent-hover);
    color: white;
    box-shadow: 0 4px 20px rgba(214,51,132,0.3);
    transform: translateY(-1px);
  }
  .cta-outline {
    background: transparent;
    color: var(--text-primary);
    border: 1.5px solid var(--border);
  }
  .cta-outline:hover {
    border-color: var(--accent);
    color: white;
    background: var(--accent);
    box-shadow: 0 4px 20px rgba(214,51,132,0.3);
    transform: translateY(-1px);
  }

  /* Stats bar */
  .hero-stats {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 16px 32px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    opacity: 0;
    animation: fade-in-up 0.8s 0.9s ease forwards;
  }
  .hero-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .hero-stat-num {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .hero-stat-label {
    font-size: 0.68rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 500;
  }
  .hero-stat-dot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--border);
  }

  /* ══════════════════════════════════
     FEATURES — Apple-style stacked rows
     ══════════════════════════════════ */
  .features-section {
    max-width: 960px;
    margin: 0 auto;
    padding: 80px 24px 40px;
  }

  /* Each feature — side-by-side image + text */
  .feat {
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: 48px;
    align-items: center;
    padding: 60px 0;
  }
  .feat-reverse {
    direction: rtl;
  }
  .feat-reverse > * {
    direction: ltr;
  }

  .feat-visual {
    transition: transform 0.3s ease;
  }
  .feat:hover .feat-visual {
    transform: translateY(-2px);
  }

  /* Browser-style app frame around screenshots */
  .app-frame {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 8px 40px rgba(0,0,0,0.08), 0 2px 8px rgba(0,0,0,0.04);
  }
  .frame-dots {
    display: flex;
    gap: 6px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }
  .frame-dots span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
  }
  .app-frame img {
    width: 100%;
    height: auto;
    display: block;
  }

  .feat-text {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .feat-text h3 {
    font-size: 1.8rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.5px;
    line-height: 1.2;
  }
  .feat-text p {
    font-size: 1rem;
    color: var(--text-secondary);
    line-height: 1.7;
  }
  .feat-link {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--accent);
    text-decoration: none;
    transition: color 0.15s ease;
  }
  .feat-link:hover {
    color: var(--accent-hover);
  }
  .feat-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 12px 0;
  }
  .feat-pill {
    padding: 3px 10px;
    font-size: 0.7rem;
    font-weight: 600;
    border: 1px solid var(--border);
    border-radius: 12px;
    color: var(--text-secondary);
    background: var(--bg-surface);
  }

  /* Secondary features — compact 3-col row */
  .feat-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 36px;
    padding: 60px 0 20px;
  }

  /* ══════════════════════════════════
     DATA SOURCES
     ══════════════════════════════════ */
  .sources-section {
    padding: 0;
  }
  .sources-inner {
    max-width: 960px;
    margin: 0 auto;
    padding: 80px 24px;
  }
  .source-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }
  .source-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 28px 20px;
    text-align: center;
    transition: all 0.3s ease;
    position: relative;
    overflow: hidden;
  }
  .source-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--accent);
    opacity: 0;
    transition: opacity 0.3s ease;
  }
  .source-card:hover {
    border-color: var(--accent);
    box-shadow: 0 6px 28px rgba(214,51,132,0.08);
    transform: translateY(-3px);
  }
  .source-card:hover::before {
    opacity: 1;
  }
  .source-num {
    font-size: 1.6rem;
    font-weight: 700;
    color: var(--accent);
    font-family: var(--font-mono);
    margin-bottom: 10px;
    line-height: 1;
  }
  .source-card h4 {
    font-size: 0.9rem;
    margin-bottom: 6px;
    color: var(--text-primary);
  }
  .source-card p {
    font-size: 0.76rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  /* ══════════════════════════════════
     UNDER THE HOOD
     ══════════════════════════════════ */
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
    background: radial-gradient(ellipse, rgba(214,51,132,0.06) 0%, transparent 70%);
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
    border-radius: 14px;
    padding: 18px 20px;
    text-align: center;
    transition: all 0.3s ease;
    box-shadow: 0 2px 8px rgba(0,0,0,0.03);
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
    border-radius: 8px;
    font-size: 0.7rem;
    color: var(--text-secondary);
    text-align: center;
  }

  /* Animated lines */
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

  /* Pipeline */
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
    border-radius: 12px;
    transition: all 0.3s ease;
    box-shadow: 0 2px 8px rgba(0,0,0,0.03);
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

  /* ══════════════════════════════════
     FOOTER
     ══════════════════════════════════ */
  .landing-footer {
    border-top: 1px solid var(--border);
  }
  .footer-inner {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    max-width: 960px;
    margin: 0 auto;
    padding: 48px 24px 32px;
    gap: 48px;
  }
  .footer-brand {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .footer-logo {
    font-size: 1.4rem;
    font-weight: 800;
    color: var(--accent);
    letter-spacing: -0.5px;
  }
  .footer-tagline {
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  .footer-author {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-top: 4px;
  }
  .footer-author a {
    color: var(--accent);
    text-decoration: none;
    font-weight: 600;
  }
  .footer-author a:hover {
    text-decoration: underline;
  }
  .footer-links {
    display: flex;
    gap: 48px;
  }
  .footer-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .footer-col h4 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--text-muted);
    font-weight: 700;
    margin-bottom: 4px;
  }
  .footer-col a {
    font-size: 0.82rem;
    color: var(--text-secondary);
    text-decoration: none;
    transition: color var(--transition);
  }
  .footer-col a:hover {
    color: var(--accent);
  }
  .footer-bottom {
    text-align: center;
    padding: 20px 24px;
    border-top: 1px solid var(--border);
  }
  .footer-bottom p {
    font-size: 0.76rem;
    color: var(--text-muted);
  }

  /* ══════════════════════════════════
     RESPONSIVE
     ══════════════════════════════════ */
  @media (max-width: 700px) {
    .hero { padding: 60px 20px 48px; gap: 20px; }
    .title-en { font-size: 3rem; }
    .title-ar { font-size: 2rem; }
    .hero-tagline { font-size: 1.15rem; }
    .hero-desc { font-size: 0.9rem; }
    .hero-pills { gap: 6px; }
    .hero-ctas { flex-direction: column; align-items: center; width: 100%; max-width: 300px; }
    .cta { width: 100%; text-align: center; }
    .hero-stats { flex-wrap: wrap; justify-content: center; padding: 12px 20px; gap: 12px; }
    .feat { grid-template-columns: 1fr; gap: 24px; padding: 40px 0; }
    .feat-reverse { direction: ltr; }
    .feat-text h3 { font-size: 1.4rem; }
    .feat-row { grid-template-columns: 1fr; gap: 32px; }
    .source-grid { grid-template-columns: 1fr; }
    .hood-columns { grid-template-columns: 1fr; }
    .sub-cards { flex-direction: column; }
    .footer-inner { flex-direction: column; gap: 32px; }
    .footer-links { gap: 32px; }
  }

  @media (min-width: 701px) and (max-width: 900px) {
    .source-grid { grid-template-columns: repeat(2, 1fr); }
  }
</style>
