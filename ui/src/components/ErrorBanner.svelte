<script lang="ts">
  import type { ParseWarning, WasmError } from '../lib/types';

  export let error: WasmError | null = null;
  export let warnings: ParseWarning[] = [];
  export let isSample = false;

  let warningsDismissed = false;
  let warningsExpanded = false;

  $: visibleWarnings = isSample ? [] : warnings;
  $: showWarnings = !warningsDismissed && visibleWarnings.length > 0;
  $: collapsed = visibleWarnings.length >= 4 && !warningsExpanded;

  function dismiss() {
    warningsDismissed = true;
  }

  function formatCode(code: string) {
    return code.replace(/_/g, ' ').toLowerCase();
  }
</script>

{#if error}
  <div class="banner error" role="alert">
    <span class="icon">✕</span>
    <div class="content">
      <strong>{formatCode(error.code)}:</strong> {error.message}
    </div>
  </div>
{/if}

{#if showWarnings}
  <div class="banner warning" role="status">
    <span class="icon">⚠</span>
    <div class="content">
      {#if collapsed}
        <strong>{visibleWarnings.length} warnings during trace load</strong>
        <button class="toggle" on:click={() => (warningsExpanded = true)}>Show all</button>
      {:else}
        <div class="warning-list" class:scrollable={visibleWarnings.length >= 4}>
          {#each visibleWarnings as w}
            <div class="warning-item">
              <span class="warn-code">[{w.code}]</span>
              {#if w.count > 1}<span class="count">×{w.count}</span>{/if}
              {w.message}
            </div>
          {/each}
        </div>
        {#if visibleWarnings.length >= 4}
          <button class="toggle" on:click={() => (warningsExpanded = false)}>Collapse</button>
        {/if}
      {/if}
    </div>
    <button class="dismiss" on:click={dismiss} aria-label="Dismiss warnings">✕</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .banner.error {
    background: var(--color-error-bg, #fee2e2);
    color: var(--color-error-text, #991b1b);
    border-bottom: 1px solid var(--color-error-border, #fca5a5);
  }

  .banner.warning {
    background: var(--color-warning-bg, #fef3c7);
    color: var(--color-warning-text, #92400e);
    border-bottom: 1px solid var(--color-warning-border, #fcd34d);
  }

  .content {
    flex: 1;
    min-width: 0;
  }

  .icon {
    flex-shrink: 0;
    font-size: 1rem;
    line-height: 1.4;
  }

  .dismiss {
    flex-shrink: 0;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    opacity: 0.6;
    padding: 0 0.25rem;
    color: inherit;
  }

  .dismiss:hover {
    opacity: 1;
  }

  .toggle {
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    text-decoration: underline;
    font-size: 0.85rem;
    padding: 0;
    margin-left: 0.5rem;
  }

  .warning-list {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .warning-list.scrollable {
    max-height: 200px;
    overflow-y: auto;
  }

  .warning-item {
    display: flex;
    gap: 0.35rem;
    align-items: baseline;
  }

  .warn-code {
    font-family: monospace;
    font-size: 0.78rem;
    opacity: 0.75;
    flex-shrink: 0;
  }

  .count {
    background: rgba(0, 0, 0, 0.1);
    border-radius: 3px;
    padding: 0 0.3rem;
    font-size: 0.78rem;
    flex-shrink: 0;
  }
</style>
