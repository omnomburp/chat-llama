<script>
  import { createEventDispatcher } from 'svelte';

  export let inputValue = '';
  export let loading = false;
  export let currentUseSearch = true;
  export let toolMenuOpen = false;
  export let hasBottomInset = false;

  const dispatch = createEventDispatcher();

  function handleInput(e) {
    dispatch('input', e.target.value);
  }
</script>

<form
  class={`input-bar ${hasBottomInset ? 'with-inset' : ''}`}
  on:submit|preventDefault={() => dispatch('send')}
>
  <div class="input-inner">
    <div class="tool-menu">
      <button
        type="button"
        class={`tool-trigger ${toolMenuOpen ? 'open' : ''}`}
        aria-haspopup="true"
        aria-expanded={toolMenuOpen}
        on:click={() => dispatch('toggleMenu', !toolMenuOpen)}
      >
        +
      </button>
      <div class={`tool-dropdown ${toolMenuOpen ? 'open' : ''}`}>
        <button
          type="button"
          class={`tool-item ${currentUseSearch ? 'selected' : ''}`}
          on:click={() => {
            dispatch('toggleSearch', !currentUseSearch);
            dispatch('toggleMenu', false);
          }}
        >
          <span>Web search</span>
          {#if currentUseSearch}
            <span class="tool-check">✓</span>
          {/if}
        </button>
        <div class="tool-item disabled" aria-disabled="true">
          <span>Add files</span>
        </div>
        <div class="tool-item disabled" aria-disabled="true">
          <span>Add images</span>
        </div>
      </div>
    </div>

    {#if currentUseSearch}
      <div class="tool-pill" title="Web search enabled">
        <span class="pill-icon">🌐</span>
        <span>Search</span>
        <button
          type="button"
          class="pill-remove"
          aria-label="Disable web search"
          on:click={() => dispatch('toggleSearch', false)}
        >
          ✕
        </button>
      </div>
    {/if}

    <textarea
      rows="1"
      bind:value={inputValue}
      placeholder="Send a message..."
      on:input={handleInput}
      on:keydown={(e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          dispatch('send');
        }
      }}
    />
    <button class="send-button" type="submit" disabled={loading || !inputValue.trim()}>
      {#if loading}
        ...
      {:else}
        ➤
      {/if}
    </button>
  </div>
  <div class="input-hints"></div>
</form>

<style>
  .input-bar {
    margin-top: 0.9rem;
    position: sticky;
    bottom: 0;
    padding-bottom: env(safe-area-inset-bottom, 0.3rem);
    background: linear-gradient(to top, #111214 60%, rgba(17, 18, 20, 0));
  }

  .input-bar.with-inset {
    padding-bottom: calc(env(safe-area-inset-bottom, 0.3rem) + 1.5rem);
  }

  .input-inner {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    border-radius: 1.1rem;
    padding: 0.6rem 0.85rem;
    border: 1px solid #2a2b31;
    background: #18191d;
  }

  .input-inner textarea {
    flex: 1;
    resize: none;
    border: none;
    background: transparent;
    color: inherit;
    font-family: inherit;
    font-size: 0.95rem;
    max-height: 120px;
    outline: none;
  }

  .send-button {
    border-radius: 999px;
    border: none;
    width: 34px;
    height: 34px;
    cursor: pointer;
    background: #f5f5f6;
    color: #151515;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .send-button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .tool-menu {
    position: relative;
  }

  .tool-trigger {
    width: 36px;
    height: 36px;
    border-radius: 999px;
    border: 1px solid #2a2b31;
    background: rgba(31, 31, 35, 0.85);
    color: #e7e8ef;
    font-size: 1rem;
    cursor: pointer;
    backdrop-filter: blur(6px);
    transition: background 0.2s ease, color 0.2s ease, border-color 0.2s ease;
  }

  .tool-trigger.open {
    background: rgba(154, 182, 255, 0.2);
    border-color: rgba(154, 182, 255, 0.5);
    color: #c8d4ff;
  }

  .tool-dropdown {
    position: absolute;
    bottom: 110%;
    left: 0;
    display: flex;
    flex-direction: column;
    background: rgba(24, 25, 29, 0.98);
    border: 1px solid #2b2b32;
    border-radius: 0.9rem;
    padding: 0.5rem;
    box-shadow: 0 15px 30px rgba(5, 5, 6, 0.6);
    min-width: 200px;
    gap: 0.4rem;
    opacity: 0;
    pointer-events: none;
    transform: translateY(6px);
    transition: opacity 0.15s ease, transform 0.15s ease;
    z-index: 5;
  }

  .tool-dropdown.open {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(0);
  }

  .tool-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.3rem 0.65rem;
    border-radius: 0.65rem;
    border: none;
    background: transparent;
    color: #f2f2f8;
    cursor: pointer;
    font-size: 0.88rem;
  }

  .tool-item:hover:not(.disabled) {
    background: rgba(154, 182, 255, 0.15);
  }

  .tool-item.selected {
    background: rgba(154, 182, 255, 0.25);
    color: #101015;
  }

  .tool-item.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .tool-check {
    font-size: 0.8rem;
  }

  .tool-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border-radius: 999px;
    background: rgba(154, 182, 255, 0.12);
    border: 1px solid rgba(154, 182, 255, 0.3);
    padding: 0.2rem 0.6rem;
    font-size: 0.85rem;
    color: #dee4ff;
  }

  .tool-pill:hover {
    background: rgba(154, 182, 255, 0.22);
  }

  .pill-icon {
    font-size: 0.9rem;
    filter: brightness(0.85);
    opacity: 0.85;
  }

  .pill-remove {
    border: none;
    background: transparent;
    color: #dee4ff;
    cursor: pointer;
    font-size: 0.85rem;
    display: none;
  }

  .tool-pill:hover .pill-remove {
    display: inline-flex;
  }

  .input-hints {
    margin-top: 0.25rem;
    font-size: 0.75rem;
    opacity: 0.7;
    padding-left: 0.25rem;
  }
</style>
