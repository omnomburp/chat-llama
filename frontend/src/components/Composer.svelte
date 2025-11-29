<script>
  import { createEventDispatcher } from 'svelte';
  import { ACCEPTED_FILE_TYPES } from '../lib/fileUtils.js';

  export let inputValue = '';
  export let loading = false;
  export let currentUseSearch = true;
  export let toolMenuOpen = false;
  export let hasBottomInset = false;
  export let bottomInset = 0;
  export let attachments = [];
  export let maxAttachments = 3;
  export let attachmentError = '';
  export let attachmentsLoading = false;

  const dispatch = createEventDispatcher();
  let fileInput;
  let dragCounter = 0;
  let isDraggingFiles = false;

  function handleInput(e) {
    dispatch('input', e.target.value);
  }

  function triggerFileDialog() {
    if (attachmentsLoading) return;
    if (fileInput) {
      fileInput.click();
    }
  }

  function handleFileChange(event) {
    const files = Array.from(event.target.files ?? []);
    if (files.length) {
      dispatch('addFiles', files);
    }
    event.target.value = '';
  }

  function handleRemoveAttachment(id) {
    dispatch('removeAttachment', id);
  }

  function handleDragEnter(event) {
    if (!event.dataTransfer) return;
    dragCounter += 1;
    if (event.dataTransfer.types?.includes('Files')) {
      isDraggingFiles = true;
    }
  }

  function handleDragLeave(event) {
    if (!event.dataTransfer) return;
    dragCounter = Math.max(dragCounter - 1, 0);
    if (dragCounter === 0) {
      isDraggingFiles = false;
    }
  }

  function handleDrop(event) {
    if (!event.dataTransfer) return;
    const files = Array.from(event.dataTransfer.files ?? []);
    isDraggingFiles = false;
    dragCounter = 0;
    if (files.length) {
      dispatch('addFiles', files);
    }
  }
</script>

<form
  class={`input-bar ${hasBottomInset ? 'with-inset' : ''} ${isDraggingFiles ? 'drag-active' : ''}`}
  style={`--bottom-inset: ${Math.min(Math.max(bottomInset, 0), 200)}px;`}
  on:submit|preventDefault={() => dispatch('send')}
  on:dragenter|preventDefault={handleDragEnter}
  on:dragover|preventDefault
  on:dragleave|preventDefault={handleDragLeave}
  on:drop|preventDefault={handleDrop}
>
  {#if isDraggingFiles}
    <div class="drag-overlay" aria-hidden="true">
      Drop files to attach
    </div>
  {/if}
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
        <button
          type="button"
          class={`tool-item ${attachments.length >= maxAttachments ? 'disabled' : ''}`}
          disabled={attachments.length >= maxAttachments || attachmentsLoading}
          on:click={() => {
            triggerFileDialog();
            dispatch('toggleMenu', false);
          }}
        >
          <span>Add files</span>
          <span class="tool-check">
            {#if attachmentsLoading}
              …
            {:else}
              {attachments.length}/{maxAttachments}
            {/if}
          </span>
        </button>
        <div class="tool-item disabled" aria-disabled="true">
          <span>Add images</span>
        </div>
      </div>
      <input
        type="file"
        accept={ACCEPTED_FILE_TYPES}
        multiple
        class="file-input"
        bind:this={fileInput}
        on:change={handleFileChange}
      />
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

    {#if attachments.length}
      <div class="attachment-preview">
        {#each attachments as file (file.id)}
          <div class="attachment-chip">
            <div class="chip-text">
              <span class="chip-name">{file.name}</span>
              {#if file.sizeLabel}
                <span class="chip-size">{file.sizeLabel}</span>
              {/if}
            </div>
            <button
              type="button"
              class="chip-remove"
              aria-label={`Remove ${file.name}`}
              on:click={() => handleRemoveAttachment(file.id)}
            >
              ✕
            </button>
          </div>
        {/each}
      </div>
    {/if}

    {#if attachmentError}
      <div class="attachment-error" aria-live="polite">{attachmentError}</div>
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
    <button
      class="send-button"
      type="submit"
      disabled={loading || (!inputValue.trim() && attachments.length === 0)}
    >
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
    bottom: var(--bottom-inset, 0px);
    padding-bottom: calc(env(safe-area-inset-bottom, 0.3rem) + var(--bottom-inset, 0px));
    background: linear-gradient(to top, #111214 60%, rgba(17, 18, 20, 0));
    z-index: 2;
    overflow: visible;
  }

  .input-bar.with-inset {
    padding-bottom: calc(env(safe-area-inset-bottom, 0.3rem) + var(--bottom-inset, 0px) + 1.5rem);
  }

  .input-inner {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
    border-radius: 1.1rem;
    padding: 0.6rem 0.85rem;
    border: 1px solid #2a2b31;
    background: #18191d;
  }

  .input-bar.drag-active .input-inner {
    border-color: rgba(154, 182, 255, 0.65);
    box-shadow: 0 0 0 1px rgba(154, 182, 255, 0.35);
    background: rgba(24, 25, 29, 0.92);
  }

  .drag-overlay {
    position: absolute;
    inset: 0;
    border-radius: 1.2rem;
    border: 1px dashed rgba(154, 182, 255, 0.5);
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.95rem;
    color: #d5ddff;
    background: rgba(17, 18, 20, 0.65);
  }

  .input-inner textarea {
    flex: 1 1 auto;
    min-width: 180px;
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

  .file-input {
    display: none;
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

  .attachment-preview {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    width: 100%;
    flex: 1 1 100%;
  }

  .attachment-chip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border: 1px solid #2f3036;
    border-radius: 0.7rem;
    padding: 0.35rem 0.55rem;
    background: rgba(255, 255, 255, 0.03);
    gap: 0.4rem;
  }

  .chip-text {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }

  .chip-name {
    font-size: 0.85rem;
  }

  .chip-size {
    font-size: 0.75rem;
    color: #9ea0ac;
  }

  .chip-remove {
    border: none;
    background: transparent;
    color: #c9cad3;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.1rem;
  }

  .attachment-error {
    width: 100%;
    flex: 1 1 100%;
    font-size: 0.78rem;
    color: #f87171;
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
