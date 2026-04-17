<script lang="ts">
  import { page } from '$app/state';
  import { getNarrator, getNarratorGraph, updateNarrator, getNarratorBooks, getTurathPages } from '$lib/api';
  import type { NarratorDetailResponse, GraphData, NarratorBookRef, TurathPage } from '$lib/types';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import GraphView from '$lib/components/graph/GraphView.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import ReaderPage from '$lib/components/reader/ReaderPage.svelte';

  let data: NarratorDetailResponse | null = $state(null);
  let graphData: GraphData | null = $state(null);
  let narratorBooks: NarratorBookRef[] = $state([]);
  let selectedBookRef: NarratorBookRef | null = $state(null);
  let bioPage: TurathPage | null = $state(null);
  let bioPageLoading = $state(false);
  let bioCurrentIndex = $state(0);
  let loading = $state(true);
  let activeTab: 'network' | 'hadiths' | 'connections' | 'readbio' | 'details' = $state('network');
  let saving = $state(false);
  let saveMsg = $state('');

  // Editable fields
  let editGender = $state('');
  let editGeneration = $state('');
  let editBio = $state('');
  let editKunya = $state('');
  let editBirthYear = $state('');
  let editBirthCalendar = $state('hijri');
  let editDeathYear = $state('');
  let editDeathCalendar = $state('hijri');
  let editLocations = $state('');
  let editTags = $state('');

  let id = $derived(page.params.id);

  async function loadBioPage(bookRef: NarratorBookRef, pageIndex: number) {
    bioPageLoading = true;
    bioPage = null;
    bioCurrentIndex = pageIndex;
    try {
      const res = await getTurathPages(bookRef.turath_book_id, pageIndex, 1);
      if (res.pages.length > 0) bioPage = res.pages[0];
    } catch (e) {
      console.error('Failed to load bio page:', e);
    } finally {
      bioPageLoading = false;
    }
  }

  function selectBook(ref: NarratorBookRef) {
    selectedBookRef = ref;
    loadBioPage(ref, ref.page_index);
  }

  function populateForm() {
    if (!data) return;
    const n = data.narrator;
    editGender = n.gender ?? '';
    editGeneration = n.generation ?? '';
    editBio = n.bio ?? '';
    editKunya = n.kunya ?? '';
    editBirthYear = n.birth_year?.toString() ?? '';
    editBirthCalendar = n.birth_calendar ?? 'hijri';
    editDeathYear = n.death_year?.toString() ?? '';
    editDeathCalendar = n.death_calendar ?? 'hijri';
    editLocations = n.locations?.join(', ') ?? '';
    editTags = n.tags?.join(', ') ?? '';
  }

  $effect(() => {
    if (!id) return;
    loading = true;
    activeTab = 'network';
    Promise.all([getNarrator(id), getNarratorGraph(id)])
      .then(([d, g]) => {
        const seen = new Set<string>();
        d.hadiths = d.hadiths.filter(h => {
          if (seen.has(h.id)) return false;
          seen.add(h.id);
          return true;
        });
        data = d;
        graphData = g;
        // Fetch book references for this narrator
        getNarratorBooks(id)
          .then(books => {
            narratorBooks = books;
            if (books.length > 0) selectBook(books[0]);
          })
          .catch(() => {});
        populateForm();
      })
      .catch((e) => console.error('Failed to load narrator:', e))
      .finally(() => { loading = false; });
  });


  async function handleSave() {
    if (!data) return;
    saving = true;
    saveMsg = '';
    const payload: Record<string, unknown> = {};

    if (editGender) payload.gender = editGender;
    if (editGeneration) payload.generation = editGeneration;
    if (editBio) payload.bio = editBio;
    if (editKunya) payload.kunya = editKunya;
    if (editBirthYear) payload.birth_year = parseInt(editBirthYear);
    if (editBirthCalendar) payload.birth_calendar = editBirthCalendar;
    if (editDeathYear) payload.death_year = parseInt(editDeathYear);
    if (editDeathCalendar) payload.death_calendar = editDeathCalendar;
    if (editLocations.trim()) payload.locations = editLocations.split(',').map(s => s.trim()).filter(Boolean);
    if (editTags.trim()) payload.tags = editTags.split(',').map(s => s.trim()).filter(Boolean);

    try {
      await updateNarrator(data.narrator.id, payload);
      saveMsg = 'Saved';
      // Refresh data
      const d = await getNarrator(id!);
      const seen = new Set<string>();
      d.hadiths = d.hadiths.filter(h => { if (seen.has(h.id)) return false; seen.add(h.id); return true; });
      data = d;
      populateForm();
    } catch (e) {
      saveMsg = 'Error saving';
      console.error(e);
    } finally {
      saving = false;
      setTimeout(() => { saveMsg = ''; }, 3000);
    }
  }
</script>

<div class="narrator-view">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <div class="hero">
      {#if data.narrator.name_ar}
        <h1 class="hero-name">{data.narrator.name_ar}</h1>
      {/if}
      {#if data.narrator.name_en && data.narrator.name_en !== data.narrator.name_ar}
        <h2 class="hero-name-secondary">{data.narrator.name_en}</h2>
      {/if}

      <div class="hero-meta">
        {#if data.narrator.death_year}
          <span class="meta-item">d. {data.narrator.death_year} {data.narrator.death_calendar === 'gregorian' ? 'CE' : 'AH'}</span>
        {:else if data.narrator.birth_year}
          <span class="meta-item">b. {data.narrator.birth_year} {data.narrator.birth_calendar === 'gregorian' ? 'CE' : 'AH'}</span>
        {/if}
        {#if data.narrator.generation}
          <span class="meta-item">Generation {data.narrator.generation}</span>
        {/if}
        {#if data.hadiths.length > 0}
          <span class="meta-item">{data.hadiths.length} {data.hadiths.length === 1 ? 'hadith' : 'hadiths'}</span>
        {/if}
        {#if data.teachers.length > 0}
          <span class="meta-item meta-teachers">{data.teachers.length} {data.teachers.length === 1 ? 'teacher' : 'teachers'}</span>
        {/if}
        {#if data.students.length > 0}
          <span class="meta-item meta-students">{data.students.length} {data.students.length === 1 ? 'student' : 'students'}</span>
        {/if}
        {#if data.narrator.locations && data.narrator.locations.length > 0}
          <span class="meta-item">{data.narrator.locations.join(', ')}</span>
        {/if}
      </div>

      {#if data.narrator.kunya || (data.narrator.aliases && data.narrator.aliases.length > 0)}
        {@const otherNames = [
          ...(data.narrator.kunya ? [data.narrator.kunya] : []),
          ...(data.narrator.aliases ?? [])
        ]}
        <p class="hero-also-known">Also known as <span class="known-names">{otherNames.join(' · ')}</span></p>
      {/if}

      {#if data.narrator.bio}
        <p class="hero-bio">{data.narrator.bio}</p>
      {/if}
    </div>

    <div class="tabs">
      <button type="button" class="tab" class:active={activeTab === 'network'} onclick={() => { activeTab = 'network'; }}>Network</button>
      <button type="button" class="tab" class:active={activeTab === 'hadiths'} onclick={() => { activeTab = 'hadiths'; }}>Hadiths ({data.hadiths.length})</button>
      <button type="button" class="tab" class:active={activeTab === 'connections'} onclick={() => { activeTab = 'connections'; }}>Connections</button>
      {#if narratorBooks.length > 0}
        <button type="button" class="tab" class:active={activeTab === 'readbio'} onclick={() => { activeTab = 'readbio'; }}>Read Bio</button>
      {/if}
      <button type="button" class="tab" class:active={activeTab === 'details'} onclick={() => { activeTab = 'details'; }}>Details</button>
    </div>

    <div class="tab-content" class:tab-content-network={activeTab === 'network'}>
      {#if activeTab === 'network'}
        <GraphView data={graphData} />
      {:else if activeTab === 'hadiths'}
        <div class="hadith-list">
          {#each data.hadiths as hadith (hadith.id)}
            <HadithCard {hadith} />
          {/each}
          {#if data.hadiths.length === 0}
            <div class="empty">No hadiths linked to this narrator.</div>
          {/if}
        </div>
      {:else if activeTab === 'connections'}
        {#if data.teachers.length > 0}
          <div class="connection-group">
            <h3>Teachers (heard from)</h3>
            <div class="chips">{#each data.teachers as teacher}<NarratorChip narrator={teacher} />{/each}</div>
          </div>
        {/if}
        {#if data.students.length > 0}
          <div class="connection-group">
            <h3>Students (narrated to)</h3>
            <div class="chips">{#each data.students as student}<NarratorChip narrator={student} />{/each}</div>
          </div>
        {/if}
        {#if data.teachers.length === 0 && data.students.length === 0}
          <div class="empty">No connections found.</div>
        {/if}
      {:else if activeTab === 'readbio'}
        <div class="readbio-tab">
          {#if narratorBooks.length > 1}
            <div class="bio-book-selector">
              <label class="bio-book-label" for="bio-book-select">Book</label>
              <select id="bio-book-select" class="bio-book-select" onchange={(e) => {
                const idx = parseInt((e.target as HTMLSelectElement).value);
                const ref = narratorBooks[idx];
                if (ref) selectBook(ref);
              }}>
                {#each narratorBooks as book, i}
                  <option value={i} selected={book === selectedBookRef}>{book.book_name}</option>
                {/each}
              </select>
            </div>
          {:else if narratorBooks.length === 1}
            <div class="bio-book-header">{narratorBooks[0].book_name}</div>
          {/if}

          <div class="bio-reader">
            {#if bioPageLoading}
              <div class="bio-loading">Loading...</div>
            {:else if bioPage}
              <ReaderPage page={bioPage} />
            {:else}
              <div class="bio-loading">No page available</div>
            {/if}
          </div>

          <div class="bio-nav">
            <button class="bio-nav-btn" onclick={() => { if (selectedBookRef) loadBioPage(selectedBookRef, bioCurrentIndex + 1); }} disabled={bioPageLoading}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg>
              Next
            </button>
            <span class="bio-nav-page">
              {#if bioPage}
                Vol {bioPage.vol} &middot; Page {bioPage.page_num}
              {/if}
            </span>
            <button class="bio-nav-btn" onclick={() => { if (selectedBookRef) loadBioPage(selectedBookRef, bioCurrentIndex - 1); }} disabled={bioPageLoading || bioCurrentIndex <= 0}>
              Prev
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          </div>

          {#if selectedBookRef}
            <div class="bio-full-link">
              <a href="/tafsir/{selectedBookRef.turath_book_id}?page={bioCurrentIndex}">Open full reader &#x2197;</a>
            </div>
          {/if}
        </div>

      {:else if activeTab === 'details'}
        <form class="details-form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
          <div class="form-section">
            <h3>Classification</h3>
            <div class="form-row">
              <label>
                <span>Generation</span>
                <input type="text" bind:value={editGeneration} placeholder="e.g., Sahabi, Tabi'i" />
              </label>
              <label>
                <span>Gender</span>
                <input type="text" bind:value={editGender} placeholder="Male / Female" />
              </label>
            </div>
          </div>

          <div class="form-section">
            <h3>Biography</h3>
            <div class="form-row">
              <label>
                <span>Kunya</span>
                <input type="text" bind:value={editKunya} placeholder="e.g., Abu Huraira" />
              </label>
            </div>
            <div class="form-row">
              <label class="half">
                <span>Birth Year</span>
                <input type="number" bind:value={editBirthYear} placeholder="Year" />
              </label>
              <label class="quarter">
                <span>Calendar</span>
                <select bind:value={editBirthCalendar}>
                  <option value="hijri">Hijri</option>
                  <option value="gregorian">Gregorian</option>
                </select>
              </label>
              <label class="half">
                <span>Death Year</span>
                <input type="number" bind:value={editDeathYear} placeholder="Year" />
              </label>
              <label class="quarter">
                <span>Calendar</span>
                <select bind:value={editDeathCalendar}>
                  <option value="hijri">Hijri</option>
                  <option value="gregorian">Gregorian</option>
                </select>
              </label>
            </div>
            <label>
              <span>Locations (comma-separated)</span>
              <input type="text" bind:value={editLocations} placeholder="e.g., Madinah, Makkah, Kufa" />
            </label>
            <label>
              <span>Tags (comma-separated)</span>
              <input type="text" bind:value={editTags} placeholder="e.g., thiqah, hafiz, mujtahid" />
            </label>
            <label>
              <span>Bio</span>
              <textarea bind:value={editBio} rows="4" placeholder="Biographical notes..."></textarea>
            </label>
          </div>

          <div class="form-actions">
            <button type="submit" class="save-btn" disabled={saving}>
              {saving ? 'Saving...' : 'Save Changes'}
            </button>
            {#if saveMsg}
              <span class="save-msg" class:error={saveMsg === 'Error saving'}>{saveMsg}</span>
            {/if}
          </div>
        </form>
      {/if}
    </div>
  {:else}
    <div class="empty">Narrator not found.</div>
  {/if}
</div>


<style>
  .narrator-view { padding: 24px; max-width: 1200px; }

  /* Hero header */
  .hero { margin-bottom: 28px; }
  .hero-name {
    font-size: 2rem;
    font-weight: 700;
    line-height: 1.2;
    color: var(--text-primary);
    margin: 0;
    font-family: 'Scheherazade New', var(--font-serif), serif;
  }
  .hero-name-secondary {
    font-size: 1.25rem;
    font-weight: 500;
    color: var(--text-secondary);
    margin: 6px 0 0;
    line-height: 1.3;
  }
  /* Horizontal dotted metadata line */
  .hero-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0;
    margin-top: 24px;
    font-size: 0.88rem;
    color: var(--text-secondary);
    line-height: 1.6;
  }
  .meta-item {
    white-space: nowrap;
  }
  .meta-item + .meta-item::before {
    content: '·';
    margin: 0 8px;
    color: var(--text-muted);
    font-weight: 700;
  }
  .meta-teachers { color: var(--graph-teacher); font-weight: 500; }
  .meta-students { color: var(--graph-student); font-weight: 500; }
  .hero-also-known {
    margin: 6px 0 0;
    font-size: 0.95rem;
    color: var(--text-muted);
  }
  .known-names {
    font-family: 'Noto Naskh Arabic', var(--font-arabic-text), serif;
    font-size: 1.05rem;
    color: var(--text-secondary);
  }
  .hero-bio {
    color: var(--text-secondary);
    font-size: 0.92rem;
    line-height: 1.7;
    margin-top: 12px;
    max-height: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @media (min-width: 768px) {
    .hero-name { font-size: 2.75rem; }
    .hero-name-secondary { font-size: 1.5rem; }
  }
  @media (min-width: 1024px) {
    .hero-name { font-size: 3.5rem; }
    .hero-name-secondary { font-size: 1.85rem; }
  }

  /* Read Bio tab */
  .readbio-tab { padding: 8px 0; }
  .bio-book-selector { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; }
  .bio-book-label { font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--accent); }
  .bio-book-select {
    flex: 1; max-width: 300px; padding: 6px 10px;
    border: 1px solid var(--border); border-radius: var(--radius-sm);
    background: var(--bg-primary); color: var(--text-primary); font-size: 0.82rem; outline: none;
  }
  .bio-book-select:focus { border-color: var(--accent); }
  .bio-book-header { font-size: 0.85rem; font-weight: 600; color: var(--text-primary); margin-bottom: 12px; }
  .bio-reader {
    background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius);
    padding: 16px 20px; min-height: 200px;
  }
  .bio-loading { display: flex; align-items: center; justify-content: center; min-height: 150px; color: var(--text-muted); font-size: 0.85rem; }
  .bio-nav {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 0; margin-top: 8px;
  }
  .bio-nav-btn {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 0.78rem; font-weight: 500; color: var(--text-secondary);
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-sm); padding: 6px 14px; cursor: pointer;
    transition: all var(--transition);
  }
  .bio-nav-btn:hover:not(:disabled) { background: var(--bg-hover); border-color: var(--accent); color: var(--accent); }
  .bio-nav-btn:disabled { opacity: 0.3; cursor: default; }
  .bio-nav-page { font-size: 0.75rem; color: var(--text-muted); font-family: var(--font-mono); }
  .bio-full-link { text-align: center; margin-top: 8px; }
  .bio-full-link a { font-size: 0.72rem; color: var(--text-muted); text-decoration: none; }
  .bio-full-link a:hover { color: var(--accent); text-decoration: underline; }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 20px;
    position: sticky;
    top: 0;
    background: var(--bg-primary);
    z-index: 10;
    padding-top: 4px;
  }
  .tab {
    padding: 10px 16px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 2px solid transparent;
    transition: all var(--transition);
    margin-bottom: -1px;
    cursor: pointer;
  }
  .tab:hover { color: var(--text-primary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .hadith-list { display: flex; flex-direction: column; gap: 12px; }
  .connection-group { margin-bottom: 20px; }
  .connection-group h3 { margin-bottom: 10px; color: var(--text-secondary); font-size: 0.9rem; text-transform: uppercase; letter-spacing: 0.5px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
  .tab-content-network { height: calc(100vh - 180px); min-height: 500px; }
  @media (max-width: 768px) { .tab-content-network { min-height: 400px; } }

  /* Details form */
  .details-form { display: flex; flex-direction: column; gap: 20px; }
  .form-section { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; }
  .form-section h3 { font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-secondary); margin-bottom: 16px; }
  .form-row { display: flex; gap: 12px; margin-bottom: 12px; flex-wrap: wrap; }
  .form-row label { flex: 1; }
  .form-row label.half { flex: 2; }
  .form-row label.quarter { flex: 1; }
  label { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; }
  label span { font-size: 0.8rem; color: var(--text-secondary); }
  input, select, textarea {
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.9rem;
    font-family: inherit;
  }
  input:focus, select:focus, textarea:focus { border-color: var(--accent); outline: none; }
  textarea { resize: vertical; }
  .form-actions { display: flex; align-items: center; gap: 12px; }
  .save-btn {
    padding: 10px 24px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.9rem;
  }
  .save-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .save-msg { font-size: 0.85rem; color: var(--accent); }
  .save-msg.error { color: #ef4444; }
</style>
