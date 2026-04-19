<script lang="ts">
  import { marked } from 'marked';

  marked.setOptions({ breaks: true, gfm: true });

  let { bookId, bookName, currentPageIndex, onNavigate, defaultQuestions = [] }: {
    bookId: number;
    bookName: string;
    currentPageIndex: number;
    onNavigate: (pageIndex: number) => void;
    defaultQuestions?: string[];
  } = $props();

  interface BookChatSource {
    volume: number;
    page: number;
    heading: string | null;
  }

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    sources?: BookChatSource[];
    status?: string;
    warning?: string;
    streaming?: boolean;
  }

  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let chatContainer: HTMLDivElement = $state(null!);

  // Load chat history from localStorage. Derived so a book switch in the
  // enclosing viewer (e.g. BookViewerModal's tafsir dropdown) re-targets
  // the correct slot instead of writing back to the initial book's key.
  const storageKey = $derived(`book_chat_${bookId}`);

  function loadHistory() {
    if (typeof localStorage === 'undefined') return;
    try {
      const saved = localStorage.getItem(storageKey);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed)) messages = parsed;
      }
    } catch { /* ignore */ }
  }

  function saveHistory() {
    if (typeof localStorage === 'undefined') return;
    try {
      // Keep last 20 messages
      const toSave = messages.filter(m => !m.streaming).slice(-20);
      localStorage.setItem(storageKey, JSON.stringify(toSave));
    } catch { /* ignore */ }
  }

  $effect(() => {
    // Reset when bookId changes
    messages = [];
    loadHistory();
  });

  function scrollToBottom() {
    if (chatContainer) {
      requestAnimationFrame(() => {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      });
    }
  }

  function clearChat() {
    messages = [];
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem(storageKey);
    }
  }

  /**
   * Replace [Vol.X/p.Y] citations in text with clickable spans.
   */
  function processCitations(html: string, sources?: BookChatSource[]): string {
    return html.replace(/\[Vol\.(\d+)\/p\.(\d+)\]/g, (match, vol, page) => {
      return `<button class="citation-btn" data-vol="${vol}" data-page="${page}" title="Go to Vol.${vol} page ${page}">${match}</button>`;
    });
  }

  function handleCitationClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.classList.contains('citation-btn')) {
      const page = parseInt(target.dataset.page || '0');
      if (page > 0) {
        onNavigate(page);
      }
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

    try {
      const res = await fetch(`/api/books/${bookId}/chat`, {
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
            if (data.status === 'no_relevant_sections') {
              messages[idx] = { ...messages[idx], warning: data.message || 'No relevant sections found.' };
            } else if (data.status) {
              messages[idx] = { ...messages[idx], status: data.status };
            } else if (data.sources) {
              messages[idx] = { ...messages[idx], sources: data.sources };
            } else if (data.text) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + data.text };
              scrollToBottom();
            } else if (data.done) {
              messages[idx] = { ...messages[idx], streaming: false, status: undefined };
            } else if (data.error) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + `\n\n[Error: ${data.error}]`, streaming: false };
            }
          } catch { /* skip */ }
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
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  const fallbackQuestions = ['ما هي المواضيع التي يتناولها هذا الكتاب؟'];
  const suggestions = $derived(
    defaultQuestions.length > 0 ? defaultQuestions : fallbackQuestions
  );
</script>

<div class="book-chat">
  <!-- Header -->
  <div class="chat-header">
    <span class="chat-title">Ask AI</span>
    {#if messages.length > 0}
      <button class="clear-btn" onclick={clearChat} title="New chat">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 5v14M5 12h14"/>
        </svg>
      </button>
    {/if}
  </div>

  <!-- Messages -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="chat-messages" bind:this={chatContainer} onclick={handleCitationClick}>
    {#if messages.length === 0}
      <div class="empty-state">
        <p class="empty-label">اسأل عن {bookName}</p>
        <p class="empty-hint">البحث متاح باللغة العربية فقط</p>
        <div class="suggestions">
          {#each suggestions as s}
            <button class="suggestion-chip" onclick={() => { input = s; handleSubmit(); }}>
              {s}
            </button>
          {/each}
        </div>
      </div>
    {:else}
      {#each messages as msg}
        <div class="message {msg.role}">
          {#if msg.role === 'assistant'}
            {#if msg.status}
              <div class="status-indicator">
                <span class="status-dot"></span>
                {msg.status === 'navigating' ? 'Navigating table of contents...' :
                 msg.status === 'reading' ? 'Reading relevant pages...' :
                 msg.status}
              </div>
            {/if}
            {#if msg.warning}
              <div class="warning-banner">⚠ {msg.warning}</div>
            {/if}
            {#if msg.content}
              <div class="msg-text">
                {@html processCitations(marked(msg.content) as string, msg.sources)}
              </div>
            {/if}
            {#if msg.streaming && !msg.content}
              <span class="cursor">|</span>
            {/if}
            {#if msg.sources && msg.sources.length > 0 && !msg.streaming}
              <div class="sources-bar">
                {#each msg.sources as src}
                  <button
                    class="source-badge"
                    onclick={() => onNavigate(src.page)}
                    title={src.heading || `Vol.${src.volume} p.${src.page}`}
                  >
                    Vol.{src.volume}/p.{src.page}
                  </button>
                {/each}
              </div>
            {/if}
          {:else}
            <div class="msg-text">{msg.content}</div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Input -->
  <form class="chat-input-area" onsubmit={handleSubmit}>
    <textarea
      bind:value={input}
      placeholder="...اسأل عن هذا الكتاب"
      dir="rtl"
      disabled={loading}
      rows="1"
      onkeydown={handleKeydown}
    ></textarea>
    <button type="submit" class="send-btn" aria-label="Send" disabled={loading || !input.trim()}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="22" y1="2" x2="11" y2="13"/>
        <polygon points="22 2 15 22 11 13 2 9 22 2"/>
      </svg>
    </button>
  </form>
</div>

<style>
  .book-chat {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .chat-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .clear-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition);
  }
  .clear-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    padding: 20px;
  }

  .empty-label {
    color: var(--text-muted);
    font-size: 14px;
    text-align: center;
    direction: rtl;
  }

  .empty-hint {
    color: var(--text-muted);
    font-size: 12px;
    text-align: center;
    opacity: 0.7;
    direction: rtl;
    margin: -8px 0 0;
  }

  .suggestions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .suggestion-chip {
    padding: 8px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    text-align: right;
    direction: rtl;
    transition: all var(--transition);
  }
  .suggestion-chip:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }

  .message {
    max-width: 100%;
  }

  .message.user {
    align-self: flex-end;
    background: var(--accent-muted, rgba(59, 130, 246, 0.1));
    border-radius: 12px 12px 4px 12px;
    padding: 8px 12px;
  }

  .message.assistant {
    align-self: flex-start;
  }

  .msg-text {
    font-size: 15px;
    line-height: 1.7;
    color: var(--text-primary);
    word-break: break-word;
  }

  .msg-text :global(p) {
    margin: 0 0 8px;
  }
  .msg-text :global(p:last-child) {
    margin-bottom: 0;
  }

  :global(.citation-btn) {
    display: inline;
    padding: 1px 4px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: var(--accent-muted, rgba(59, 130, 246, 0.1));
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition);
  }
  :global(.citation-btn:hover) {
    background: var(--accent);
    color: white;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    padding: 4px 0;
  }

  .warning-banner {
    padding: 8px 10px;
    margin: 8px 0;
    border: 1px solid var(--border);
    border-left: 3px solid #f59e0b;
    border-radius: 4px;
    background: rgba(245, 158, 11, 0.05);
    color: var(--text-secondary);
    font-size: 12px;
    direction: rtl;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }

  .cursor {
    animation: blink 0.8s infinite;
    color: var(--text-muted);
  }
  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .sources-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }

  .source-badge {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    transition: all var(--transition);
  }
  .source-badge:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .chat-input-area {
    display: flex;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--border-subtle);
    flex-shrink: 0;
    align-items: flex-end;
  }

  .chat-input-area textarea {
    flex: 1;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    resize: none;
    min-height: 36px;
    max-height: 100px;
    outline: none;
    transition: border-color var(--transition);
  }
  .chat-input-area textarea:focus {
    border-color: var(--accent);
  }
  .chat-input-area textarea:disabled {
    opacity: 0.5;
  }

  .send-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--accent);
    cursor: pointer;
    flex-shrink: 0;
    transition: all var(--transition);
  }
  .send-btn:hover:not(:disabled) {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .send-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }
</style>
