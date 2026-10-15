<script lang="ts">
  /**
   * Shaped placeholder for the stage while a trace parses. Skeleton bars in
   * roughly waterfall proportions read as "the view is coming" — a spinner
   * only says "something is happening".
   */
  export let label: string;
  export let progress = 20;

  // Widths/indents chosen to echo a real trace tree, not to represent one.
  const ROWS = [
    { w: 96, i: 0 }, { w: 62, i: 4 }, { w: 48, i: 8 }, { w: 71, i: 8 },
    { w: 35, i: 12 }, { w: 54, i: 4 }, { w: 43, i: 8 }, { w: 28, i: 12 },
  ];
</script>

<div class="skeleton" role="status" aria-live="polite" aria-label={label}>
  <div class="rows" aria-hidden="true">
    {#each ROWS as row, i}
      <div class="row" style={`--w:${row.w}%; --i:${row.i}%; --d:${i * 70}ms`}></div>
    {/each}
  </div>
  <div class="foot">
    <span class="label">{label}</span>
    <div class="bar"><div class="fill" style={`width:${progress}%`}></div></div>
  </div>
</div>

<style>
  .skeleton {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-5);
    padding: var(--space-6);
    min-height: 0;
  }

  .rows { display: flex; flex-direction: column; gap: var(--space-2); }

  .row {
    height: 14px;
    width: var(--w);
    margin-left: var(--i);
    border-radius: var(--radius-xs);
    background: linear-gradient(
      90deg,
      var(--color-panel-subtle) 0%,
      var(--color-panel-highlight) 50%,
      var(--color-panel-subtle) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.4s var(--ease-smooth) infinite;
    animation-delay: var(--d);
  }

  .foot {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .label {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  .bar {
    flex: 1;
    height: 3px;
    border-radius: var(--radius-pill);
    background: var(--color-panel-subtle);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--color-accent);
    border-radius: var(--radius-pill);
    transition: width var(--dur-slow) var(--ease-out);
  }

  @keyframes shimmer {
    from { background-position: 200% 0; }
    to { background-position: -200% 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .row { animation: none; }
  }
</style>
