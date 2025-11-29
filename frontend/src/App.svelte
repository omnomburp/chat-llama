<script>
  import { tick, onMount } from 'svelte';
  import { marked } from 'marked';
  import katex from 'katex';
  import 'katex/dist/katex.min.css';
  import Sidebar from './components/Sidebar.svelte';
  import ChatMessages from './components/ChatMessages.svelte';
  import Composer from './components/Composer.svelte';

  const STORAGE_KEY = 'llama-chat-conversations';
  const defaultUseSearch = true;
  const SIDEBAR_BREAKPOINT = 900;

  let conversations = [];
  let currentId = null;
  let input = '';
  let loading = false;
  let showLoadingBubble = false;
  let controller; // AbortController for streaming
  let sidebarOpen = false;
  let toolMenuOpen = false;
  let sidebarCollapsed = false;

  // Derived state
  $: currentConversation =
    conversations.find((c) => c.id === currentId) || null;
  $: currentMessages = currentConversation?.messages
    ? [...currentConversation.messages]
    : [];
  $: currentSources = currentConversation?.sources
    ? [...currentConversation.sources]
    : [];
  $: currentUseSearch =
    currentConversation?.useSearch ?? defaultUseSearch;

  function escapeHtml(str) {
    if (str === undefined || str === null) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  function maybeCloseSidebar() {
    if (typeof window === 'undefined') return;
    if (window.innerWidth < SIDEBAR_BREAKPOINT) {
      sidebarOpen = false;
    }
  }

  function sanitizeUrl(url) {
    if (!url) return '#';
    try {
      const parsed = new URL(url, typeof window !== 'undefined' ? window.location.origin : 'http://localhost');
      const protocol = parsed.protocol.toLowerCase();
      if (protocol === 'javascript:' || protocol === 'data:') return '#';
      return parsed.href;
    } catch (e) {
      return '#';
    }
  }

  const renderer = new marked.Renderer();
  renderer.code = function ({ text = '', lang = '' }) {
    const language = typeof lang === 'string' ? lang.trim() : '';
    const cls = language ? `language-${language}` : '';
    const safeCode =
      typeof text === 'string' ? text : text !== undefined && text !== null ? String(text) : '';
    return `<pre class="code-block"><code class="${cls}">${escapeHtml(
      safeCode.trimEnd()
    )}</code></pre>`;
  };
  renderer.codespan = function ({ text = '' }) {
    const code = typeof text === 'string' ? text : text !== undefined && text !== null ? String(text) : '';
    return `<code class="inline-code">${escapeHtml(code)}</code>`;
  };
  renderer.link = function ({ href, title, tokens }) {
    const safeHref = sanitizeUrl(href);
    const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
    const inner =
      (this.parser && this.parser.parseInline(tokens ?? [])) ||
      escapeHtml(typeof href === 'string' ? href : String(href ?? ''));
    return `<a href="${safeHref}" target="_blank" rel="noreferrer"${titleAttr}>${inner}</a>`;
  };
  renderer.html = function ({ text = '' }) {
    const html = typeof text === 'string' ? text : text !== undefined && text !== null ? String(text) : '';
    return escapeHtml(html);
  };

  function renderMath(text, displayMode = false) {
    const safe = typeof text === 'string' ? text : String(text ?? '');
    try {
      return katex.renderToString(safe, {
        displayMode,
        throwOnError: false,
        output: 'html',
        strict: 'ignore'
      });
    } catch (e) {
      console.error('Failed to render math', e, text);
      const tag = displayMode ? 'div' : 'span';
      const cls = displayMode ? 'math math-block' : 'math';
      return `<${tag} class="${cls}">${escapeHtml(safe)}</${tag}>`;
    }
  }

  const mathExtensions = [
    {
      name: 'mathBlock',
      level: 'block',
      start(src) {
        const dollar = src.indexOf('$$');
        const bracket = src.indexOf('\\[');
        if (dollar === -1) return bracket === -1 ? undefined : bracket;
        if (bracket === -1) return dollar;
        return Math.min(dollar, bracket);
      },
      tokenizer(src) {
        const dollarMatch = src.match(/^\$\$([^]*?)\$\$/);
        if (dollarMatch) {
          return {
            type: 'mathBlock',
            raw: dollarMatch[0],
            text: dollarMatch[1]
          };
        }
        const bracketMatch = src.match(/^\\\[([^]*?)\\\]/);
        if (bracketMatch) {
          return {
            type: 'mathBlock',
            raw: bracketMatch[0],
            text: bracketMatch[1]
          };
        }
      },
      renderer(token) {
        return renderMath(token.text.trim(), true);
      }
    },
    {
      name: 'mathInline',
      level: 'inline',
      start(src) {
        const dollar = src.indexOf('$');
        const paren = src.indexOf('\\(');
        if (dollar === -1) return paren === -1 ? undefined : paren;
        if (paren === -1) return dollar;
        return Math.min(dollar, paren);
      },
      tokenizer(src) {
        const dollarMatch = src.match(/^\$([^\n$]+?)\$/);
        if (dollarMatch) {
          return {
            type: 'mathInline',
            raw: dollarMatch[0],
            text: dollarMatch[1]
          };
        }
        const parenMatch = src.match(/^\\\((.+?)\\\)/);
        if (parenMatch) {
          return {
            type: 'mathInline',
            raw: parenMatch[0],
            text: parenMatch[1]
          };
        }
      },
      renderer(token) {
        return renderMath(token.text, false);
      }
    }
  ];

  marked.use({
    renderer,
    gfm: true,
    breaks: true,
    headerIds: false,
    mangle: false,
    extensions: mathExtensions
  });

  function renderMarkdown(text) {
    if (!text) return '';
    try {
      const rendered = marked.parse(text);
      if (rendered instanceof Promise) {
        console.warn('Unexpected async markdown render; falling back to plain text.');
        return escapeHtml(text).replace(/\n/g, '<br>');
      }
      return rendered;
    } catch (e) {
      console.error('Failed to render markdown', e, text);
      return escapeHtml(text).replace(/\n/g, '<br>');
    }
  }

  function saveConversations() {
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(conversations));
      }
    } catch (e) {
      console.error('Failed to save conversations', e);
    }
  }

  function createNewConversation() {
    const id = Date.now().toString();
    const conv = {
      id,
      title: 'New chat',
      messages: [],
      sources: [],
      useSearch: defaultUseSearch,
      createdAt: Date.now(),
      updatedAt: Date.now()
    };
    conversations = [conv, ...conversations];
    currentId = id;
    maybeCloseSidebar();
    saveConversations();
    return conv;
  }

  function selectConversation(id) {
    currentId = id;
    maybeCloseSidebar();
  }

  function deleteConversation(id) {
    const idx = conversations.findIndex((c) => c.id === id);
    if (idx === -1) return;

    // Remove the conversation
    conversations = conversations.filter((c) => c.id !== id);

    // If we deleted the current one, pick a new current
    if (currentId === id) {
      if (conversations.length === 0) {
        // No chats left: create a fresh one
        createNewConversation();
      } else {
        // Choose the next one in the list or the last one
        const newIdx = idx < conversations.length ? idx : conversations.length - 1;
        currentId = conversations[newIdx].id;
      }
    }

    saveConversations();
  }

  function toggleUseSearch(checked) {
    if (!currentConversation) return;
    currentConversation.useSearch = checked;
    currentConversation.updatedAt = Date.now();
    conversations = [...conversations];
    saveConversations();
  }

  onMount(() => {
    const handleResize = () => {
      if (typeof window === 'undefined') return;
      if (window.innerWidth >= SIDEBAR_BREAKPOINT) {
        sidebarOpen = false;
      } else {
        sidebarCollapsed = false;
      }
    };

    if (typeof window !== 'undefined') {
      handleResize();
      window.addEventListener('resize', handleResize);
    }

    try {
      if (typeof localStorage === 'undefined') {
        createNewConversation();
        return () => {
          if (typeof window !== 'undefined') {
            window.removeEventListener('resize', handleResize);
          }
        };
      }
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) {
          conversations = parsed;
          currentId = parsed[0].id;
          return () => {
            if (typeof window !== 'undefined') {
              window.removeEventListener('resize', handleResize);
            }
          };
        }
      }
    } catch (e) {
      console.error('Failed to load conversations', e);
    }
    createNewConversation();

    return () => {
      if (typeof window !== 'undefined') {
        window.removeEventListener('resize', handleResize);
      }
    };
  });

  async function sendMessage() {
    if (!input.trim() || loading) return;
    toolMenuOpen = false;

    let conv = currentConversation;
    if (!conv) {
      conv = createNewConversation();
    }

    const userMsg = {
      role: 'user',
      content: input.trim()
    };

    // Update conversation with user message
    conv.messages = [...conv.messages, userMsg];
    conv.updatedAt = Date.now();
    if (!conv.title || conv.title === 'New chat') {
      conv.title = userMsg.content.slice(0, 40);
    }

    const historyForRequest = conv.messages.slice(); // includes new user msg
    input = '';

    // Placeholder assistant message to stream into
    const assistantMsg = {
      role: 'assistant',
      content: ''
    };
    conv.messages = [...conv.messages, assistantMsg];
    const assistantIndex = conv.messages.length - 1;

    conversations = [...conversations];
    saveConversations();

    loading = true;
    showLoadingBubble = true;
    // Reset sources for this turn; server will send fresh ones
    conv.sources = [];
    conversations = [...conversations];
    saveConversations();

    // Abort previous stream if any
    if (controller) controller.abort();
    controller = new AbortController();

    try {
      const res = await fetch('/api/chat/stream', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: userMsg.content,
          use_search: conv.useSearch,
          history: historyForRequest
        }),
        signal: controller.signal
      });

      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }

      const reader = res.body.getReader();
      const decoder = new TextDecoder('utf-8');
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // SSE events are separated by blank lines (\n\n)
        const events = buffer.split('\n\n');
        buffer = events.pop() ?? '';

        for (const evt of events) {
          if (!evt.trim()) continue;

          // Parse SSE: lines like "event: foo" and "data: {...}"
          const lines = evt
            .split('\n')
            .map((l) => l.trim())
            .filter(Boolean);

          let eventName = 'message';
          let dataBuf = '';

          for (const line of lines) {
            if (line.startsWith('event:')) {
              eventName = line.slice(6).trim();
            } else if (line.startsWith('data:')) {
              const d = line.slice(5).trim();
              if (dataBuf) dataBuf += '\n';
              dataBuf += d;
            }
          }

          if (!dataBuf) continue;
          if (dataBuf === '[DONE]') {
            continue;
          }

          // Our custom "sources" event from the server
          if (eventName === 'sources') {
            try {
              const parsed = JSON.parse(dataBuf);
              if (currentConversation && currentConversation.id === conv.id) {
                currentConversation.sources = parsed;
                conversations = [...conversations];
                saveConversations();
              }
            } catch (e) {
              console.error('Failed to parse sources JSON', e, dataBuf);
            }
            continue;
          }

          // Normal chat chunk from llama-server (OpenAI-style streaming)
          try {
            const json = JSON.parse(dataBuf);
            const delta = json.choices?.[0]?.delta?.content ?? '';
            if (delta && currentConversation && currentConversation.id === conv.id) {
              currentConversation.messages[assistantIndex].content += delta;
              if (showLoadingBubble) {
                showLoadingBubble = false;
              }
              currentConversation.updatedAt = Date.now();
              conversations = [...conversations];
              saveConversations();
              await tick();
              scrollToBottom();
            }
          } catch (e) {
            console.error('Failed to parse stream JSON', e, dataBuf);
          }
        }
      }
    } catch (e) {
      console.error(e);
      if (currentConversation && currentConversation.id === conv.id) {
        currentConversation.messages[assistantIndex].content =
          currentConversation.messages[assistantIndex].content ||
          'Sorry, something went wrong while streaming from the model.';
        conversations = [...conversations];
        saveConversations();
      }
    } finally {
      loading = false;
      showLoadingBubble = false;
      await tick();
      scrollToBottom();
    }
  }

  function scrollToBottom() {
    const container = document.getElementById('messages');
    if (container) container.scrollTop = container.scrollHeight;
  }

  function stopStreaming() {
    if (controller) controller.abort();
  }

  function handleWindowClick(event) {
    if (!toolMenuOpen) return;
    const target = event.target;
    if (target && typeof target.closest === 'function') {
      if (target.closest('.tool-menu')) {
        return;
      }
    }
    toolMenuOpen = false;
  }
</script>

<svelte:window on:click={handleWindowClick} />

<main class={`app ${sidebarOpen ? 'sidebar-open' : ''}`}>
  <header class="top-bar">
    <div class="top-left">
      <button
        class="sidebar-toggle"
        type="button"
        aria-label="Open chat list"
        on:click={() => (sidebarOpen = true)}
      >
        ☰
      </button>
      <button
        class="sidebar-collapse-trigger"
        type="button"
        aria-label={sidebarCollapsed ? 'Show chat list' : 'Hide chat list'}
        on:click={() => {
          sidebarCollapsed = !sidebarCollapsed;
          sidebarOpen = false;
        }}
      >
        {sidebarCollapsed ? 'Show chats' : 'Hide chats'}
      </button>
      <div class="logo">Llama Chat</div>
    </div>
    <div class="top-controls">
      {#if loading}
        <button class="stop-btn" type="button" on:click={stopStreaming}>
          Stop
        </button>
      {/if}
    </div>
  </header>

  <div class="app-body">
    <Sidebar
      {conversations}
      {currentId}
      {sidebarOpen}
      {sidebarCollapsed}
      on:create={() => {
        createNewConversation();
        maybeCloseSidebar();
      }}
      on:select={(event) => selectConversation(event.detail.id)}
      on:delete={(event) => deleteConversation(event.detail.id)}
      on:close={() => (sidebarOpen = false)}
    />

    <section class="chat-area">
      <div class="chat-wrapper">
        <ChatMessages
          {currentConversation}
          {currentMessages}
          {renderMarkdown}
          loading={loading && showLoadingBubble}
        />

        <Composer
          inputValue={input}
          {loading}
          currentUseSearch={currentUseSearch}
          toolMenuOpen={toolMenuOpen}
          on:input={(event) => (input = event.detail)}
          on:send={sendMessage}
          on:toggleSearch={(event) => toggleUseSearch(event.detail)}
          on:toggleMenu={(event) => (toolMenuOpen = event.detail)}
        />
      </div>
    </section>
  </div>

  {#if sidebarOpen}
    <div
      class="sidebar-backdrop"
      role="button"
      tabindex="0"
      aria-label="Close chat list"
      on:click={() => (sidebarOpen = false)}
      on:keydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          sidebarOpen = false;
        }
      }}
    ></div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    background: #111214;
    color: #f7f7f8;
    font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'SF Pro Text',
      sans-serif;
  }

  .app {
    min-height: 100vh;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #111214;
    position: relative;
    overflow: hidden;
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.25rem;
    border-bottom: 1px solid #2b2c31;
    background: #151518;
    position: sticky;
    top: 0;
    z-index: 50;
  }

  .top-left {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .sidebar-toggle {
    display: none;
    border: none;
    background: #1f1f23;
    color: #e3e3e9;
    border-radius: 0.6rem;
    width: 36px;
    height: 36px;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .sidebar-collapse-trigger {
    border: none;
    background: #1f1f23;
    color: #c9cad3;
    border-radius: 0.6rem;
    padding: 0.3rem 0.8rem;
    cursor: pointer;
    font-size: 0.85rem;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }

  .logo {
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .top-controls {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    font-size: 0.85rem;
    color: #c9cad3;
  }

  .stop-btn {
    border-radius: 999px;
    border: none;
    padding: 0.25rem 0.9rem;
    font-size: 0.8rem;
    cursor: pointer;
    background: #ef4444;
    color: #fff;
  }

  .app-body {
    flex: 1;
    display: flex;
    min-height: 0;
    position: relative;
    overflow: hidden;
  }

  .chat-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: radial-gradient(circle at 20% 0%, rgba(255, 255, 255, 0.07), transparent 35%),
      #111214;
    overflow: hidden;
  }

  .chat-wrapper {
    flex: 1;
    width: 100%;
    max-width: 960px;
    margin: 0 auto;
    padding: 1rem 1.5rem;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(10, 10, 11, 0.75);
    z-index: 30;
  }

  @media (max-width: 900px) {
    .sidebar-toggle {
      display: inline-flex;
    }

    .sidebar-collapse-trigger {
      display: none;
    }

    .chat-wrapper {
      padding: 0.75rem;
    }
  }

  @media (min-width: 901px) {
    .sidebar-backdrop {
      display: none;
    }
  }

  @media (max-width: 640px) {
    .top-bar {
      padding: 0.75rem;
    }

    .top-controls {
      gap: 0.5rem;
    }
  }
</style>
