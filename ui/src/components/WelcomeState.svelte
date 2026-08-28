<script lang="ts">
  import Icon from './ui/Icon.svelte';
  import Button from './ui/Button.svelte';

  /**
   * First run, rendered *inside* the visualization stage rather than as a hero
   * panel stacked above the editor. The workspace is the product; this is just
   * the three ways to put a trace into it.
   */
  export let onLoadSample: () => void;
  export let onOpenFile: () => void;
  export let onPaste: () => void;
</script>

<div class="welcome">
  <div class="welcome-inner">
    <h1 class="welcome-title">Load a trace</h1>
    <p class="welcome-sub">Parsed in your browser — nothing is uploaded.</p>

    <div class="welcome-actions">
      <Button variant="primary" on:click={onLoadSample}>Load sample trace</Button>
      <Button on:click={onOpenFile}>Open file <kbd>⌘O</kbd></Button>
      <Button on:click={onPaste}>Paste JSON <kbd>⌘V</kbd></Button>
    </div>

    <div class="welcome-formats" aria-label="Supported formats">
      <span class="fmt"><b>OTLP</b> resourceSpans</span>
      <span class="fmt"><b>Jaeger</b> data[].spans</span>
      <span class="fmt"><b>OpenInference</b> spans</span>
    </div>

    <p class="welcome-note">
      <Icon name="shield" size={12} /> no backend · no upload · no telemetry
      <span class="sep">·</span> drag a .json anywhere
    </p>
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    min-height: 0;
    overflow-y: auto;
  }

  .welcome-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    text-align: center;
    max-width: 46ch;
  }

  .welcome-title {
    font-size: var(--text-xl);
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--color-text);
  }

  .welcome-sub {
    font-size: var(--text-base);
    color: var(--color-text-muted);
  }

  .welcome-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .welcome-actions :global(kbd) {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    opacity: 0.7;
  }

  .welcome-formats {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .fmt {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    letter-spacing: 0.03em;
    padding: 0.25rem 0.6rem;
    border: 1px solid var(--color-border-soft);
    border-radius: var(--radius-pill);
    background: var(--color-panel-subtle);
    color: var(--color-text-muted);
  }

  .fmt b { color: var(--color-sky); font-weight: 600; }

  .welcome-note {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-2);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-faint);
  }

  .sep { opacity: 0.5; }

  @media (max-width: 520px) {
    .welcome-actions { flex-direction: column; align-self: stretch; }
  }
</style>
