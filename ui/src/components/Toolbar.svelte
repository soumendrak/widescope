<script lang="ts">
  import { traceState } from '../stores/trace';
  import { openFilePicker } from '../lib/input';
  import { activeView } from '../stores/selection';

  const FORMAT_LABELS: Record<string, string> = {
    OtlpJson: 'OTLP JSON',
    JaegerJson: 'Jaeger JSON',
    OpenInferenceJson: 'OpenInference',
  };

  $: summary = $traceState.summary;
  $: isSample = $traceState.isSampleTrace;
  $: status = $traceState.status;
</script>

<header class="toolbar">
  <div class="left">
    <div class="brand">
      <span class="logo">🔭</span>
      <span class="name">WideScope</span>
    </div>

    <button type="button" class="btn-open" on:click={openFilePicker}>
      Open file
    </button>
  </div>

  <div class="center">
    {#if status === 'loading'}
      <span class="status-loading">Parsing…</span>
    {:else if summary}
      {#if isSample}
        <span class="sample-badge">Sample trace</span>
      {/if}
      {#if summary.detected_format}
        <span class="format-badge">{FORMAT_LABELS[summary.detected_format] ?? summary.detected_format}</span>
      {/if}
      <span class="summary-text">
        {summary.span_count.toLocaleString()} span{summary.span_count !== 1 ? 's' : ''}
        · {summary.service_count} service{summary.service_count !== 1 ? 's' : ''}
        · {summary.total_duration_display}
        {#if summary.has_errors}<span class="error-dot" title="Trace contains errors">⚠ errors</span>{/if}
      </span>
    {/if}
  </div>

  <div class="right">
    {#if status === 'loaded'}
      <div class="view-tabs" role="tablist" aria-label="View mode">
        <button
          type="button"
          class="view-tab"
          class:view-tab--active={$activeView === 'flame'}
          role="tab"
          aria-selected={$activeView === 'flame'}
          on:click={() => activeView.set('flame')}
        >🔥 Flame</button>
        <button
          type="button"
          class="view-tab"
          class:view-tab--active={$activeView === 'waterfall'}
          role="tab"
          aria-selected={$activeView === 'waterfall'}
          on:click={() => activeView.set('waterfall')}
        >≋ Waterfall</button>
      </div>
    {/if}
  </div>
</header>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0 0.75rem;
    height: 44px;
    position: relative;
    isolation: isolate;
    background: var(--color-toolbar, #1e293b);
    color: var(--color-toolbar-text, #f1f5f9);
    border-bottom: 1px solid var(--color-border, #334155);
    flex-shrink: 0;
    z-index: 10;
  }

  .left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-shrink: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-weight: 700;
    font-size: 1rem;
    letter-spacing: -0.01em;
  }

  .logo {
    font-size: 1.1rem;
  }

  .center {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    min-width: 0;
    overflow: hidden;
  }

  .right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-open {
    padding: 0.3rem 0.7rem;
    background: var(--color-accent, #3b82f6);
    color: #fff;
    border: none;
    border-radius: 5px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-open:hover {
    background: var(--color-accent-hover, #2563eb);
  }

  .format-badge {
    background: rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    padding: 0.1rem 0.45rem;
    font-size: 0.78rem;
    font-family: monospace;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .sample-badge {
    background: rgba(251, 191, 36, 0.2);
    color: #fbbf24;
    border-radius: 4px;
    padding: 0.1rem 0.45rem;
    font-size: 0.78rem;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .summary-text {
    color: var(--color-toolbar-muted, #94a3b8);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .error-dot {
    color: #f87171;
    margin-left: 0.25rem;
  }

  .status-loading {
    color: var(--color-toolbar-muted, #94a3b8);
    font-style: italic;
  }

  .view-tabs {
    display: flex;
    gap: 2px;
    background: rgba(0, 0, 0, 0.25);
    border-radius: 6px;
    padding: 2px;
  }

  .view-tab {
    padding: 0.2rem 0.65rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.12s, color 0.12s;
  }

  .view-tab:hover {
    color: var(--color-toolbar-text, #f1f5f9);
    background: rgba(255, 255, 255, 0.08);
  }

  .view-tab--active {
    background: var(--color-accent, #3b82f6);
    color: #fff;
  }

  .view-tab--active:hover {
    background: var(--color-accent-hover, #2563eb);
  }
</style>
