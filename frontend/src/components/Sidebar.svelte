<script>
  import { createEventDispatcher } from 'svelte';

  export let conversations = [];
  export let currentId = null;
  export let sidebarCollapsed = false;
  export let sidebarOpen = false;

  const dispatch = createEventDispatcher();

  function getConversationSubtitle(conv) {
    if (!conv || !Array.isArray(conv.messages) || conv.messages.length === 0) {
      return 'No messages yet';
    }
    const last = conv.messages[conv.messages.length - 1];
    const snippet = (last?.content || '').replace(/\s+/g, ' ').trim();
    return snippet ? (snippet.length > 50 ? `${snippet.slice(0, 50)}…` : snippet) : 'No messages yet';
  }
</script>

<aside class={`sidebar ${sidebarOpen ? 'open' : ''} ${sidebarCollapsed ? 'collapsed' : ''}`}>
  <div class="sidebar-header">
    <span>Chats</span>
    <button
      class="sidebar-close"
      type="button"
      aria-label="Close chat list"
      on:click={() => dispatch('close')}
    >
      ✕
    </button>
  </div>
  <button class="new-chat-btn" type="button" on:click={() => dispatch('create')}>
    + New chat
  </button>

  {#if conversations.length > 0}
    <div class="conversation-list">
      {#each conversations as conv}
        <div class={`conversation-item ${conv.id === currentId ? 'active' : ''}`}>
          <button
            type="button"
            class="conversation-select"
            on:click={() => dispatch('select', { id: conv.id })}
          >
            <span class="conversation-title">
              {conv.title || 'New chat'}
            </span>
            <span class="conversation-subtitle">
              {getConversationSubtitle(conv)}
            </span>
          </button>
          <button
            type="button"
            class="conversation-delete"
            aria-label="Delete chat"
            on:click={() => dispatch('delete', { id: conv.id })}
          >
            ✕
          </button>
        </div>
      {/each}
    </div>
  {:else}
    <p class="conversation-empty">No chats yet.</p>
  {/if}
</aside>

<style>
  .sidebar {
    width: 280px;
    background: #18181b;
    border-right: 1px solid #2b2b32;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    transition: transform 0.25s ease, box-shadow 0.25s ease, width 0.25s ease, padding 0.25s;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-weight: 600;
    font-size: 0.95rem;
    color: #e5e5ef;
  }

  .sidebar-close {
    border: none;
    background: transparent;
    color: #a5a7b7;
    font-size: 1.2rem;
    cursor: pointer;
    border-radius: 0.4rem;
    padding: 0.1rem 0.25rem;
  }

  .new-chat-btn {
    width: 100%;
    border-radius: 0.9rem;
    border: 1px dashed #34343c;
    background: #1f1f23;
    color: #f2f2ff;
    padding: 0.45rem 0.8rem;
    font-size: 0.88rem;
    cursor: pointer;
    text-align: center;
  }

  .new-chat-btn:hover {
    background: #27272f;
  }

  .conversation-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding-right: 0.3rem;
  }

  .conversation-item {
    display: flex;
    align-items: flex-start;
    gap: 0.35rem;
    padding: 0.45rem 0.45rem;
    border-radius: 0.9rem;
    border: 1px solid transparent;
  }

  .conversation-item.active {
    background: #202025;
    border-color: rgba(148, 163, 255, 0.35);
  }

  .conversation-select {
    flex: 1;
    border: none;
    background: transparent;
    text-align: left;
    color: inherit;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0;
  }

  .conversation-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: #f9f9ff;
  }

  .conversation-subtitle {
    font-size: 0.78rem;
    color: #9fa1b1;
  }

  .conversation-delete {
    border: none;
    background: rgba(255, 255, 255, 0.05);
    color: #9fa1b1;
    border-radius: 0.5rem;
    padding: 0.15rem 0.35rem;
    cursor: pointer;
  }

  .conversation-delete:hover {
    background: rgba(239, 68, 68, 0.15);
    color: #fecaca;
  }

  .conversation-empty {
    margin: 0;
    color: #9fa1b1;
    font-size: 0.85rem;
  }

  @media (max-width: 900px) {
    .sidebar {
      position: fixed;
      top: 0;
      bottom: 0;
      left: 0;
      transform: translateX(-100%);
      z-index: 40;
      width: 78vw;
      max-width: 320px;
      box-shadow: 25px 0 50px rgba(2, 6, 23, 0.7);
    }

    .sidebar.open {
      transform: translateX(0);
    }
  }

  @media (min-width: 901px) {
    .sidebar.open {
      transform: none;
    }

    .sidebar.collapsed {
      width: 0;
      padding: 0;
      border: none;
      margin: 0;
    }

    .sidebar.collapsed * {
      opacity: 0;
      pointer-events: none;
    }
  }
</style>
