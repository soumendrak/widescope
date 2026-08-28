<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Dialog from './ui/Dialog.svelte';

  const dispatch = createEventDispatcher<{ close: void }>();

  const shortcuts: { keys: string; action: string }[] = [
    { keys: '?', action: 'Show / hide this help' },
    { keys: 'Esc', action: 'Close help / deselect span' },
    { keys: 'Ctrl+O', action: 'Open trace file' },
    { keys: 'Ctrl+K', action: 'Open the command palette' },
    { keys: 'Ctrl+V', action: 'Paste JSON from clipboard' },
    { keys: 'Ctrl+Enter', action: 'Collapse the editor and focus the trace' },
    { keys: '1 / 2 / 3 / 4 / 5', action: 'Switch lens — slot 3 is Conversation on LLM traces, Timeline otherwise' },
    { keys: '\u2191\u2193\u2190\u2192', action: 'Navigate spans in flame graph' },
    { keys: 'Enter', action: 'Select focused span' },
    { keys: 'F', action: 'Zoom to selected span' },
    { keys: '0', action: 'Reset zoom' },
    { keys: 'Ctrl+Scroll', action: 'Zoom flame / timeline' },
  ];
</script>

<Dialog open title="Keyboard shortcuts" maxWidth="430px" on:close={() => dispatch('close')}>
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
</Dialog>

<style>
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
