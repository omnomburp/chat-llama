<script>
  export let currentConversation = null;
  export let currentMessages = [];
  export let renderMarkdown = (text) => text ?? '';
  export let loading = false;
</script>

<div id="messages" class="messages">
  {#if !currentConversation || currentMessages.length === 0}
    <div class="empty-state">
      <h1>How can I help you today?</h1>
      <p>Ask anything. Enable web search to pull in fresh context.</p>
    </div>
  {/if}

  {#if currentConversation}
    {#each currentMessages as msg, i (i)}
      {#if msg.role !== 'assistant' || (msg.content && msg.content.trim().length > 0)}
        <div class={`message ${msg.role === 'user' ? 'user' : 'assistant'}`}>
          <div class="bubble">
            <div class="message-content">
              {@html renderMarkdown(msg.content)}
            </div>
          </div>
        </div>
      {/if}
    {/each}

    {#if loading}
      <div class="message assistant">
        <div class="bubble loading-bubble" aria-live="polite">
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 1rem;
    border-radius: 1.1rem;
    background: #1a1b1f;
    border: 1px solid #2a2b31;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    box-shadow: 0 15px 55px rgba(5, 5, 6, 0.55);
  }

  .empty-state {
    margin-top: 10vh;
    text-align: center;
    opacity: 0.85;
  }

  .empty-state h1 {
    font-size: 1.6rem;
    margin-bottom: 0.35rem;
  }

  .message {
    display: flex;
    gap: 0.75rem;
    max-width: 100%;
  }

  .message.user {
    justify-content: flex-end;
  }

  .message.assistant {
    justify-content: flex-start;
  }

  .bubble {
    border-radius: 1rem;
    padding: 0.65rem 0.95rem;
    font-size: 0.95rem;
    line-height: 1.45;
    max-width: min(100%, 680px);
    background: #212226;
    border: 1px solid #2f3036;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.35);
  }

  .message.user .bubble {
    background: #f5f5f6;
    color: #121215;
    border-color: #d1d5db;
  }

  .message-content {
    margin: 0;
    word-wrap: break-word;
    word-break: break-word;
    overflow-wrap: anywhere;
    font-family: inherit;
  }

  .message-content :global(p) {
    margin: 0.15rem 0 0.35rem;
  }

  .message-content :global(p:first-child) {
    margin-top: 0;
  }

  .message-content :global(ul),
  .message-content :global(ol) {
    margin: 0.2rem 0 0.4rem 1.1rem;
    padding-left: 0.7rem;
  }

  .message-content :global(li) {
    margin: 0.15rem 0;
  }

  .message-content :global(blockquote) {
    margin: 0.35rem 0;
    padding-left: 0.7rem;
    border-left: 3px solid #3a3b3f;
    color: #d7d7de;
  }

  .message-content :global(h1),
  .message-content :global(h2),
  .message-content :global(h3),
  .message-content :global(h4),
  .message-content :global(h5) {
    margin: 0.4rem 0 0.25rem;
    line-height: 1.25;
  }

  .message-content :global(h1) {
    font-size: 1.35rem;
  }

  .message-content :global(h2) {
    font-size: 1.2rem;
  }

  .message-content :global(h3) {
    font-size: 1.05rem;
  }

  .message-content :global(a) {
    color: #9ab6ff;
    text-decoration: none;
  }

  .message-content :global(a:hover) {
    text-decoration: underline;
  }

  .message-content :global(.math) {
    font-family: "STIX Two Text", "CMU Serif", Georgia, "Times New Roman", serif;
    background: rgba(255, 255, 255, 0.04);
    padding: 0.02rem 0.15rem;
    border-radius: 0.2rem;
  }

  .message-content :global(.math-block) {
    display: block;
    margin: 0.45rem 0;
    padding: 0.4rem 0.5rem;
    border-radius: 0.35rem;
    background: #1f2024;
    border: 1px solid #34353c;
    white-space: pre-wrap;
  }

  .code-block {
    margin: 0.5rem 0;
    padding: 0.6rem 0.8rem;
    border-radius: 0.6rem;
    background: #111215;
    border: 1px solid #2b2c31;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      "Liberation Mono", "Courier New", monospace;
    font-size: 0.85rem;
    overflow-x: auto;
    white-space: pre;
  }

  :global(.code-block code) {
    display: block;
  }

  .inline-code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      "Liberation Mono", "Courier New", monospace;
    background: rgba(255, 255, 255, 0.08);
    padding: 0.05rem 0.25rem;
    border-radius: 0.25rem;
  }

  .loading-bubble {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    min-width: 70px;
    justify-content: center;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: #fdfdfd;
    animation: bounce 1s infinite ease-in-out;
  }

  .dot:nth-child(2) {
    animation-delay: 0.15s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.3s;
  }

  @keyframes bounce {
    0%, 80%, 100% {
      transform: scale(0);
      opacity: 0.3;
    }
    40% {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
