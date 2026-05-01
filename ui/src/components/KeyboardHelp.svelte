<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  const dispatch = createEventDispatcher<{ close: void }>();

  const shortcuts: { keys: string; action: string }[] = [
    { keys: '?', action: 'Show / hide this help' },
    { keys: 'Esc', action: 'Close help / deselect span' },
    { keys: 'Ctrl+O', action: 'Open trace file' },
    { keys: 'Ctrl+K', action: 'Focus search bar' },
    { keys: 'Ctrl+V', action: 'Paste JSON from clipboard' },
    { keys: 'Ctrl+Enter', action: 'Submit and parse trace' },
    { keys: '1 / 2 / 3', action: 'Switch to Flame / Timeline / Waterfall view' },
    { keys: '↑↓←→', action: 'Navigate spans in flame graph' },
    { keys: 'Enter', action: 'Select focused span' },
    { keys: 'F', action: 'Zoom to selected span' },
    { keys: '0', action: 'Reset zoom' },
    { keys: 'Ctrl+Scroll', action: 'Zoom flame / timeline' },
  ];

  onMount(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        dispatch('close');
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  });
</script>

<div class="overlay" role="dialog" aria-label="Keyboard shortcuts" tabindex="-1" on:click|self={() => dispatch('close')} on:keydown={() => {}}>
  <div class="panel">
    <div class="header">
      <h2>Keyboard shortcuts</h2>
      <button class="close-btn" aria-label="Close" on:click={() => dispatch('close')}>✕</button>
    </div>
    <div class="shortcuts">
      {#each shortcuts as { keys, action }}
        <div class="row">
          <kbd class="key">{keys}</kbd>
          <span class="action">{action}</span>
        </div>
      {/each}
    </div>
    <div class="footer">
      Press <kbd>?</kbd> at any time to show this reference.
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(4px);
  }

  .panel {
    background: var(--color-surface, #1e293b);
    border: 1px solid var(--color-border, #334155);
    border-radius: 14px;
    padding: 1.5rem;
    max-width: 420px;
    width: calc(100% - 2rem);
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.35);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  .header h2 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--color-text, #e2e8f0);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--color-text-muted, #94a3b8);
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 4px;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--color-text, #e2e8f0);
    background: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  kbd.key {
    display: inline-block;
    min-width: 100px;
    padding: 0.15rem 0.45rem;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--color-border, #334155);
    border-radius: 5px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.75rem;
    color: var(--color-text, #e2e8f0);
    text-align: center;
    white-space: nowrap;
  }

  .action {
    font-size: 0.85rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .footer {
    margin-top: 1rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--color-border, #334155);
    font-size: 0.78rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .footer kbd {
    padding: 0.1rem 0.35rem;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--color-border, #334155);
    border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.75rem;
    color: var(--color-text, #e2e8f0);
  }
</style>
