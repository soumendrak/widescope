<script lang="ts">
  import Icon from './ui/Icon.svelte';
  import Button from './ui/Button.svelte';
  import { recentTraces, getRecentJson, clearRecent } from '../lib/recent';

  /**
   * First run, rendered *inside* the visualization stage rather than as a hero
   * panel stacked above the editor. The workspace is the product; this is just
   * the three ways to put a trace into it.
   */
  export let onLoadSample: () => void;
  export let onOpenFile: () => void;
  export let onPaste: () => void;
  /** Reload a trace kept in IndexedDB from a previous session. */
  export let onLoadText: (text: string) => void = () => {};

  async function reloadRecent(id: number): Promise<void> {
    const json = await getRecentJson(id);
    if (json) onLoadText(json);
  }
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

    {#if $recentTraces.length > 0}
      <div class="welcome-recent" aria-label="Recent traces">
        <span class="recent-label">Recent</span>
        {#each $recentTraces as r (r.id)}
          <button
            type="button"
            class="recent-chip"
            title={`Reload ${r.name} · ${(r.size / 1024 / 1024).toFixed(1)} MB`}
            on:click={() => void reloadRecent(r.id)}
          >{r.name}</button>
        {/each}
        <button type="button" class="recent-chip recent-chip--clear" title="Forget all recent traces" on:click={() => void clearRecent()}>Clear</button>
      </div>
    {/if}

    <p class="welcome-note">
      <Icon name="shield" size={12} /> no backend · no upload · no telemetry
      <span class="sep">·</span> drag a .json anywhere
      <span class="sep">·</span> <a class="welcome-link" href="/docs/">no trace yet?</a>
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

  .welcome-recent {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
  }
  .recent-label {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-text-faint);
  }
  .recent-chip {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-muted);
    background: var(--color-bg-raised);
    border: 1px solid var(--color-border-soft);
    border-radius: var(--radius-pill);
    padding: 0.2rem 0.7rem;
    cursor: pointer;
    max-width: 22ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .recent-chip:hover {
    color: var(--color-sky);
    border-color: var(--color-sky);
  }
  .recent-chip--clear {
    color: var(--color-text-faint);
  }
  .welcome-link {
    color: var(--color-accent);
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
