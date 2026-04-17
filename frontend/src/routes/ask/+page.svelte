<script lang="ts">
  import type { ApiHadithSearchResult, ApiAyahSearchResult } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import { marked } from 'marked';

  marked.setOptions({ breaks: true, gfm: true });

  type SourceMode = 'both' | 'quran' | 'hadith';

  interface NarratorSource {
    id: string;
    name_ar?: string;
    name_en: string;
    generation?: string;
    hadith_count?: number;
    kunya?: string;
    bio?: string;
    death_year?: number;
    teachers?: { id: string; name_ar?: string; name_en: string; generation?: string }[];
    students?: { id: string; name_ar?: string; name_en: string; generation?: string }[];
  }

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    hadith_sources?: ApiHadithSearchResult[];
    quran_sources?: ApiAyahSearchResult[];
    narrator_sources?: NarratorSource[];
    streaming?: boolean;
  }

  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let sourceMode: SourceMode = $state('both');
  let chatContainer: HTMLDivElement = $state(null!);

  function scrollToBottom() {
    if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
  }

  function getEndpoint(): string {
    switch (sourceMode) {
      case 'quran': return '/api/quran/ask';
      case 'hadith': return '/api/ask';
      case 'both': return '/api/unified/ask';
    }
  }

  function getTitle(): string {
    switch (sourceMode) {
      case 'quran': return 'Ask about the Quran';
      case 'hadith': return 'Ask about Hadith';
      case 'both': return 'Ask about Quran & Sunnah';
    }
  }

  function getPlaceholder(): string {
    switch (sourceMode) {
      case 'quran': return 'Ask about the Quran...';
      case 'hadith': return 'Ask about hadiths...';
      case 'both': return 'Ask about Quran & Sunnah...';
    }
  }

  const suggestions: Record<SourceMode, { label: string; text: string }[]> = {
    both: [
      { label: 'Patience', text: 'What do the Quran and Hadith say about patience?' },
      { label: 'Abu Huraira', text: 'How many hadiths did Abu Huraira narrate?' },
      { label: 'Teachers', text: "Who were Imam al-Bukhari's teachers?" },
    ],
    quran: [
      { label: 'Patience', text: 'What does the Quran say about patience?' },
      { label: 'Charity', text: 'What are the verses about charity and giving?' },
      { label: 'Justice', text: 'What does the Quran say about justice?' },
    ],
    hadith: [
      { label: 'Neighbors', text: 'What did the Prophet say about kindness to neighbors?' },
      { label: 'Abu Huraira', text: 'How many hadiths did Abu Huraira narrate?' },
      { label: 'Teachers', text: "Who were Imam al-Bukhari's teachers?" },
    ],
  };

  function switchMode(mode: SourceMode) {
    if (mode !== sourceMode) {
      sourceMode = mode;
      messages = [];
    }
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    const question = input.trim();
    if (!question || loading) return;

    input = '';
    messages = [...messages, { role: 'user', content: question }];
    const assistantMsg: Message = { role: 'assistant', content: '', streaming: true };
    messages = [...messages, assistantMsg];
    loading = true;

    try {
      const res = await fetch(getEndpoint(), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question }),
      });

      if (!res.ok) {
        const idx = messages.length - 1;
        messages[idx] = { ...messages[idx], content: `Error: ${res.statusText}`, streaming: false };
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
            // Unified: {quran_sources, hadith_sources, narrator_sources}
            if (data.quran_sources || data.hadith_sources || data.narrator_sources) {
              messages[idx] = {
                ...messages[idx],
                quran_sources: data.quran_sources,
                hadith_sources: data.hadith_sources,
                narrator_sources: data.narrator_sources,
              };
            }
            // Hadith-only: {sources}
            else if (data.sources && sourceMode === 'hadith') {
              messages[idx] = { ...messages[idx], hadith_sources: data.sources };
            }
            // Quran-only: {sources}
            else if (data.sources && sourceMode === 'quran') {
              messages[idx] = { ...messages[idx], quran_sources: data.sources };
            }
            else if (data.text) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + data.text };
              scrollToBottom();
            } else if (data.done) {
              messages[idx] = { ...messages[idx], streaming: false };
            } else if (data.error) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + `\n\n[Error: ${data.error}]`, streaming: false };
            }
          } catch { /* skip */ }
        }
      }
      messages[idx] = { ...messages[idx], streaming: false };
    } catch (e: any) {
      const idx = messages.length - 1;
      messages[idx] = { ...messages[idx], content: `Error: ${e.message}`, streaming: false };
    } finally {
      loading = false;
      scrollToBottom();
    }
  }
</script>

<div class="ask-page">
  <div class="ask-header">
    <div class="mode-toggle">
      <button class="mode-btn" class:active={sourceMode === 'both'} onclick={() => switchMode('both')}>Both</button>
      <button class="mode-btn" class:active={sourceMode === 'quran'} onclick={() => switchMode('quran')}>Quran</button>
      <button class="mode-btn" class:active={sourceMode === 'hadith'} onclick={() => switchMode('hadith')}>Hadith</button>
    </div>
  </div>

  <div class="chat-container" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="empty-state">
        <div class="empty-icon">{sourceMode === 'quran' ? '◈' : sourceMode === 'hadith' ? '☰' : '✦'}</div>
        <h2>{getTitle()}</h2>
        <p>Answers are grounded in {sourceMode === 'both' ? 'Quranic verses, Tafsir, and Hadith' : sourceMode === 'quran' ? 'Quranic verses and Tafsir Ibn Kathir' : 'hadith texts'} using semantic search.</p>
        <div class="suggestions">
          {#each suggestions[sourceMode] as s}
            <button class="suggestion" onclick={() => { input = s.text; }}>{s.label}</button>
          {/each}
        </div>
      </div>
    {/if}

    {#each messages as msg}
      <div class="message {msg.role}">
        <div class="message-header">
          <span class="role-label">{msg.role === 'user' ? 'You' : 'Assistant'}</span>
        </div>
        <div class="message-content">
          {#if msg.role === 'assistant'}
            <div class="assistant-text prose">{@html marked(msg.content)}{#if msg.streaming}<span class="cursor">|</span>{/if}</div>

            {#if msg.quran_sources && msg.quran_sources.length > 0}
              <details class="sources">
                <summary>Quran Sources ({msg.quran_sources.length} ayahs)</summary>
                <div class="source-list">
                  {#each msg.quran_sources as s}
                    <a href="/quran/{s.surah_number}?ayah={s.ayah_number}" class="source-card">
                      <span class="source-ref mono quran-ref">{s.surah_number}:{s.ayah_number}</span>
                      <span class="source-arabic" dir="rtl">{truncate(s.text_ar, 80)}</span>
                      {#if s.text_en}<span class="source-text">{truncate(s.text_en, 120)}</span>{/if}
                    </a>
                  {/each}
                </div>
              </details>
            {/if}

            {#if msg.hadith_sources && msg.hadith_sources.length > 0}
              <details class="sources">
                <summary>Hadith Sources ({msg.hadith_sources.length} hadiths)</summary>
                <div class="source-list">
                  {#each msg.hadith_sources as s}
                    <a href="/hadiths/{s.id}" class="source-card">
                      <span class="source-num mono">#{s.hadith_number}</span>
                      {#if s.narrator_text}<span class="source-narrator">{s.narrator_text}</span>{/if}
                      <span class="source-text">{$language === 'en' && s.text_en ? truncate(stripHtml(s.text_en), 120) : truncate(s.text_ar || stripHtml(s.text_en ?? ''), 120)}</span>
                    </a>
                  {/each}
                </div>
              </details>
            {/if}

            {#if msg.narrator_sources && msg.narrator_sources.length > 0}
              <details class="sources" open>
                <summary>Narrator Sources ({msg.narrator_sources.length})</summary>
                <div class="source-list">
                  {#each msg.narrator_sources as n}
                    <a href="/narrators/{n.id}" class="source-card narrator-card">
                      <div class="narrator-header">
                        <span class="source-narrator">{n.name_en}</span>
                        {#if n.name_ar}<span class="source-arabic" dir="rtl">{n.name_ar}</span>{/if}
                      </div>
                      <div class="narrator-meta">
                        {#if n.generation}<span class="narrator-tag">Gen {n.generation}</span>{/if}
                        {#if n.hadith_count}<span class="narrator-tag">{n.hadith_count} hadiths</span>{/if}
                        {#if n.death_year}<span class="narrator-tag">d. {n.death_year} AH</span>{/if}
                      </div>
                    </a>
                  {/each}
                </div>
              </details>
            {/if}
          {:else}
            <div>{msg.content}</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <form class="input-area" onsubmit={handleSubmit}>
    <input type="text" placeholder={getPlaceholder()} bind:value={input} disabled={loading} class="chat-input" />
    <button type="submit" class="send-btn" disabled={loading || !input.trim()}>{loading ? '...' : 'Send'}</button>
  </form>
</div>

<style>
  .ask-page { display: flex; flex-direction: column; height: 100%; }

  .ask-header {
    padding: 12px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    display: flex;
    align-items: center;
  }
  .mode-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .mode-btn {
    padding: 8px 18px;
    font-size: 0.8rem;
    font-weight: 500;
    background: var(--bg-surface);
    color: var(--text-secondary);
    border: none;
    cursor: pointer;
    transition: all var(--transition);
  }
  .mode-btn.active {
    background: var(--accent);
    color: white;
  }
  .mode-btn:hover:not(.active) {
    background: var(--bg-hover);
  }

  .chat-container { flex: 1; overflow-y: auto; padding: 24px; display: flex; flex-direction: column; gap: 16px; }
  .empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; flex: 1; text-align: center; color: var(--text-secondary); gap: 12px; padding: 40px; }
  .empty-icon { font-size: 2.5rem; color: var(--accent); margin-bottom: 8px; }
  .empty-state h2 { color: var(--text-primary); }
  .empty-state p { max-width: 480px; line-height: 1.6; font-size: 0.9rem; }
  .suggestions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; margin-top: 12px; }
  .suggestion { padding: 8px 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 20px; color: var(--text-secondary); font-size: 0.8rem; transition: all var(--transition); cursor: pointer; }
  .suggestion:hover { border-color: var(--accent); color: var(--accent); }
  .message { max-width: 800px; }
  .message.user { align-self: flex-end; }
  .message-header { margin-bottom: 4px; }
  .role-label { font-size: 0.75rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; font-weight: 600; }
  .message.user .message-content { background: var(--accent-muted); color: var(--text-primary); padding: 12px 16px; border-radius: var(--radius); font-size: 0.9rem; }
  .message.assistant .message-content { background: var(--bg-surface); border: 1px solid var(--border); padding: 16px; border-radius: var(--radius); }
  .assistant-text { font-size: 0.9rem; line-height: 1.7; color: var(--text-primary); }
  .assistant-text :global(p) { margin: 0.5em 0; }
  .assistant-text :global(strong) { font-weight: 700; color: var(--text-primary); }
  .assistant-text :global(em) { font-style: italic; }
  .assistant-text :global(ul), .assistant-text :global(ol) { margin: 0.5em 0; padding-left: 1.5em; }
  .assistant-text :global(li) { margin: 0.25em 0; }
  .assistant-text :global(h1), .assistant-text :global(h2), .assistant-text :global(h3) { margin: 0.75em 0 0.25em; font-weight: 700; }
  .assistant-text :global(h2) { font-size: 1.05rem; }
  .assistant-text :global(h3) { font-size: 0.95rem; }
  .assistant-text :global(code) { background: var(--bg-hover); padding: 2px 5px; border-radius: 3px; font-size: 0.85em; }
  .assistant-text :global(blockquote) { border-left: 3px solid var(--accent); margin: 0.5em 0; padding: 0.25em 0.75em; color: var(--text-secondary); }
  .cursor { animation: blink 1s step-end infinite; color: var(--accent); }
  @keyframes blink { 50% { opacity: 0; } }
  .sources { margin-top: 12px; border-top: 1px solid var(--border); padding-top: 12px; }
  .sources summary { font-size: 0.8rem; color: var(--text-muted); cursor: pointer; }
  .sources summary:hover { color: var(--accent); }
  .source-list { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
  .source-card { display: flex; flex-direction: column; gap: 2px; padding: 10px 12px; background: var(--bg-hover); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 0.8rem; transition: background var(--transition); }
  .source-card:hover { background: var(--bg-active); color: var(--text-primary); }
  .source-ref { font-size: 0.75rem; font-weight: 600; }
  .quran-ref { color: var(--success); }
  .source-num { color: var(--text-muted); font-size: 0.75rem; }
  .source-narrator { color: var(--accent); font-size: 0.8rem; }
  .source-arabic { color: var(--text-primary); font-size: 0.95rem; }
  .source-text { color: var(--text-secondary); }
  .narrator-card { gap: 6px; }
  .narrator-header { display: flex; align-items: baseline; gap: 8px; }
  .narrator-meta { display: flex; flex-wrap: wrap; gap: 4px; }
  .narrator-tag { font-size: 0.7rem; padding: 2px 8px; background: var(--bg-secondary); border-radius: 10px; color: var(--text-muted); white-space: nowrap; }
  .input-area { display: flex; gap: 8px; padding: 16px 24px; border-top: 1px solid var(--border); background: var(--bg-secondary); }
  .chat-input { flex: 1; padding: 12px 16px; font-size: 0.9rem; }
  .send-btn { padding: 12px 24px; background: var(--accent); color: white; border: none; border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; cursor: pointer; transition: background var(--transition); }
  .send-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .send-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  @media (max-width: 640px) {
    .ask-header { padding: 8px 12px; }
    .mode-btn { padding: 6px 12px; font-size: 0.75rem; }
    .chat-container { padding: 16px; }
    .input-area { padding: 12px; }
  }
</style>
