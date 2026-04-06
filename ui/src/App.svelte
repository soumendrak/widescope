<script lang="ts">
  import { onMount } from 'svelte';
  import { loadWasm, getInitWarnings } from './lib/wasm';
  import { handleRawInput } from './lib/input';
  import { SAMPLE_TRACE } from './lib/sample';
  import { traceState } from './stores/trace';
  import { theme } from './lib/theme';

  import Toolbar from './components/Toolbar.svelte';
  import FlameGraph from './components/FlameGraph.svelte';
  import WaterfallView from './components/WaterfallView.svelte';
  import SpanDetail from './components/SpanDetail.svelte';
  import DropZone from './components/DropZone.svelte';
  import ErrorBanner from './components/ErrorBanner.svelte';
  import { activeView } from './stores/selection';

  let wasmReady = false;
  let wasmError: string | null = null;

  onMount(async () => {
    // Apply system theme preference
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.apply(prefersDark ? 'dark' : 'light');

    try {
      await loadWasm();
      wasmReady = true;
      // Auto-load sample trace on first visit
      handleRawInput(SAMPLE_TRACE, true);
    } catch (e) {
      wasmError = String(e);
    }
  });

  $: state = $traceState;
  $: warnings = state.summary?.warnings ?? [];
  $: initWarnings = wasmReady ? getInitWarnings() : [];
  $: allWarnings = [...initWarnings, ...warnings];
</script>

<div class="app" data-theme="dark">
  {#if wasmError}
    <div class="fatal-error">
      <h2>Failed to initialize WideScope</h2>
      <pre>{wasmError}</pre>
      <p>Please try refreshing the page. If the issue persists, check that your browser supports WebAssembly.</p>
    </div>
  {:else if !wasmReady}
    <div class="splash">
      <div class="splash-inner">
        <span class="splash-logo">🔭</span>
        <span class="splash-name">WideScope</span>
        <span class="splash-loading">Loading…</span>
      </div>
    </div>
  {:else}
    <div class="layout">
      <Toolbar />
      <ErrorBanner
        error={state.status === 'error' ? state.error : null}
        warnings={allWarnings}
        isSample={state.isSampleTrace}
      />
      <div class="main">
        {#if state.status === 'empty' || state.status === 'loading'}
          <div class="empty-state">
            <div class="empty-icon">🔭</div>
            <div class="empty-title">Drop a trace file to get started</div>
            <div class="empty-sub">Supports OTLP JSON · Drag & drop or click "Open file"</div>
          </div>
        {:else if state.status === 'loaded' && state.flameLayout}
          {#if $activeView === 'waterfall' && state.waterfallLayout}
            <WaterfallView layout={state.waterfallLayout} />
          {:else}
            <FlameGraph layout={state.flameLayout} />
          {/if}
          <SpanDetail />
        {:else if state.status === 'error'}
          <div class="empty-state">
            <div class="empty-icon">⚠️</div>
            <div class="empty-title">Could not parse trace</div>
            <div class="empty-sub">Drop a different file or check the error above</div>
          </div>
        {/if}
      </div>
    </div>
    <DropZone />
  {/if}
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(html, body) {
    height: 100%;
    font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
    font-size: 14px;
  }

  :global([data-theme='dark']) {
    --color-bg: #0f172a;
    --color-surface: #1e293b;
    --color-toolbar: #1e293b;
    --color-toolbar-text: #f1f5f9;
    --color-toolbar-muted: #94a3b8;
    --color-border: #334155;
    --color-text: #e2e8f0;
    --color-text-muted: #94a3b8;
    --color-accent: #3b82f6;
    --color-accent-hover: #2563eb;
    --color-canvas-bg: #0f172a;
    --color-sidebar: #1e293b;
    --color-sidebar-text: #e2e8f0;
    --color-error-bg: #450a0a;
    --color-error-text: #fca5a5;
    --color-error-border: #991b1b;
    --color-warning-bg: #451a03;
    --color-warning-text: #fcd34d;
    --color-warning-border: #92400e;
    --focus-color: #3b82f6;
  }

  :global([data-theme='light']) {
    --color-bg: #f8fafc;
    --color-surface: #ffffff;
    --color-toolbar: #1e293b;
    --color-toolbar-text: #f1f5f9;
    --color-toolbar-muted: #94a3b8;
    --color-border: #e2e8f0;
    --color-text: #1e293b;
    --color-text-muted: #64748b;
    --color-accent: #3b82f6;
    --color-accent-hover: #2563eb;
    --color-canvas-bg: #f1f5f9;
    --color-sidebar: #ffffff;
    --color-sidebar-text: #1e293b;
    --color-error-bg: #fee2e2;
    --color-error-text: #991b1b;
    --color-error-border: #fca5a5;
    --color-warning-bg: #fef3c7;
    --color-warning-text: #92400e;
    --color-warning-border: #fcd34d;
    --focus-color: #2563eb;
  }

  @media (prefers-reduced-motion: reduce) {
    :global(*) { animation: none !important; transition: none !important; }
  }

  .app {
    height: 100vh;
    overflow: hidden;
    background: var(--color-bg, #0f172a);
    color: var(--color-text, #e2e8f0);
    display: flex;
    flex-direction: column;
  }

  .layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .main {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 0.5rem;
  }

  .empty-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-text, #e2e8f0);
  }

  .empty-sub {
    font-size: 0.875rem;
    font-family: monospace;
  }

  .splash {
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #0f172a;
  }

  .splash-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    color: #e2e8f0;
  }

  .splash-logo { font-size: 3rem; }
  .splash-name { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }
  .splash-loading { font-size: 0.875rem; color: #64748b; }

  .fatal-error {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 2rem;
    background: #0f172a;
    color: #f87171;
    text-align: center;
  }

  .fatal-error pre {
    background: rgba(255, 255, 255, 0.05);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.8rem;
    max-width: 600px;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .fatal-error p { color: #94a3b8; font-size: 0.875rem; }
</style>
