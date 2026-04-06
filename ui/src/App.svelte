<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { loadWasm, getInitWarnings } from './lib/wasm';
  import { openFilePicker, handleFile, handleRawInput } from './lib/input';
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
  let editorValue = '';
  let editorMessage: string | null = null;
  let liveParseTimer: ReturnType<typeof setTimeout> | null = null;
  let flameGraphView: { focusView: () => void } | null = null;
  let waterfallView: { focusView: () => void } | null = null;

  const LIVE_PARSE_DELAY_MS = 150;

  onMount(async () => {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.apply(prefersDark ? 'dark' : 'light');

    try {
      await loadWasm();
      wasmReady = true;
    } catch (e) {
      wasmError = String(e);
    }
  });

  function clearLiveParseTimer(): void {
    if (liveParseTimer === null) return;
    clearTimeout(liveParseTimer);
    liveParseTimer = null;
  }

  function applyEditorValue(): boolean {
    editorMessage = null;
    if (!editorValue.trim()) {
      traceState.reset();
      return false;
    }
    return handleRawInput(editorValue, false, false);
  }

  function scheduleLiveParse(): void {
    clearLiveParseTimer();
    liveParseTimer = setTimeout(() => {
      liveParseTimer = null;
      applyEditorValue();
    }, LIVE_PARSE_DELAY_MS);
  }

  function onEditorInput(): void {
    scheduleLiveParse();
  }

  function onEditorKeyDown(e: KeyboardEvent): void {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      void submitEditor();
    }
  }

  function loadEditorText(text: string): void {
    editorValue = text;
    clearLiveParseTimer();
    applyEditorValue();
  }

  function openEditorFilePicker(): void {
    openFilePicker(loadEditorText);
  }

  function loadSampleJson(): void {
    loadEditorText(SAMPLE_TRACE);
  }

  function clearEditorJson(): void {
    clearLiveParseTimer();
    editorMessage = null;
    editorValue = '';
    traceState.reset();
  }

  function onDroppedFile(file: File): void {
    void handleFile(file, loadEditorText);
  }

  async function pasteFromClipboard(): Promise<void> {
    editorMessage = null;
    try {
      const text = await navigator.clipboard.readText();
      if (!text.trim()) return;
      loadEditorText(text);
    } catch {
      editorMessage = 'Clipboard access was blocked. Paste directly into the editor instead.';
    }
  }

  function formatEditorJson(): void {
    editorMessage = null;
    if (!editorValue.trim()) return;
    try {
      editorValue = JSON.stringify(JSON.parse(editorValue), null, 2);
      clearLiveParseTimer();
      applyEditorValue();
    } catch {
      editorMessage = 'Input is not valid JSON, so it could not be formatted.';
    }
  }

  async function submitEditor(): Promise<void> {
    clearLiveParseTimer();
    const parsed = applyEditorValue();
    if (!parsed) return;
    activeView.set('waterfall');
    await tick();
    waterfallView?.focusView();
  }

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
      <Toolbar onOpenFile={openEditorFilePicker} />
      <ErrorBanner
        error={state.status === 'error' ? state.error : null}
        warnings={allWarnings}
        isSample={state.isSampleTrace}
      />
      <div class="main">
        <section class="editor-panel">
          <div class="editor-header">
            <div class="editor-copy">
              <div class="editor-title">Trace JSON input</div>
              <div class="editor-subtitle">Paste formatted or unformatted JSON, then submit it or let the graphs update live while you type.</div>
            </div>
            <div class="editor-actions">
              <button type="button" class="editor-btn editor-btn--ghost" on:click={clearEditorJson} disabled={!editorValue.trim()}>
                Clear JSON
              </button>
              <button type="button" class="editor-btn editor-btn--ghost" on:click={loadSampleJson}>
                Load sample JSON
              </button>
              <button type="button" class="editor-btn editor-btn--ghost" on:click={pasteFromClipboard}>
                Paste JSON
              </button>
              <button type="button" class="editor-btn editor-btn--ghost" on:click={formatEditorJson} disabled={!editorValue.trim()}>
                Format
              </button>
              <button type="button" class="editor-btn" on:click={submitEditor} disabled={!editorValue.trim()}>
                Submit JSON
              </button>
            </div>
          </div>

          <textarea
            class="editor-input"
            bind:value={editorValue}
            on:input={onEditorInput}
            on:keydown={onEditorKeyDown}
            placeholder="Paste a trace JSON payload here…"
            spellcheck="false"
            aria-label="Trace JSON input"
          ></textarea>

          <div class="editor-footer">
            <span class="editor-hint">Supports OTLP JSON · Jaeger JSON · OpenInference JSON · Use “Load sample JSON” to try the built-in example</span>
            {#if editorMessage}
              <span class="editor-message">{editorMessage}</span>
            {/if}
          </div>
        </section>

        {#if editorValue.trim()}
          <div class="workspace">
            {#if state.status === 'loaded' && state.flameLayout}
              {#if $activeView === 'waterfall' && state.waterfallLayout}
                <WaterfallView bind:this={waterfallView} layout={state.waterfallLayout} />
              {:else}
                <FlameGraph bind:this={flameGraphView} layout={state.flameLayout} />
              {/if}
              <SpanDetail />
            {:else if state.status === 'error'}
              <div class="empty-state">
                <div class="empty-icon">⚠️</div>
                <div class="empty-title">Could not parse trace</div>
                <div class="empty-sub">Update the JSON above and the flame graph and waterfall view will refresh when the payload becomes valid.</div>
              </div>
            {:else if state.status === 'loading'}
              <div class="empty-state">
                <div class="empty-icon">⏳</div>
                <div class="empty-title">Parsing trace JSON</div>
                <div class="empty-sub">The visualization will appear below when parsing completes.</div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
    <DropZone onFileDrop={onDroppedFile} />
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
    flex-direction: column;
    gap: 0.75rem;
    min-height: 0;
    overflow: hidden;
    padding: 0.75rem;
  }

  .editor-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 12px;
    background: var(--color-surface, #1e293b);
    box-shadow: 0 10px 30px rgba(15, 23, 42, 0.12);
  }

  .editor-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .editor-copy {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .editor-title {
    font-size: 1rem;
    font-weight: 700;
    color: var(--color-text, #e2e8f0);
  }

  .editor-subtitle {
    font-size: 0.875rem;
    color: var(--color-text-muted, #94a3b8);
    max-width: 720px;
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .editor-btn {
    padding: 0.55rem 0.9rem;
    background: var(--color-accent, #3b82f6);
    color: #fff;
    border: 1px solid transparent;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease, opacity 0.12s ease;
  }

  .editor-btn:hover:not(:disabled) {
    background: var(--color-accent-hover, #2563eb);
  }

  .editor-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .editor-btn--ghost {
    background: transparent;
    color: var(--color-text, #e2e8f0);
    border-color: var(--color-border, #334155);
  }

  .editor-btn--ghost:hover:not(:disabled) {
    background: rgba(59, 130, 246, 0.08);
    border-color: var(--color-accent, #3b82f6);
  }

  .editor-input {
    width: 100%;
    min-height: 280px;
    resize: vertical;
    border: 1px solid var(--color-border, #334155);
    border-radius: 10px;
    padding: 1rem;
    background: var(--color-bg, #0f172a);
    color: var(--color-text, #e2e8f0);
    font: 0.875rem/1.6 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    outline: none;
  }

  .editor-input:focus {
    border-color: var(--color-accent, #3b82f6);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.16);
  }

  .editor-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .editor-hint {
    font-size: 0.8125rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .editor-message {
    font-size: 0.8125rem;
    color: #fca5a5;
  }

  .workspace {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--color-border, #334155);
    border-radius: 12px;
    background: var(--color-surface, #1e293b);
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
