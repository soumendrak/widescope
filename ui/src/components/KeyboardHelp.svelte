<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { fadeConfig, scaleConfig } from '../lib/animation';

  const dispatch = createEventDispatcher<{ close: void }>();

  const shortcuts: { keys: string; action: string }[] = [
    { keys: '?', action: 'Show / hide this help' },
    { keys: 'Esc', action: 'Close help / deselect span' },
    { keys: 'Ctrl+O', action: 'Open trace file' },
    { keys: 'Ctrl+K', action: 'Focus search bar' },
    { keys: 'Ctrl+V', action: 'Paste JSON from clipboard' },
    { keys: 'Ctrl+Enter', action: 'Submit and parse trace' },
    { keys: '1 / 2 / 3 / 4 / 5 / 6 / 7', action: 'Switch to Flame / Timeline / Waterfall / Graph / Agent / Diff / Trends' },
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

<div class="overlay" role="dialog" aria-label="Keyboard shortcuts" tabindex="-1" on:click|self={() => dispatch('close')} on:keydown={() => {}} transition:fade={fadeConfig()}>
  <div class="panel" transition:scale={scaleConfig()}>
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
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface, #0d1626) 96%, var(--color-accent)), var(--color-surface, #0d1626));
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 16px;
    padding: 1.5rem;
    max-width: 430px;
    width: calc(100% - 2rem);
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.12) inset;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  .header h2 {
    font-family: var(--font-display);
    font-size: 1.15rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    color: var(--color-text, #e9eff8);
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
    padding: 0.18rem 0.5rem;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-bottom-width: 2px;
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-sky, #7dd3fc);
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
