<script lang="ts">
  /**
   * The one button. Replaces .editor-btn / .welcome-btn / .toolbar-btn /
   * .add-btn / .diff-clear, which had drifted to different paddings, radii and
   * hover colours across six components.
   */
  export let variant: 'primary' | 'default' | 'ghost' | 'danger' = 'default';
  export let size: 'sm' | 'md' = 'md';
  export let disabled = false;
  /** Renders an icon-only square button; `title`/`aria-label` then carry the name. */
  export let icon = false;
  export let type: 'button' | 'submit' = 'button';
</script>

<button
  {type}
  {disabled}
  class="btn btn--{variant} btn--{size}"
  class:btn--icon={icon}
  on:click
  {...$$restProps}
>
  <slot />
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-panel-subtle);
    color: var(--color-text);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    line-height: 1;
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--dur-fast) var(--ease-smooth),
      border-color var(--dur-fast) var(--ease-smooth),
      color var(--dur-fast) var(--ease-smooth);
  }

  .btn--md { padding: 0.45rem 0.75rem; }
  .btn--sm { padding: 0.3rem 0.55rem; font-size: var(--text-xs); }
  .btn--icon { padding: 0.35rem; aspect-ratio: 1; }
  .btn--sm.btn--icon { padding: 0.25rem; }

  .btn:hover:not(:disabled) {
    background: var(--color-panel-highlight);
    border-color: var(--color-sky);
  }

  .btn:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn--primary {
    background: var(--grad-cta);
    border-color: transparent;
    color: #fff;
    box-shadow: var(--shadow-cta);
  }
  .btn--primary:hover:not(:disabled) {
    background: var(--grad-cta);
    border-color: transparent;
    filter: brightness(1.08);
  }

  .btn--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--color-text-muted);
  }
  .btn--ghost:hover:not(:disabled) {
    background: var(--color-panel-subtle);
    border-color: var(--color-border);
    color: var(--color-text);
  }

  .btn--danger { color: var(--color-danger); border-color: var(--color-border); }
  .btn--danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-color: var(--color-danger);
  }
</style>
