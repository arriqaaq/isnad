<script lang="ts">
  import { marked } from 'marked';
  import type { MultiBookSource } from '$lib/types';

  marked.setOptions({ breaks: true, gfm: true });

  let { open, verse, onclose }: {
    open: boolean;
    /**
     * Optional verse anchor. When provided, the drawer sends it along with
     * the question so the backend can take the verse-aware shortcut
     * (skip PageIndex LLM navigation, fetch exact pages from
     * `tafsir_ayah_map`). The `/tafsir` page always has this. Other call
     * sites may omit it and fall back to nav-based retrieval.
     */
    verse?: { surah: number; ayah: number };
    onclose: () => void;
  } = $props();

  interface SkippedBook { book_id: number; reason: string; }
  interface ReadingBook { book_id: number; name_en: string; sections: number; }
  // Per-book extraction progress (verse-aware path, parallel fan-out).
  // One of these arrives as each book's chat_json call finishes.
  interface ExtractedBook {
    book_id: number;
    book_name_en: string;
    entries: number;
    dropped: number;
    error?: string | null;
  }

  // Verse-aware extractive synthesis output. Each entry is a verified
  // verbatim Arabic passage plus a short explanation. The backend drops
  // any entry whose `arabic_quote` isn't literally in the source page.
  interface ExtractEntry {
    book_id: number;
    page_index: number;
    arabic_quote: string;
    english_note: string;
  }
  interface ExtractResult {
    overview: string | null;
    entries: ExtractEntry[];
    dropped: number;
  }

  // Fallback payload when validation produces zero valid entries — the UI
  // shows direct links to the raw pages we fetched so the user can read
  // them manually.
  interface AvailablePage {
    book_id: number;
    book_name_en: string;
    book_name_ar: string;
    page_index: number;
    title: string;
  }

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    sources?: MultiBookSource[];
    reading?: ReadingBook[];
    skipped?: SkippedBook[];
    status?: string;
    navigatingBooks?: number[];
    anchorVerse?: { surah: number; ayah: number };
    warning?: string;
    streaming?: boolean;
    // Verse-aware (extractive) result. When set, the UI renders structured
    // cards instead of the streaming markdown body.
    extract?: ExtractResult;
    // `no_valid_extraction` fallback pages.
    availablePages?: AvailablePage[];
    availableReason?: string;
    // Verse-aware progress: total books fanned out + those that have
    // completed so far. Shown as "3 / 4 books…" during the parallel
    // extraction phase.
    extractTotal?: number;
    extractedBooks?: ExtractedBook[];
  }

  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let chatContainer: HTMLDivElement = $state(null!);

  // "Stick to bottom" only when the user is already near the bottom. When
  // they scroll up to re-read earlier tokens, stop pinning so manual scroll
  // survives the next streaming tick.
  let stickToBottom = $state(true);
  const STICK_THRESHOLD_PX = 60;

  function handleScroll() {
    if (!chatContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = chatContainer;
    stickToBottom = scrollHeight - (scrollTop + clientHeight) <= STICK_THRESHOLD_PX;
  }

  const STORAGE_KEY = 'tafsir_ask_history';

  function loadHistory() {
    if (typeof localStorage === 'undefined') return;
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed)) messages = parsed;
      }
    } catch { /* ignore */ }
  }

  function saveHistory() {
    if (typeof localStorage === 'undefined') return;
    try {
      const toSave = messages.filter(m => !m.streaming).slice(-20);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(toSave));
    } catch { /* ignore */ }
  }

  $effect(() => {
    if (open) loadHistory();
  });

  function scrollToBottom(force = false) {
    if (!chatContainer) return;
    if (!force && !stickToBottom) return;
    requestAnimationFrame(() => {
      chatContainer.scrollTop = chatContainer.scrollHeight;
    });
  }

  function clearChat() {
    messages = [];
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem(STORAGE_KEY);
    }
  }

  async function handleSubmit(e?: Event) {
    e?.preventDefault();
    const question = input.trim();
    if (!question || loading) return;

    input = '';
    messages = [...messages, { role: 'user', content: question }];
    const assistantMsg: Message = { role: 'assistant', content: '', streaming: true };
    messages = [...messages, assistantMsg];
    loading = true;
    // New question → always jump to bottom and re-enable stickiness.
    stickToBottom = true;
    scrollToBottom(true);

    try {
      // Passing `verse` switches the backend to its fast path: it skips
      // LLM navigation and reads the exact tafsir pages for the anchored
      // ayah straight from the index. Omitting it forces the slow nav
      // fallback — fine for non-verse-anchored questions.
      const body: { question: string; verse?: { surah: number; ayah: number } } = { question };
      if (verse) body.verse = verse;

      const res = await fetch('/api/tafsir/ask', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (!res.ok) {
        const idx = messages.length - 1;
        let detail = res.statusText;
        if (res.status === 503) detail = 'No tafsir PageIndex trees loaded — run `make pageindex-build`.';
        messages[idx] = { ...messages[idx], content: `Error: ${detail}`, streaming: false };
        loading = false;
        return;
      }

      const reader = res.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      const idx = messages.length - 1;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (!line.startsWith('data: ')) continue;
          const jsonStr = line.slice(6).trim();
          if (!jsonStr) continue;
          try {
            const data = JSON.parse(jsonStr);
            if (data.status === 'no_relevant_sections') {
              messages[idx] = { ...messages[idx], warning: data.message || 'No relevant sections found.' };
            } else if (data.status === 'book_skipped') {
              const cur = messages[idx].skipped ?? [];
              messages[idx] = {
                ...messages[idx],
                skipped: [...cur, { book_id: data.book_id, reason: data.reason }],
              };
            } else if (data.status === 'reading') {
              messages[idx] = { ...messages[idx], reading: data.books, status: 'reading' };
            } else if (data.status === 'navigating') {
              messages[idx] = {
                ...messages[idx],
                status: 'navigating',
                navigatingBooks: Array.isArray(data.books) ? data.books : [],
              };
            } else if (data.status === 'loading_verse') {
              messages[idx] = {
                ...messages[idx],
                status: 'loading_verse',
                anchorVerse: data.verse,
              };
            } else if (data.status === 'extracting') {
              messages[idx] = {
                ...messages[idx],
                status: 'extracting',
                extractTotal: typeof data.books === 'number' ? data.books : undefined,
                extractedBooks: [],
              };
            } else if (data.status === 'book_extracted') {
              const cur = messages[idx].extractedBooks ?? [];
              messages[idx] = {
                ...messages[idx],
                status: 'extracting',
                extractedBooks: [
                  ...cur,
                  {
                    book_id: data.book_id,
                    book_name_en: data.book_name_en,
                    entries: data.entries ?? 0,
                    dropped: data.dropped ?? 0,
                    error: data.error ?? null,
                  },
                ],
              };
            } else if (data.status === 'no_valid_extraction') {
              // Backend bailed out — either LLM failed, or every entry
              // failed verification. Switch the message into fallback
              // mode showing direct links to the raw pages instead.
              messages[idx] = {
                ...messages[idx],
                status: 'no_valid_extraction',
                availablePages: Array.isArray(data.available_pages) ? data.available_pages : [],
                availableReason: typeof data.reason === 'string' ? data.reason : undefined,
              };
            } else if (data.status) {
              messages[idx] = { ...messages[idx], status: data.status };
            } else if (data.result) {
              // Verse-aware extractive result — already validated server-side.
              // Takes precedence over any accumulated `content` (which shouldn't
              // exist on this path anyway).
              messages[idx] = { ...messages[idx], extract: data.result, content: '' };
              scrollToBottom();
            } else if (data.sources) {
              messages[idx] = { ...messages[idx], sources: data.sources };
            } else if (data.text) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + data.text };
              scrollToBottom();
            } else if (data.done) {
              messages[idx] = { ...messages[idx], streaming: false, status: undefined };
            } else if (data.error) {
              messages[idx] = {
                ...messages[idx],
                content: messages[idx].content + `\n\n[Error: ${data.error}]`,
                streaming: false,
              };
            }
          } catch { /* skip malformed chunk */ }
        }
      }
      messages[idx] = { ...messages[idx], streaming: false, status: undefined };
      saveHistory();
    } catch (e: any) {
      const idx = messages.length - 1;
      messages[idx] = { ...messages[idx], content: `Error: ${e.message}`, streaming: false };
    } finally {
      loading = false;
      scrollToBottom();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !loading) {
      onclose();
      return;
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function statusLabel(m: Message): string {
    switch (m.status) {
      case 'loading_verse': {
        if (m.anchorVerse) {
          return `Pulling tafsir pages for ${m.anchorVerse.surah}:${m.anchorVerse.ayah}…`;
        }
        return 'Pulling tafsir pages for the verse…';
      }
      case 'navigating': {
        const n = m.navigatingBooks?.length ?? 0;
        if (n === 0) return 'Navigating tafsir books…';
        return `Navigating ${n} tafsir book${n === 1 ? '' : 's'}… (this may take up to a few minutes for local models)`;
      }
      case 'reading': return 'Reading sections…';
      case 'extracting': {
        const total = m.extractTotal ?? 0;
        const done = m.extractedBooks?.length ?? 0;
        if (total > 0) {
          return `Extracting passages… (${done} / ${total} book${total === 1 ? '' : 's'} done)`;
        }
        return 'Asking the model to pick relevant Arabic passages…';
      }
      default: return m.status ?? '';
    }
  }

  const suggestions = [
    'What do the tafsirs say about the Throne Verse?',
    'How do Ibn Kathir and al-Tabari differ on Al-Fatihah?',
    'Compare their commentary on 2:30.',
  ];
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
{#if open}
  <div class="drawer-backdrop" onclick={onclose}></div>
  <aside class="drawer" role="dialog" aria-label="Ask AI across tafsirs">
    <div class="drawer-header">
      <div class="header-left">
        <span class="drawer-title">Ask across all tafsirs</span>
        {#if messages.length > 0}
          <button class="header-btn" type="button" onclick={clearChat} title="New chat">+ New</button>
        {/if}
      </div>
      <button class="close-btn" type="button" onclick={onclose} aria-label="Close">×</button>
    </div>

    <div
      class="drawer-messages"
      bind:this={chatContainer}
      onkeydown={handleKeydown}
      onscroll={handleScroll}
    >
      {#if messages.length === 0}
        <div class="empty-state">
          <p class="empty-line">Ask a question — the answer is synthesized across every loaded tafsir book.</p>
          <div class="suggestions">
            {#each suggestions as s}
              <button class="suggest-btn" type="button" onclick={() => { input = s; handleSubmit(); }}>{s}</button>
            {/each}
          </div>
        </div>
      {/if}

      {#each messages as msg, i (i)}
        <div class="msg" class:user={msg.role === 'user'} class:assistant={msg.role === 'assistant'}>
          {#if msg.role === 'user'}
            <div class="msg-body user-body">{msg.content}</div>
          {:else}
            {#if msg.status}
              <div class="msg-status">{statusLabel(msg)}</div>
            {/if}
            {#if msg.reading && msg.reading.length > 0}
              <ul class="reading-list">
                {#each msg.reading as r}
                  <li>{r.name_en}: <span class="muted">{r.sections} {r.sections === 1 ? 'section' : 'sections'}</span></li>
                {/each}
              </ul>
            {/if}
            {#if msg.extractedBooks && msg.extractedBooks.length > 0 && !msg.extract && msg.status !== 'no_valid_extraction'}
              <ul class="reading-list">
                {#each msg.extractedBooks as b}
                  <li>
                    {b.book_name_en}:
                    {#if b.error}
                      <span class="muted">{b.error}</span>
                    {:else}
                      <span class="muted">{b.entries} {b.entries === 1 ? 'passage' : 'passages'}{b.dropped > 0 ? ` (${b.dropped} dropped)` : ''}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
            {#if msg.skipped && msg.skipped.length > 0}
              <details class="skipped">
                <summary>{msg.skipped.length} book{msg.skipped.length === 1 ? '' : 's'} skipped</summary>
                <ul>
                  {#each msg.skipped as s}
                    <li>book {s.book_id}: <span class="muted">{s.reason}</span></li>
                  {/each}
                </ul>
              </details>
            {/if}
            {#if msg.warning}
              <div class="msg-warning">{msg.warning}</div>
            {/if}
            <!--
              Template order matters: sources render BEFORE the streaming
              content. With scrollToBottom() firing on every token, the
              viewport tracks the bottom of the message — which needs to
              be the streaming cursor, not a static list. Also collapsed
              by default so a long page window (Tabari commentaries can
              run 20+ pages) doesn't eat the screen before the answer
              starts arriving.
            -->
            {#if msg.sources && msg.sources.length > 0}
              <details class="msg-sources">
                <summary>Sources · {msg.sources.length} page{msg.sources.length === 1 ? '' : 's'}</summary>
                <ul>
                  {#each msg.sources as src}
                    <li>
                      <a
                        class="src-link"
                        href="/tafsir/{src.book_id}?page={src.line}"
                        target="_blank"
                        rel="noopener"
                        title="Open in full reader"
                      >
                        <span class="src-book">{src.book_name_en}</span>
                        <span class="src-sep">·</span>
                        <span class="src-title">{src.title}</span>
                      </a>
                    </li>
                  {/each}
                </ul>
              </details>
            {/if}
            <!--
              Verse-aware extractive path: render structured cards.
              Every card's attribution line is a link to the full reader —
              that IS the "inline linking". No post-processing of generated
              prose, nothing to parse from LLM output, can't break.
            -->
            {#if msg.extract}
              {@const e = msg.extract}
              {#if e.overview}
                <p class="extract-overview">{e.overview}</p>
              {/if}
              {#if e.entries.length === 0}
                <div class="extract-empty">The model didn't return any verified passages for this question.</div>
              {:else}
                <div class="extract-cards">
                  {#each e.entries as entry}
                    {@const src = msg.sources?.find(
                      (s) => s.book_id === entry.book_id && s.line === entry.page_index,
                    )}
                    <article class="extract-card">
                      <header class="card-attribution">
                        <a
                          class="card-link"
                          href="/tafsir/{entry.book_id}?page={entry.page_index}"
                          target="_blank"
                          rel="noopener"
                          title="Open in full reader"
                        >
                          <span class="card-book">{src?.book_name_en ?? `Book ${entry.book_id}`}</span>
                          <span class="card-sep">·</span>
                          <span class="card-page">{src?.title ?? `Page ${entry.page_index}`}</span>
                          <span class="card-arrow" aria-hidden="true">→</span>
                        </a>
                      </header>
                      <blockquote class="arabic-verbatim" dir="rtl" lang="ar">{entry.arabic_quote}</blockquote>
                      {#if entry.english_note}
                        <p class="english-note">{entry.english_note}</p>
                      {/if}
                    </article>
                  {/each}
                </div>
              {/if}
              {#if e.dropped > 0}
                <p class="drop-note">
                  {e.dropped} source{e.dropped === 1 ? '' : 's'} dropped — likely paraphrased or hallucinated by the model.
                </p>
              {/if}
            {:else if msg.status === 'no_valid_extraction'}
              <!-- Fallback: synthesis failed; show the raw pages we pulled. -->
              <div class="extract-fallback">
                <p class="fallback-lede">
                  Couldn't synthesize an answer{msg.availableReason ? ` (${msg.availableReason})` : ''}.
                  Here are the pages pulled for this verse — open any directly:
                </p>
                {#if msg.availablePages && msg.availablePages.length > 0}
                  <ul class="fallback-pages">
                    {#each msg.availablePages as p}
                      <li>
                        <a
                          href="/tafsir/{p.book_id}?page={p.page_index}"
                          target="_blank"
                          rel="noopener"
                        >
                          {p.book_name_en} · {p.title} →
                        </a>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {:else if msg.content}
              <div class="msg-body assistant-body">{@html marked.parse(msg.content)}</div>
            {/if}
          {/if}
        </div>
      {/each}
    </div>

    <form class="drawer-input" onsubmit={handleSubmit}>
      <textarea
        bind:value={input}
        onkeydown={handleKeydown}
        placeholder="Ask a question about any verse…"
        rows="2"
        disabled={loading}
      ></textarea>
      <button class="send-btn" type="submit" disabled={loading || !input.trim()}>
        {loading ? '…' : 'Send'}
      </button>
    </form>
  </aside>
{/if}

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    z-index: 250;
    animation: fadeIn 0.12s ease-out;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(460px, 100vw);
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    box-shadow: -8px 0 28px rgba(0, 0, 0, 0.18);
    display: flex;
    flex-direction: column;
    z-index: 260;
    animation: slideInRight 0.18s ease-out;
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideInRight {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }

  .drawer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .header-left { display: flex; align-items: center; gap: 10px; }
  .drawer-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .header-btn {
    padding: 3px 10px;
    font-size: 0.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .header-btn:hover { border-color: var(--accent); color: var(--accent); }
  .close-btn {
    font-size: 1.5rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }
  .close-btn:hover { color: var(--text-primary); }

  .drawer-messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .empty-state {
    color: var(--text-muted);
    font-size: 0.9rem;
    text-align: center;
    padding: 20px 8px;
  }
  .empty-line { margin-bottom: 16px; }
  .suggestions { display: flex; flex-direction: column; gap: 6px; }
  .suggest-btn {
    text-align: left;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-size: 0.85rem;
    cursor: pointer;
    transition: all var(--transition);
  }
  .suggest-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .msg { display: flex; flex-direction: column; gap: 6px; }
  .msg.user { align-items: flex-end; }
  .msg-body { font-size: 0.92rem; line-height: 1.6; max-width: 100%; }
  .user-body {
    background: var(--accent-muted);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 12px 12px 3px 12px;
    max-width: 85%;
  }
  .assistant-body {
    color: var(--text-primary);
  }
  .assistant-body :global(p) { margin: 0.4rem 0; }
  .assistant-body :global(ul), .assistant-body :global(ol) { margin: 0.4rem 0; padding-left: 1.2rem; }
  .assistant-body :global(code) {
    font-family: var(--font-mono);
    background: var(--bg-surface);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .msg-status {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-style: italic;
  }
  .reading-list {
    list-style: none;
    margin: 0;
    padding: 6px 10px;
    font-size: 0.78rem;
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
  }
  .reading-list .muted { color: var(--text-muted); }
  .skipped {
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  .skipped summary { cursor: pointer; }
  .skipped ul { margin: 4px 0 0; padding-left: 1.1rem; }
  .skipped .muted { color: var(--text-muted); font-style: italic; }

  .msg-warning {
    font-size: 0.82rem;
    color: var(--warning, #a66);
    background: var(--warning-muted, rgba(200, 100, 100, 0.1));
    padding: 8px 10px;
    border-radius: var(--radius-sm);
  }

  .msg-sources {
    font-size: 0.78rem;
    color: var(--text-muted);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
  }
  .msg-sources summary {
    cursor: pointer;
    font-weight: 500;
    padding: 2px 0;
    color: var(--text-secondary);
    list-style: revert;
  }
  .msg-sources summary:hover { color: var(--text-primary); }
  .msg-sources[open] summary {
    margin-bottom: 4px;
    padding-bottom: 4px;
    border-bottom: 1px dashed var(--border-subtle);
  }
  .msg-sources ul {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 200px;
    overflow-y: auto;
  }
  .msg-sources li { padding: 2px 0; }
  .src-link {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    text-decoration: none;
    color: inherit;
    padding: 2px 4px;
    margin: -2px -4px;
    border-radius: var(--radius-sm);
    transition: background var(--transition);
  }
  .src-link:hover {
    background: var(--bg-primary);
  }
  .src-link:hover .src-book { text-decoration: underline; }
  .src-book { color: var(--accent); font-weight: 500; }
  .src-sep { color: var(--text-muted); margin: 0 4px; }
  .src-title {
    color: var(--text-secondary);
    font-family: var(--font-arabic-text), serif;
  }

  /* ── Extractive result cards ─────────────────────────────── */
  .extract-overview {
    font-size: 0.88rem;
    line-height: 1.55;
    color: var(--text-primary);
    margin: 0 0 12px;
  }
  .extract-cards {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .extract-card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    background: var(--bg-surface);
    overflow: hidden;
  }
  .card-attribution {
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-subtle);
    padding: 6px 12px;
    font-size: 0.78rem;
  }
  .card-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-decoration: none;
    color: var(--text-secondary);
    transition: color var(--transition);
  }
  .card-link:hover { color: var(--accent); }
  .card-book { font-weight: 600; color: var(--text-primary); }
  .card-link:hover .card-book { color: var(--accent); }
  .card-sep { color: var(--text-muted); }
  .card-page { font-family: var(--font-mono); color: var(--text-muted); }
  .card-arrow { color: var(--accent); font-size: 0.85rem; }

  .arabic-verbatim {
    direction: rtl;
    text-align: right;
    font-family: var(--font-arabic-text), 'Noto Naskh Arabic', serif;
    font-size: 1.05rem;
    line-height: 2;
    color: var(--text-primary);
    margin: 0;
    padding: 12px 14px;
    border-left: 3px solid var(--accent);
    border-left-width: 0;
    border-right: 3px solid var(--accent);
    background: var(--bg-primary);
  }
  .english-note {
    margin: 0;
    padding: 10px 14px 12px;
    font-size: 0.85rem;
    line-height: 1.55;
    color: var(--text-secondary);
    border-top: 1px dashed var(--border-subtle);
    background: var(--bg-primary);
  }
  .drop-note {
    margin: 10px 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    font-style: italic;
  }
  .extract-empty {
    padding: 12px;
    border: 1px dashed var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  /* ── No-valid-extraction fallback ─────────────────────────── */
  .extract-fallback {
    padding: 10px 14px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .fallback-lede { margin: 0 0 8px; }
  .fallback-pages { list-style: none; margin: 0; padding: 0; }
  .fallback-pages li { padding: 2px 0; }
  .fallback-pages a { color: var(--accent); text-decoration: none; font-size: 0.82rem; }
  .fallback-pages a:hover { text-decoration: underline; }

  .drawer-input {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-secondary);
  }
  .drawer-input textarea {
    flex: 1;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.9rem;
    resize: none;
  }
  .drawer-input textarea:focus {
    outline: none;
    border-color: var(--accent);
  }
  .send-btn {
    padding: 0 16px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--bg-primary);
    font-weight: 600;
    cursor: pointer;
    transition: opacity var(--transition);
  }
  .send-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
