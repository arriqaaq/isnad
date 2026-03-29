<script lang="ts">
  import type { ApiHadithSearchResult } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    sources?: ApiHadithSearchResult[];
    streaming?: boolean;
  }

  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let chatContainer: HTMLDivElement = $state(null!);

  function scrollToBottom() {
    if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
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
      const res = await fetch('/api/ask', {
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
            if (data.sources) {
              messages[idx] = { ...messages[idx], sources: data.sources };
            } else if (data.text) {
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
  <div class="chat-container" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="empty-state">
        <div class="empty-icon">◆</div>
        <h2>Ask about Sahih al-Bukhari</h2>
        <p>Ask questions about hadiths. Answers are grounded in the actual text using semantic search.</p>
        <div class="suggestions">
          <button class="suggestion" onclick={() => { input = 'What did the Prophet say about kindness to neighbors?'; }}>Kindness to neighbors</button>
          <button class="suggestion" onclick={() => { input = 'What are the hadiths about prayer times?'; }}>Prayer times</button>
          <button class="suggestion" onclick={() => { input = 'What did Abu Huraira narrate about fasting?'; }}>Abu Huraira on fasting</button>
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
            <div class="assistant-text">{msg.content}{#if msg.streaming}<span class="cursor">|</span>{/if}</div>
            {#if msg.sources && msg.sources.length > 0}
              <details class="sources">
                <summary>Sources ({msg.sources.length} hadiths)</summary>
                <div class="source-list">
                  {#each msg.sources as s}
                    <a href="/hadiths/{s.id}" class="source-card">
                      <span class="source-num mono">#{s.hadith_number}</span>
                      {#if s.narrator_text}<span class="source-narrator">{s.narrator_text}</span>{/if}
                      <span class="source-text">{truncate(stripHtml(s.text_en), 120)}</span>
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
    <input type="text" placeholder="Ask about hadiths..." bind:value={input} disabled={loading} class="chat-input" />
    <button type="submit" class="send-btn" disabled={loading || !input.trim()}>{loading ? '...' : 'Send'}</button>
  </form>
</div>

<style>
  .ask-page { display: flex; flex-direction: column; height: 100%; }
  .chat-container { flex: 1; overflow-y: auto; padding: 24px; display: flex; flex-direction: column; gap: 16px; }
  .empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; flex: 1; text-align: center; color: var(--text-secondary); gap: 12px; padding: 40px; }
  .empty-icon { font-size: 2.5rem; color: var(--accent); margin-bottom: 8px; }
  .empty-state h2 { color: var(--text-primary); }
  .empty-state p { max-width: 480px; line-height: 1.6; font-size: 0.9rem; }
  .suggestions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; margin-top: 12px; }
  .suggestion { padding: 8px 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 20px; color: var(--text-secondary); font-size: 0.8rem; transition: all var(--transition); }
  .suggestion:hover { border-color: var(--accent); color: var(--accent); }
  .message { max-width: 800px; }
  .message.user { align-self: flex-end; }
  .message-header { margin-bottom: 4px; }
  .role-label { font-size: 0.75rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; font-weight: 600; }
  .message.user .message-content { background: var(--accent-muted); color: var(--text-primary); padding: 12px 16px; border-radius: var(--radius); font-size: 0.9rem; }
  .message.assistant .message-content { background: var(--bg-surface); border: 1px solid var(--border); padding: 16px; border-radius: var(--radius); }
  .assistant-text { font-size: 0.9rem; line-height: 1.7; white-space: pre-wrap; color: var(--text-primary); }
  .cursor { animation: blink 1s step-end infinite; color: var(--accent); }
  @keyframes blink { 50% { opacity: 0; } }
  .sources { margin-top: 12px; border-top: 1px solid var(--border); padding-top: 12px; }
  .sources summary { font-size: 0.8rem; color: var(--text-muted); cursor: pointer; }
  .sources summary:hover { color: var(--accent); }
  .source-list { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
  .source-card { display: flex; flex-direction: column; gap: 2px; padding: 10px 12px; background: var(--bg-hover); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 0.8rem; transition: background var(--transition); }
  .source-card:hover { background: var(--bg-active); color: var(--text-primary); }
  .source-num { color: var(--text-muted); font-size: 0.75rem; }
  .source-narrator { color: var(--accent); font-size: 0.8rem; }
  .source-text { color: var(--text-secondary); }
  .input-area { display: flex; gap: 8px; padding: 16px 24px; border-top: 1px solid var(--border); background: var(--bg-secondary); }
  .chat-input { flex: 1; padding: 12px 16px; font-size: 0.9rem; }
  .send-btn { padding: 12px 24px; background: var(--accent); color: var(--bg-primary); border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; transition: background var(--transition); }
  .send-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .send-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
