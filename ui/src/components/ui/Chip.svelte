<script lang="ts">
  /**
   * Small pill. `interactive` makes it a real <button> (filter chips that
   * remove themselves); otherwise it is a static <span> badge.
   */
  export let tone: 'neutral' | 'accent' | 'llm' | 'danger' | 'success' = 'neutral';
  export let interactive = false;
</script>

{#if interactive}
  <button type="button" class="chip chip--{tone} chip--btn" on:click {...$$restProps}>
    <slot />
  </button>
{:else}
  <span class="chip chip--{tone}" {...$$restProps}><slot /></span>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0.15rem 0.45rem;
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    line-height: 1.5;
    white-space: nowrap;
  }
  .chip--neutral {
    background: var(--color-panel-subtle);
    color: var(--color-text-muted);
    border-color: var(--color-border);
  }
  .chip--accent { background: var(--color-badge-bg); color: var(--color-badge-text); }
  .chip--llm { background: var(--color-llm-badge-bg); color: var(--color-llm-badge-text); }
  .chip--danger {
    background: color-mix(in srgb, var(--color-danger) 16%, transparent);
    color: var(--color-danger);
  }
  .chip--success {
    background: color-mix(in srgb, var(--color-success) 16%, transparent);
    color: var(--color-success);
  }
  .chip--btn { cursor: pointer; font-family: inherit; }
  .chip--btn:hover { border-color: currentColor; }
</style>
