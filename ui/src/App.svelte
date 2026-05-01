<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { loadWasm, getInitWarnings } from './lib/wasm';
  import { openFilePicker, handleFile, handleRawInput } from './lib/input';
  import { SAMPLE_TRACE } from './lib/sample';
  import { traceState } from './stores/trace';
  import { theme } from './lib/theme';

  import Toolbar from './components/Toolbar.svelte';
  import FlameGraph from './components/FlameGraph.svelte';
  import Timeline from './components/Timeline.svelte';
  import WaterfallView from './components/WaterfallView.svelte';
  import ServiceGraph from './components/ServiceGraph.svelte';
  import DiffView from './components/DiffView.svelte';
  import SpanDetail from './components/SpanDetail.svelte';
  import DropZone from './components/DropZone.svelte';
  import ErrorBanner from './components/ErrorBanner.svelte';
  import KeyboardHelp from './components/KeyboardHelp.svelte';
  import { activeView, focusedSpanId, hoveredSpanId, searchQuery, searchResults, selectedSpanId } from './stores/selection';

  let wasmReady = false;
  let wasmError: string | null = null;
  let editorValue = '';
  let editorMessage: string | null = null;
  let editorCollapsed = false;
  let editorInputEl: HTMLTextAreaElement;
  let editorResizeObserver: ResizeObserver | null = null;
  let editorCurrentHeight = 280;
  let editorExpandedHeight = 280;
  let isEditorResizing = false;
  let editorResizeStartY = 0;
  let editorResizeStartHeight = 0;
  let liveParseTimer: ReturnType<typeof setTimeout> | null = null;
  let flameGraphView: { focusView: () => void } | null = null;
  let timelineView: { focusView: () => void } | null = null;
  let waterfallView: { focusView: () => void } | null = null;
  let showKeyboardHelp = false;

  const STORAGE_KEY_THEME = 'widescope:theme';
  const STORAGE_KEY_VIEW = 'widescope:view';
  const STORAGE_KEY_EDITOR = 'widescope:editor';

  const LIVE_PARSE_DELAY_MS = 150;
  const DEFAULT_EDITOR_HEIGHT_PX = 280;
  const EMPTY_EDITOR_HEIGHT_PX = 160;
  const COLLAPSED_EDITOR_HEIGHT_PX = 88;
  const AUTO_EXPAND_EDITOR_DELTA_PX = 24;

  onMount(async () => {
    const storedTheme = localStorage.getItem(STORAGE_KEY_THEME);
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.apply(storedTheme === 'dark' ? 'dark' : storedTheme === 'light' ? 'light' : (prefersDark ? 'dark' : 'light'));

    const storedView = localStorage.getItem(STORAGE_KEY_VIEW);
    if (storedView === 'flame' || storedView === 'timeline' || storedView === 'waterfall' || storedView === 'graph' || storedView === 'diff') {
      activeView.set(storedView);
    }

    if (editorInputEl) {
      editorResizeObserver = new ResizeObserver((entries) => {
        const nextHeight = Math.max(
          COLLAPSED_EDITOR_HEIGHT_PX,
          Math.round(entries[0]?.contentRect.height ?? editorInputEl.getBoundingClientRect().height)
        );

        editorCurrentHeight = nextHeight;

        if (editorCollapsed) {
          if (nextHeight > COLLAPSED_EDITOR_HEIGHT_PX + AUTO_EXPAND_EDITOR_DELTA_PX) {
            editorCollapsed = false;
            editorExpandedHeight = Math.max(DEFAULT_EDITOR_HEIGHT_PX, nextHeight);
          }
          return;
        }

        editorExpandedHeight = Math.max(DEFAULT_EDITOR_HEIGHT_PX, nextHeight);
      });

      editorResizeObserver.observe(editorInputEl);
      editorCurrentHeight = Math.max(COLLAPSED_EDITOR_HEIGHT_PX, Math.round(editorInputEl.getBoundingClientRect().height));
      editorExpandedHeight = Math.max(DEFAULT_EDITOR_HEIGHT_PX, editorCurrentHeight);
    }

    try {
      await loadWasm();
      wasmReady = true;
    } catch (e) {
      wasmError = String(e);
      return;
    }

    const params = new URLSearchParams(window.location.search);
    const traceUrl = params.get('trace');
    if (traceUrl) {
      try {
        await loadTraceFromUrl(traceUrl);
      } catch {
        editorMessage = 'Failed to load trace from URL.';
      }
    }
  });

  onDestroy(() => {
    clearLiveParseTimer();
    editorResizeObserver?.disconnect();
    if (globalKeydownHandler) document.removeEventListener('keydown', globalKeydownHandler);
  });

  function clearLiveParseTimer(): void {
    if (liveParseTimer === null) return;
    clearTimeout(liveParseTimer);
    liveParseTimer = null;
  }

  function applyEditorValue(): boolean {
    editorMessage = null;
    if (!editorValue.trim()) {
      selectedSpanId.set(null);
      hoveredSpanId.set(null);
      focusedSpanId.set(null);
      searchQuery.set('');
      searchResults.set([]);
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

  function expandEditor(): void {
    editorCollapsed = false;
    editorCurrentHeight = Math.max(editorExpandedHeight, DEFAULT_EDITOR_HEIGHT_PX);
  }

  function collapseEditor(): void {
    editorExpandedHeight = Math.max(DEFAULT_EDITOR_HEIGHT_PX, editorCurrentHeight);
    editorCollapsed = true;
    editorCurrentHeight = COLLAPSED_EDITOR_HEIGHT_PX;
  }

  function beginEditorResize(event: PointerEvent): void {
    event.preventDefault();
    isEditorResizing = true;
    editorResizeStartY = event.clientY;
    editorResizeStartHeight = editorCurrentHeight;
    document.body.style.cursor = 'ns-resize';
    document.body.style.userSelect = 'none';
  }

  function onWindowPointerMove(event: PointerEvent): void {
    if (!isEditorResizing) return;

    const nextHeight = Math.max(
      COLLAPSED_EDITOR_HEIGHT_PX,
      editorResizeStartHeight + event.clientY - editorResizeStartY
    );

    editorCurrentHeight = nextHeight;

    if (nextHeight > COLLAPSED_EDITOR_HEIGHT_PX + AUTO_EXPAND_EDITOR_DELTA_PX) {
      editorCollapsed = false;
      editorExpandedHeight = Math.max(DEFAULT_EDITOR_HEIGHT_PX, nextHeight);
    } else if (!editorCollapsed) {
      editorExpandedHeight = Math.max(DEFAULT_EDITOR_HEIGHT_PX, nextHeight);
    }
  }

  function endEditorResize(): void {
    if (!isEditorResizing) return;
    isEditorResizing = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }

  function loadEditorText(text: string): void {
    editorValue = text;
    expandEditor();
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
    expandEditor();
    selectedSpanId.set(null);
    hoveredSpanId.set(null);
    focusedSpanId.set(null);
    searchQuery.set('');
    searchResults.set([]);
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
    collapseEditor();
    activeView.set($activeView || 'timeline');
    await tick();
    if ($activeView === 'waterfall') waterfallView?.focusView();
    else if ($activeView === 'timeline') timelineView?.focusView();
    else flameGraphView?.focusView();
  }

  async function loadTraceFromUrl(url: string): Promise<void> {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const text = await response.text();
    loadEditorText(text);
  }

  let globalKeydownHandler: ((e: KeyboardEvent) => void) | null = null;

  onMount(() => {
    globalKeydownHandler = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

      if (e.key === '?' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        showKeyboardHelp = !showKeyboardHelp;
        return;
      }
      if (showKeyboardHelp) {
        if (e.key === 'Escape') { showKeyboardHelp = false; e.preventDefault(); }
        return;
      }

      const mod = e.metaKey || e.ctrlKey;

      if (mod && e.key === 'o') { e.preventDefault(); openEditorFilePicker(); return; }
      if (mod && e.key === 'k') {
        e.preventDefault();
        const searchInput = document.querySelector<HTMLInputElement>('.search-input');
        searchInput?.focus();
        return;
      }
      if (mod && e.key === 'Enter') { e.preventDefault(); void submitEditor(); return; }
      if (mod && e.key === 'v') { e.preventDefault(); void pasteFromClipboard(); return; }

      if (!mod && e.key >= '1' && e.key <= '5') {
        e.preventDefault();
        const views: Array<'flame' | 'timeline' | 'waterfall' | 'graph' | 'diff'> = ['flame', 'timeline', 'waterfall', 'graph', 'diff'];
        activeView.set(views[parseInt(e.key) - 1]);
        localStorage.setItem(STORAGE_KEY_VIEW, views[parseInt(e.key) - 1]);
        return;
      }
    };
    document.addEventListener('keydown', globalKeydownHandler);
  });

  $: state = $traceState;
  $: warnings = state.summary?.warnings ?? [];
  $: initWarnings = wasmReady ? getInitWarnings() : [];
  $: allWarnings = [...initWarnings, ...warnings];
  $: localStorage.setItem(STORAGE_KEY_THEME, $theme);
  $: localStorage.setItem(STORAGE_KEY_VIEW, $activeView);
  $: {
    try {
      if (editorValue.trim()) {
        localStorage.setItem(STORAGE_KEY_EDITOR, editorValue);
      }
    } catch { /* ignore */ }
  }

  $: if (state.status === 'error' && state.error?.context && editorInputEl) {
    const line = state.error.context.line;
    if (typeof line === 'number' && line > 0) {
      const lines = editorValue.substring(0, editorValue.split('\n').slice(0, line - 1).join('\n').length + (line > 1 ? 1 : 0)).split('\n');
      let charOffset = 0;
      for (let i = 0; i < line - 1 && i < lines.length; i++) {
        charOffset += lines[i].length + 1;
      }
      editorInputEl.focus();
      editorInputEl.setSelectionRange(charOffset, charOffset);
      const lineHeight = 20;
      editorInputEl.scrollTop = Math.max(0, (line - 1) * lineHeight - 60);
    }
  }
</script>

<svelte:window on:pointermove={onWindowPointerMove} on:pointerup={endEditorResize} on:pointercancel={endEditorResize} />

<div class="app" data-theme={$theme}>
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
        {#if !editorValue.trim()}
          <section class="welcome-panel" aria-labelledby="welcome-title">
            <div class="welcome-copy">
              <div class="eyebrow">Trace explorer</div>
              <h1 id="welcome-title">Explore distributed traces locally</h1>
              <p>
                Drop OTLP, Jaeger, or OpenInference JSON and inspect spans, timelines, errors, and LLM calls in your browser.
              </p>
              <div class="privacy-note">Files stay local in your browser.</div>
            </div>

            <div class="welcome-actions" aria-label="Load trace actions">
              <button type="button" class="welcome-btn welcome-btn--primary" on:click={loadSampleJson}>
                Load sample trace
              </button>
              <button type="button" class="welcome-btn" on:click={openEditorFilePicker}>
                Open file
              </button>
              <button type="button" class="welcome-btn" on:click={pasteFromClipboard}>
                Paste JSON
              </button>
            </div>

            <div class="format-row" aria-label="Supported formats">
              <span>Supports</span>
              <span class="format-chip">OTLP JSON</span>
              <span class="format-chip">Jaeger JSON</span>
              <span class="format-chip">OpenInference</span>
              <span class="drop-hint">Drag and drop a file anywhere</span>
            </div>
          </section>
        {/if}

        <section
          class="editor-panel"
          class:editor-panel--collapsed={editorCollapsed}
          class:editor-panel--empty={!editorValue.trim()}
        >
          <div class="editor-header">
            <div class="editor-copy">
              <div class="editor-title">{editorValue.trim() ? 'Trace JSON input' : 'Paste JSON manually'}</div>
              <div class="editor-subtitle">
                {editorValue.trim()
                  ? 'Paste formatted or unformatted JSON, then submit it or let the graphs update live while you type.'
                  : 'Prefer the raw payload? Paste trace JSON here and submit it when ready.'}
              </div>
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

          <div class="editor-input-shell">
            <textarea
              class="editor-input"
              class:editor-input--collapsed={editorCollapsed}
              bind:this={editorInputEl}
              bind:value={editorValue}
              on:input={onEditorInput}
              on:keydown={onEditorKeyDown}
              placeholder="Paste a trace JSON payload here…"
              spellcheck="false"
              aria-label="Trace JSON input"
              style={`height: ${editorValue.trim() ? editorCurrentHeight : EMPTY_EDITOR_HEIGHT_PX}px;`}
            ></textarea>
            {#if editorCollapsed && editorValue.trim()}
              <button
                type="button"
                class="editor-expand-btn"
                aria-label="Expand trace JSON input"
                on:click={expandEditor}
              >
                Expand editor
              </button>
            {/if}
            <div
              class="editor-resize-handle"
              class:editor-resize-handle--active={isEditorResizing}
              role="separator"
              aria-label="Resize trace JSON input"
              aria-orientation="horizontal"
              on:pointerdown={beginEditorResize}
            ></div>
          </div>

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
              {#if $activeView === 'timeline' && state.timelineLayout}
                <Timeline bind:this={timelineView} layout={state.timelineLayout} />
              {:else if $activeView === 'waterfall' && state.waterfallLayout}
                <WaterfallView bind:this={waterfallView} layout={state.waterfallLayout} />
              {:else if $activeView === 'graph' && state.serviceGraph}
                <ServiceGraph graph={state.serviceGraph} />
              {:else if $activeView === 'diff'}
                <DiffView />
              {:else}
                <FlameGraph bind:this={flameGraphView} layout={state.flameLayout} />
              {/if}
              {#if $activeView !== 'diff'}
                <SpanDetail />
              {/if}
            {:else if state.status === 'error'}
              <div class="empty-state">
                <div class="empty-icon">⚠️</div>
                <div class="empty-title">Could not parse trace</div>
                <div class="empty-sub">Update the JSON above and the flame graph and timeline view will refresh when the payload becomes valid.</div>
                {#if state.error?.code === 'INVALID_JSON' && state.error?.context}
                  <div class="error-context">
                    {#if state.error.context.line !== undefined && state.error.context.line !== null}
                      Line {state.error.context.line}{state.error.context.column !== undefined && state.error.context.column !== null ? `, column ${state.error.context.column}` : ''}
                    {/if}
                  </div>
                {/if}
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
    {#if showKeyboardHelp}
      <KeyboardHelp on:close={() => (showKeyboardHelp = false)} />
    {/if}
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
    --color-panel-highlight: rgba(255, 255, 255, 0.04);
    --color-panel-subtle: rgba(255, 255, 255, 0.05);
    --color-badge-bg: rgba(59, 130, 246, 0.2);
    --color-badge-text: #93c5fd;
    --color-llm-panel-bg: rgba(139, 92, 246, 0.07);
    --color-llm-badge-bg: rgba(139, 92, 246, 0.25);
    --color-llm-badge-text: #c4b5fd;
    --color-link: #93c5fd;
    --color-danger: #f87171;
    --color-success: #4ade80;
    --color-code-text: #e2e8f0;
    --color-code-muted: #cbd5e1;
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
    --color-panel-highlight: rgba(15, 23, 42, 0.03);
    --color-panel-subtle: rgba(15, 23, 42, 0.05);
    --color-badge-bg: rgba(59, 130, 246, 0.12);
    --color-badge-text: #1d4ed8;
    --color-llm-panel-bg: rgba(139, 92, 246, 0.08);
    --color-llm-badge-bg: rgba(139, 92, 246, 0.14);
    --color-llm-badge-text: #6d28d9;
    --color-link: #1d4ed8;
    --color-danger: #dc2626;
    --color-success: #15803d;
    --color-code-text: #0f172a;
    --color-code-muted: #334155;
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

  .welcome-panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1.25rem;
    align-items: center;
    padding: 1.5rem;
    border: 1px solid color-mix(in srgb, var(--color-accent, #3b82f6) 28%, var(--color-border, #334155));
    border-radius: 18px;
    background:
      radial-gradient(circle at top left, color-mix(in srgb, var(--color-accent, #3b82f6) 18%, transparent), transparent 34rem),
      linear-gradient(135deg, var(--color-surface, #1e293b), color-mix(in srgb, var(--color-surface, #1e293b) 86%, var(--color-accent, #3b82f6)));
    box-shadow: 0 18px 50px rgba(15, 23, 42, 0.16);
  }

  .welcome-copy {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    max-width: 760px;
  }

  .eyebrow {
    color: var(--color-badge-text, #93c5fd);
    font-size: 0.75rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .welcome-copy h1 {
    color: var(--color-text, #e2e8f0);
    font-size: clamp(1.65rem, 3vw, 2.55rem);
    line-height: 1.05;
    letter-spacing: -0.045em;
  }

  .welcome-copy p {
    color: var(--color-text-muted, #94a3b8);
    font-size: 1rem;
    line-height: 1.55;
  }

  .privacy-note {
    width: fit-content;
    padding: 0.35rem 0.6rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 999px;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    color: var(--color-text, #e2e8f0);
    font-size: 0.82rem;
    font-weight: 700;
  }

  .welcome-actions {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-width: 180px;
  }

  .welcome-btn {
    padding: 0.75rem 1rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 11px;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    color: var(--color-text, #e2e8f0);
    font-size: 0.92rem;
    font-weight: 800;
    cursor: pointer;
    text-align: center;
    transition: transform 0.14s ease, border-color 0.14s ease, background 0.14s ease, box-shadow 0.14s ease;
  }

  .welcome-btn:hover {
    transform: translateY(-1px);
    border-color: var(--color-accent, #3b82f6);
    background: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .welcome-btn--primary {
    border-color: transparent;
    background: linear-gradient(135deg, var(--color-accent, #3b82f6), color-mix(in srgb, var(--color-accent, #3b82f6) 68%, #8b5cf6));
    color: #fff;
    box-shadow: 0 12px 28px color-mix(in srgb, var(--color-accent, #3b82f6) 30%, transparent);
  }

  .welcome-btn--primary:hover {
    background: linear-gradient(135deg, var(--color-accent-hover, #2563eb), color-mix(in srgb, var(--color-accent-hover, #2563eb) 62%, #8b5cf6));
  }

  .format-row {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.82rem;
  }

  .format-chip,
  .drop-hint {
    padding: 0.28rem 0.55rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 999px;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    color: var(--color-text, #e2e8f0);
    font-weight: 700;
  }

  .drop-hint {
    margin-left: auto;
    color: var(--color-text-muted, #94a3b8);
    font-weight: 600;
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
    transition: padding 0.16s ease, gap 0.16s ease;
  }

  .editor-panel--collapsed {
    gap: 0.5rem;
    padding: 0.875rem 1rem;
  }

  .editor-panel--empty {
    border-style: dashed;
    box-shadow: none;
    opacity: 0.94;
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

  .editor-panel--collapsed .editor-subtitle {
    display: none;
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

  .editor-input-shell {
    position: relative;
    padding-bottom: 12px;
  }

  .editor-input {
    width: 100%;
    min-height: 88px;
    resize: none;
    border: 1px solid var(--color-border, #334155);
    border-radius: 10px;
    padding: 1rem;
    background: var(--color-bg, #0f172a);
    color: var(--color-text, #e2e8f0);
    font: 0.875rem/1.6 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    outline: none;
    transition: min-height 0.16s ease, max-height 0.16s ease, padding 0.16s ease;
  }

  .editor-input:focus {
    border-color: var(--color-accent, #3b82f6);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.16);
  }

  .editor-input--collapsed {
    padding-right: 8.5rem;
  }

  .editor-expand-btn {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    padding: 0.45rem 0.75rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-surface, #1e293b) 88%, transparent);
    color: var(--color-text, #e2e8f0);
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
    opacity: 0;
    pointer-events: none;
    transform: translateY(-4px);
    transition: opacity 0.14s ease, transform 0.14s ease, border-color 0.12s ease, background 0.12s ease;
  }

  .editor-input-shell:hover .editor-expand-btn,
  .editor-input-shell:focus-within .editor-expand-btn {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(0);
  }

  .editor-expand-btn:hover {
    border-color: var(--color-accent, #3b82f6);
    background: color-mix(in srgb, var(--color-panel-highlight, rgba(255, 255, 255, 0.04)) 92%, transparent);
  }

  .editor-resize-handle {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 12px;
    cursor: ns-resize;
  }

  .editor-resize-handle::before {
    content: '';
    position: absolute;
    left: 50%;
    bottom: 4px;
    transform: translateX(-50%);
    width: 72px;
    height: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-text-muted, #94a3b8) 45%, transparent);
    transition: background 0.12s ease, width 0.12s ease;
  }

  .editor-input-shell:hover .editor-resize-handle::before,
  .editor-resize-handle--active::before {
    width: 108px;
    background: color-mix(in srgb, var(--color-accent, #3b82f6) 70%, transparent);
  }

  .editor-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .editor-panel--collapsed .editor-footer {
    display: none;
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
    position: relative;
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

  .error-context {
    margin-top: 0.5rem;
    padding: 0.35rem 0.65rem;
    border: 1px solid var(--color-error-border, #fca5a5);
    border-radius: 6px;
    background: var(--color-error-bg, #fee2e2);
    color: var(--color-error-text, #991b1b);
    font-family: monospace;
    font-size: 0.8125rem;
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

  @media (max-width: 820px) {
    .app,
    .layout {
      min-height: 100vh;
      height: auto;
      overflow: auto;
    }

    .main {
      overflow: visible;
      padding: 0.65rem;
    }

    .welcome-panel {
      grid-template-columns: 1fr;
      padding: 1.15rem;
      border-radius: 14px;
    }

    .welcome-actions {
      min-width: 0;
    }

    .format-row {
      align-items: flex-start;
    }

    .drop-hint {
      margin-left: 0;
      width: 100%;
    }

    .editor-header,
    .editor-footer {
      align-items: stretch;
    }

    .editor-actions,
    .editor-btn {
      width: 100%;
    }

    .editor-actions {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .editor-btn:last-child {
      grid-column: 1 / -1;
    }
  }

  @media (max-width: 520px) {
    .welcome-copy h1 {
      font-size: 1.75rem;
    }

    .editor-actions {
      grid-template-columns: 1fr;
    }
  }
</style>
