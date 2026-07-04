<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { loadWasm, getInitWarnings, getSpanDetail } from './lib/wasm';
  import { openFilePicker, handleFile, handleRawInputAsync } from './lib/input';
  import { parsePermalink, decodeTrace } from './lib/permalink';
  import { connectLive, disconnectLive } from './lib/live';
  import { SAMPLE_TRACE } from './lib/sample';
  import { traceState } from './stores/trace';
  import { theme } from './lib/theme';
  import { flyUpConfig, viewSlideIn, viewSlideOut } from './lib/animation';

  import Toolbar from './components/Toolbar.svelte';
  import FlameGraph from './components/FlameGraph.svelte';
  import Timeline from './components/Timeline.svelte';
  import WaterfallView from './components/WaterfallView.svelte';
  import ServiceGraph from './components/ServiceGraph.svelte';
  import AgentFlow from './components/AgentFlow.svelte';
  import AgentTimeline from './components/AgentTimeline.svelte';
  import DiffView from './components/DiffView.svelte';
  import ComparisonTable from './components/ComparisonTable.svelte';
  import TokenTrends from './components/TokenTrends.svelte';
  import DashboardView from './components/DashboardView.svelte';
  import SpanDetail from './components/SpanDetail.svelte';
  import DropZone from './components/DropZone.svelte';
  import ErrorBanner from './components/ErrorBanner.svelte';
  import Footer from './components/Footer.svelte';
  import KeyboardHelp from './components/KeyboardHelp.svelte';
  import { activeView, focusedSpanId, fullscreen, hoveredSpanId, searchQuery, searchResults, selectedSpanId } from './stores/selection';

  let wasmReady = false;
  let wasmError: string | null = null;
  let remoteTraceLoading = false;
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
  let showFeatureHint = false;
  let lastViewIdx = 0;
  let slideDirection: 1 | -1 = 1;

  const STORAGE_KEY_THEME = 'widescope:theme';
  const STORAGE_KEY_VIEW = 'widescope:view';
  const STORAGE_KEY_EDITOR = 'widescope:editor';
  const STORAGE_KEY_HINT_DISMISSED = 'widescope:hint-dismissed';

  function dismissFeatureHint() {
    showFeatureHint = false;
    localStorage.setItem(STORAGE_KEY_HINT_DISMISSED, '1');
  }

  $: isEmbedded = new URLSearchParams(window.location.search).get('embed') === '1';

  const VIEW_ORDER: Array<'flame' | 'timeline' | 'waterfall' | 'graph' | 'agent' | 'diff' | 'analytics' | 'matrix' | 'dashboard'> = ['waterfall', 'flame', 'timeline', 'graph', 'agent', 'diff', 'analytics', 'matrix', 'dashboard'];

  let agentSubview: 'flow' | 'timeline' = 'flow';

  $: {
    const currentIdx = VIEW_ORDER.indexOf($activeView);
    slideDirection = currentIdx >= lastViewIdx ? 1 : -1;
    lastViewIdx = currentIdx;
  }

  const LIVE_PARSE_DELAY_MS = 150;
  const DEFAULT_EDITOR_HEIGHT_PX = 280;
  const EMPTY_EDITOR_HEIGHT_PX = 160;
  const COLLAPSED_EDITOR_HEIGHT_PX = 88;
  const AUTO_EXPAND_EDITOR_DELTA_PX = 24;

  onMount(async () => {
    const storedTheme = localStorage.getItem(STORAGE_KEY_THEME);
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.apply(storedTheme === 'dark' ? 'dark' : storedTheme === 'light' ? 'light' : (prefersDark ? 'dark' : 'light'));

    showFeatureHint = !localStorage.getItem(STORAGE_KEY_HINT_DISMISSED);

    const storedView = localStorage.getItem(STORAGE_KEY_VIEW);
    if (storedView === 'flame' || storedView === 'timeline' || storedView === 'waterfall' || storedView === 'graph' || storedView === 'agent' || storedView === 'diff' || storedView === 'analytics' || storedView === 'matrix' || storedView === 'dashboard') {
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

    const permalink = parsePermalink();
    let permalinkLoaded = false;
    if (permalink.traceData) {
      try {
        await loadEditorText(await decodeTrace(permalink.traceData));
        permalinkLoaded = true;
      } catch {
        editorMessage = 'Failed to load the shared trace from the link.';
      }
    } else if (permalink.traceUrl) {
      remoteTraceLoading = true;
      try {
        await loadTraceFromUrl(permalink.traceUrl);
        permalinkLoaded = true;
      } catch {
        editorMessage = 'Failed to load trace from URL.';
      } finally {
        remoteTraceLoading = false;
      }
    }

    if (permalinkLoaded) {
      if (permalink.view) activeView.set(permalink.view);
      if (permalink.spanId) applyPermalinkSpan(permalink.spanId);
    } else if (new URLSearchParams(window.location.search).get('sample') === '1') {
      // Landing-page "Try the sample trace" CTA deep link.
      loadSampleJson();
    }

    // Host bridge: an embedding host (e.g. the VS Code extension) posts trace
    // text in via postMessage instead of a URL, so big files dodge URL limits.
    // Announce readiness so the host waits for the listener (WASM load is async).
    window.addEventListener('message', hostMessageHandler);
    if (window.parent !== window) window.parent.postMessage({ type: 'widescope:ready' }, '*');

    // Live mode: stream traces from an SSE relay and auto-select each newest one.
    // ponytail: every trace runs the full editor load — fine for Phase 1 cadence.
    const liveUrl = new URLSearchParams(window.location.search).get('live');
    if (liveUrl) connectLive(liveUrl, (json) => { void loadEditorText(json); });
  });

  function hostMessageHandler(event: MessageEvent): void {
    // Only the embedding parent drives this bridge; ignore any other frame.
    // (The host webview origin isn't statically known, so gate on source.)
    if (event.source !== window.parent) return;
    const data = event.data;
    if (data && data.type === 'widescope:load' && typeof data.text === 'string') {
      void loadEditorText(data.text);
    }
  }

  /** Pre-select a span from a share link, ignoring it if absent in the trace. */
  function applyPermalinkSpan(spanId: string): void {
    try {
      getSpanDetail(spanId);
      selectedSpanId.set(spanId);
    } catch {
      // Span id not present in this trace — leave nothing selected.
    }
  }

  onDestroy(() => {
    disconnectLive();
    clearLiveParseTimer();
    editorResizeObserver?.disconnect();
    window.removeEventListener('message', hostMessageHandler);
    if (globalKeydownHandler) document.removeEventListener('keydown', globalKeydownHandler);
  });

  function clearLiveParseTimer(): void {
    if (liveParseTimer === null) return;
    clearTimeout(liveParseTimer);
    liveParseTimer = null;
  }

  async function applyEditorValue(showLoading = false): Promise<boolean> {
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
    return await handleRawInputAsync(editorValue, false, showLoading);
  }

  function scheduleLiveParse(): void {
    clearLiveParseTimer();
    liveParseTimer = setTimeout(() => {
      liveParseTimer = null;
      void applyEditorValue(false);
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

  async function loadEditorText(text: string): Promise<boolean> {
    editorValue = text;
    expandEditor();
    clearLiveParseTimer();
    return await applyEditorValue(true);
  }

  function openEditorFilePicker(): void {
    openFilePicker((text) => { void loadEditorText(text); });
  }

  function loadSampleJson(): void {
    void loadEditorText(SAMPLE_TRACE);
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
    void handleFile(file, (text) => { void loadEditorText(text); });
  }

  async function pasteFromClipboard(): Promise<void> {
    editorMessage = null;
    try {
      const text = await navigator.clipboard.readText();
      if (!text.trim()) return;
      await loadEditorText(text);
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
      void applyEditorValue(false);
    } catch {
      editorMessage = 'Input is not valid JSON, so it could not be formatted.';
    }
  }

  async function submitEditor(): Promise<void> {
    clearLiveParseTimer();
    const parsed = await applyEditorValue(true);
    if (!parsed) return;
    collapseEditor();
    activeView.set($activeView || VIEW_ORDER[0]);
    await tick();
    if ($activeView === 'waterfall') waterfallView?.focusView();
    else if ($activeView === 'timeline') timelineView?.focusView();
    else flameGraphView?.focusView();
  }

  async function loadTraceFromUrl(url: string): Promise<void> {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const text = await response.text();
    await loadEditorText(text);
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

      if (!mod && e.key >= '1' && e.key <= '9') {
        e.preventDefault();
        const views: Array<'flame' | 'timeline' | 'waterfall' | 'graph' | 'agent' | 'diff' | 'analytics' | 'matrix' | 'dashboard'> = ['flame', 'timeline', 'waterfall', 'graph', 'agent', 'diff', 'analytics', 'matrix', 'dashboard'];
        activeView.set(views[parseInt(e.key) - 1]);
        localStorage.setItem(STORAGE_KEY_VIEW, views[parseInt(e.key) - 1]);
        return;
      }

      if (e.key === 'F' && e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        fullscreen.update(v => !v);
        return;
      }

      if ($fullscreen && e.key === 'Escape') {
        e.preventDefault();
        fullscreen.set(false);
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

<div class="app" class:app--fullscreen={$fullscreen} data-theme={$theme}>
  {#if wasmError}
    <div class="fatal-error">
      <h2>Failed to initialize WideScope</h2>
      <pre>{wasmError}</pre>
      <p>Please try refreshing the page. If the issue persists, check that your browser supports WebAssembly.</p>
    </div>
  {:else if !wasmReady || remoteTraceLoading}
    <div class="splash">
      <div class="splash-inner">
        <span class="splash-ring" aria-hidden="true"></span>
        <img class="splash-logo" src="/widescope-logo.svg" alt="" width="64" height="64" />
        <span class="splash-name">WideScope</span>
        <span class="splash-loading">{remoteTraceLoading ? 'fetching trace…' : 'initializing wasm…'}</span>
      </div>
    </div>
  {:else}
    <div class="layout">
      {#if !isEmbedded && !$fullscreen}
        <Toolbar onOpenFile={openEditorFilePicker} />
      {/if}
      <ErrorBanner
        error={state.status === 'error' ? state.error : null}
        warnings={allWarnings}
        isSample={state.isSampleTrace}
      />
      <div class="main">
        {#if !editorValue.trim() && !isEmbedded}
          <section class="welcome-panel" aria-labelledby="welcome-title" transition:fly={flyUpConfig()}>
            <div class="welcome-copy">
              <div class="eyebrow">span 00 · trace.load <em>// local-first</em></div>
              <h1 id="welcome-title">Put your trace <span class="lens">under the lens.</span></h1>
              <p>
                Drop OTLP, Jaeger, or OpenInference JSON and inspect spans, timelines, errors,
                token costs, and LLM calls — parsed in your browser, never uploaded.
              </p>
              <div class="welcome-flames" aria-hidden="true">
                <i style="--w:92%;--d:.05s"></i>
                <div class="wf-r"><i style="--w:30%;--d:.18s"></i><i class="hot" style="--w:24%;--d:.28s"></i><i style="--w:30%;--d:.38s"></i></div>
                <div class="wf-r"><i style="--w:16%;--d:.46s"></i><i style="--w:20%;--d:.54s"></i><i class="err" style="--w:7%;--d:.62s"></i><i class="hot" style="--w:14%;--d:.7s"></i></div>
              </div>
            </div>

            <div class="welcome-actions" aria-label="Load trace actions">
              <button type="button" class="welcome-btn welcome-btn--primary" on:click={loadSampleJson}>
                Load sample trace
              </button>
              <button type="button" class="welcome-btn" on:click={openEditorFilePicker}>
                Open file <kbd>⌘O</kbd>
              </button>
              <button type="button" class="welcome-btn" on:click={pasteFromClipboard}>
                Paste JSON <kbd>⌘V</kbd>
              </button>
            </div>

            <div class="format-row" aria-label="Supported formats">
              <span class="format-chip"><b>OTLP</b> resourceSpans</span>
              <span class="format-chip"><b>Jaeger</b> data[].spans</span>
              <span class="format-chip"><b>OpenInference</b> spans</span>
              <span class="privacy-note">⬢ no backend · no upload · no telemetry</span>
              <a class="format-chip format-chip--link" href="/docs/">No trace yet? Get one →</a>
              <span class="drop-hint">drag a .json anywhere</span>
            </div>
          </section>
        {/if}

        {#if !isEmbedded && !$fullscreen}
          <section
            class="editor-panel"
          class:editor-panel--collapsed={editorCollapsed}
          class:editor-panel--empty={!editorValue.trim()}
        >
          <div class="editor-header">
            <div class="editor-copy">
              <div class="editor-dots" aria-hidden="true"><i></i><i></i><i></i></div>
              <div class="editor-titles">
                <div class="editor-title">{editorValue.trim() ? 'trace.json — editor' : 'trace.json — paste payload'}</div>
                <div class="editor-subtitle">
                  {editorValue.trim()
                    ? 'Live parse is on — graphs refresh as you type, or submit to collapse the editor.'
                    : 'Prefer the raw payload? Paste trace JSON here and submit it when ready.'}
                </div>
              </div>
            </div>
            <div class="editor-actions">
              <button type="button" class="editor-btn editor-btn--ghost" on:click={clearEditorJson} disabled={!editorValue.trim()}>
                Clear
              </button>
              <button type="button" class="editor-btn editor-btn--ghost" on:click={loadSampleJson}>
                Sample
              </button>
              <button type="button" class="editor-btn editor-btn--ghost" on:click={pasteFromClipboard}>
                Paste
              </button>
              <button type="button" class="editor-btn editor-btn--ghost" on:click={formatEditorJson} disabled={!editorValue.trim()}>
                Format
              </button>
              <button type="button" class="editor-btn" on:click={submitEditor} disabled={!editorValue.trim()}>
                Submit <kbd>⌘⏎</kbd>
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
            <span class="editor-hint">OTLP JSON · Jaeger JSON · OpenInference JSON — or hit <b>Sample</b> to try the built-in trace</span>
            {#if editorMessage}
              <span class="editor-message">{editorMessage}</span>
            {/if}
          </div>
        </section>
        {/if}

        {#if editorValue.trim()}
          <div class="workspace">
            {#if showFeatureHint}
              <div class="feature-hint" role="note" transition:fly={{ y: -8, duration: 200 }}>
                <span class="feature-hint-text">
                  Tip: press <button type="button" class="feature-hint-key" on:click={() => (showKeyboardHelp = true)}>?</button> for shortcuts ·
                  try <code>duration&gt;100ms</code> or <code>status=error</code> in search ·
                  compare many runs with matrix view + session grouping.
                </span>
                <button type="button" class="feature-hint-close" aria-label="Dismiss hint" on:click={dismissFeatureHint}>✕</button>
              </div>
            {/if}
            {#if state.status === 'loaded' && state.flameLayout}
              {#if $activeView === 'timeline' && state.timelineLayout}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <Timeline bind:this={timelineView} layout={state.timelineLayout} />
                </div>
              {:else if $activeView === 'waterfall' && state.waterfallLayout}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <WaterfallView bind:this={waterfallView} layout={state.waterfallLayout} />
                </div>
              {:else if $activeView === 'graph' && state.serviceGraph}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <ServiceGraph graph={state.serviceGraph} />
                </div>
              {:else if $activeView === 'agent' && state.agentFlow}
                <div class="view-wrapper agent-view" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <div class="agent-subview-toggle" role="tablist" aria-label="Agent view mode">
                    <button type="button" role="tab" class:active={agentSubview === 'flow'} aria-selected={agentSubview === 'flow'} on:click={() => (agentSubview = 'flow')}>Flow</button>
                    <button type="button" role="tab" class:active={agentSubview === 'timeline'} aria-selected={agentSubview === 'timeline'} on:click={() => (agentSubview = 'timeline')}>Timeline</button>
                  </div>
                  {#if agentSubview === 'timeline'}
                    <AgentTimeline flow={state.agentFlow} />
                  {:else}
                    <AgentFlow flow={state.agentFlow} />
                  {/if}
                </div>
              {:else if $activeView === 'diff'}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <DiffView />
                </div>
              {:else if $activeView === 'analytics'}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <TokenTrends />
                </div>
              {:else if $activeView === 'matrix'}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <ComparisonTable />
                </div>
              {:else if $activeView === 'dashboard'}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <DashboardView />
                </div>
              {:else}
                <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
                  <FlameGraph bind:this={flameGraphView} layout={state.flameLayout} />
                </div>
              {/if}
              {#if $activeView !== 'diff' && $activeView !== 'analytics' && $activeView !== 'matrix' && $activeView !== 'dashboard'}
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
                <div class="empty-title">{state.loadingPhase ?? 'Parsing trace JSON'}</div>
                <div class="loading-progress" aria-label="Trace loading progress">
                  <div class="loading-progress-fill" style={`width: ${state.loadingProgress ?? 20}%;`}></div>
                </div>
                <div class="empty-sub">Large traces may take a few seconds while layouts are prepared.</div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
      {#if !isEmbedded && !$fullscreen}
        <Footer />
      {/if}
    </div>
    {#if $fullscreen}
      <button class="fullscreen-exit" aria-label="Exit fullscreen" on:click={() => fullscreen.set(false)}>
        ⊠
      </button>
    {/if}
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
    font-family: var(--font-display, system-ui, sans-serif);
    font-optical-sizing: auto;
    font-size: 14px;
    -webkit-font-smoothing: antialiased;
    text-rendering: optimizeLegibility;
  }

  :global(::selection) {
    background: #fcd34d;
    color: #1a1203;
  }

  :global(:root) {
    --font-display: 'Bricolage Grotesque', system-ui, -apple-system, sans-serif;
    --font-mono: 'Spline Sans Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
    --ease-spring: cubic-bezier(0.25, 0.1, 0.25, 1);
    --ease-bounce: cubic-bezier(0.34, 1.56, 0.64, 1);
    --ease-smooth: cubic-bezier(0.4, 0, 0.2, 1);
    --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  }

  :global([data-theme='dark']) {
    color-scheme: dark;
    --color-bg: #05080f;
    --color-surface: #0d1626;
    --color-toolbar: #070c16;
    --color-toolbar-text: #e9eff8;
    --color-toolbar-muted: #8b9cb5;
    --color-border: rgba(125, 211, 252, 0.13);
    --color-border-soft: rgba(125, 211, 252, 0.07);
    --color-text: #e9eff8;
    --color-text-muted: #9aa8bd;
    --color-text-faint: #5b6b84;
    --color-accent: #3b82f6;
    --color-accent-hover: #2563eb;
    --color-sky: #7dd3fc;
    --color-ice: #bae6fd;
    --color-gold: #fcd34d;
    --color-amber: #f59e0b;
    --color-violet: #a78bfa;
    --color-canvas-bg: #070c16;
    --color-sidebar: #0a111e;
    --color-sidebar-text: #e9eff8;
    --color-panel-highlight: rgba(125, 211, 252, 0.05);
    --color-panel-subtle: rgba(125, 211, 252, 0.06);
    --color-badge-bg: rgba(59, 130, 246, 0.16);
    --color-badge-text: #7dd3fc;
    --color-llm-panel-bg: rgba(245, 158, 11, 0.06);
    --color-llm-badge-bg: rgba(245, 158, 11, 0.16);
    --color-llm-badge-text: #fcd34d;
    --color-link: #7dd3fc;
    --color-danger: #f87171;
    --color-success: #34d399;
    --color-code-text: #e9eff8;
    --color-code-muted: #b9c5d8;
    --color-error-bg: rgba(69, 10, 10, 0.65);
    --color-error-text: #fca5a5;
    --color-error-border: rgba(153, 27, 27, 0.8);
    --color-warning-bg: rgba(69, 26, 3, 0.6);
    --color-warning-text: #fcd34d;
    --color-warning-border: rgba(146, 64, 14, 0.8);
    --focus-color: #7dd3fc;
    --grad-cta: linear-gradient(135deg, #1d4ed8 0%, #3b82f6 55%, #38bdf8 100%);
    --grad-span: linear-gradient(180deg, #7dd3fc 0%, #3b82f6 100%);
    --grad-hot: linear-gradient(180deg, #fcd34d 0%, #f59e0b 100%);
    --shadow-panel: 0 24px 70px -28px rgba(2, 6, 18, 0.9);
    --shadow-cta: 0 1px 0 rgba(255, 255, 255, 0.28) inset, 0 10px 32px -8px rgba(37, 99, 235, 0.55);
    --atmo-grid: rgba(125, 211, 252, 0.04);
    --atmo-glow: rgba(29, 78, 216, 0.22);
  }

  :global([data-theme='light']) {
    color-scheme: light;
    --color-bg: #eef3fa;
    --color-surface: #ffffff;
    --color-toolbar: #fbfdff;
    --color-toolbar-text: #0b1b33;
    --color-toolbar-muted: #5a6b85;
    --color-border: rgba(13, 49, 105, 0.14);
    --color-border-soft: rgba(13, 49, 105, 0.08);
    --color-text: #0b1b33;
    --color-text-muted: #51627d;
    --color-text-faint: #8294ad;
    --color-accent: #2563eb;
    --color-accent-hover: #1d4ed8;
    --color-sky: #0284c7;
    --color-ice: #075985;
    --color-gold: #b45309;
    --color-amber: #d97706;
    --color-violet: #7c3aed;
    --color-canvas-bg: #f5f8fd;
    --color-sidebar: #ffffff;
    --color-sidebar-text: #0b1b33;
    --color-panel-highlight: rgba(13, 49, 105, 0.035);
    --color-panel-subtle: rgba(13, 49, 105, 0.05);
    --color-badge-bg: rgba(37, 99, 235, 0.1);
    --color-badge-text: #1d4ed8;
    --color-llm-panel-bg: rgba(245, 158, 11, 0.08);
    --color-llm-badge-bg: rgba(245, 158, 11, 0.16);
    --color-llm-badge-text: #92400e;
    --color-link: #1d4ed8;
    --color-danger: #dc2626;
    --color-success: #047857;
    --color-code-text: #0b1b33;
    --color-code-muted: #33425c;
    --color-error-bg: #fee2e2;
    --color-error-text: #991b1b;
    --color-error-border: #fca5a5;
    --color-warning-bg: #fef3c7;
    --color-warning-text: #92400e;
    --color-warning-border: #fcd34d;
    --focus-color: #2563eb;
    --grad-cta: linear-gradient(135deg, #1d4ed8 0%, #3b82f6 55%, #38bdf8 100%);
    --grad-span: linear-gradient(180deg, #38bdf8 0%, #2563eb 100%);
    --grad-hot: linear-gradient(180deg, #fbbf24 0%, #d97706 100%);
    --shadow-panel: 0 20px 50px -24px rgba(13, 49, 105, 0.18);
    --shadow-cta: 0 1px 0 rgba(255, 255, 255, 0.3) inset, 0 10px 28px -8px rgba(37, 99, 235, 0.45);
    --atmo-grid: rgba(13, 49, 105, 0.045);
    --atmo-glow: rgba(59, 130, 246, 0.1);
  }

  @media (prefers-reduced-motion: reduce) {
    :global(*) { animation: none !important; transition: none !important; }
  }

  :global(:focus-visible) {
    outline: 2px solid var(--focus-color, #7dd3fc);
    outline-offset: 2px;
  }

  :global(::-webkit-scrollbar) {
    width: 10px;
    height: 10px;
  }

  :global(::-webkit-scrollbar-thumb) {
    background: color-mix(in srgb, var(--color-text-faint, #5b6b84) 42%, transparent);
    border-radius: 8px;
    border: 2px solid transparent;
    background-clip: padding-box;
  }

  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }

  .app {
    height: 100vh;
    overflow: hidden;
    background: var(--color-bg, #05080f);
    color: var(--color-text, #e9eff8);
    display: flex;
    flex-direction: column;
    position: relative;
  }

  /* graph-paper grid + brand glow, sitting behind every panel */
  .app::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    background-image:
      linear-gradient(var(--atmo-grid) 1px, transparent 1px),
      linear-gradient(90deg, var(--atmo-grid) 1px, transparent 1px);
    background-size: 56px 56px;
    mask-image: radial-gradient(ellipse 95% 70% at 50% 0%, #000 0%, transparent 80%);
  }

  .app::after {
    content: '';
    position: absolute;
    top: -360px;
    left: 50%;
    transform: translateX(-50%);
    width: 1100px;
    height: 620px;
    border-radius: 50%;
    z-index: 0;
    pointer-events: none;
    background: radial-gradient(closest-side, var(--atmo-glow), transparent 72%);
    filter: blur(30px);
  }

  .layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    position: relative;
    z-index: 1;
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
    gap: 1.5rem;
    align-items: center;
    padding: 2rem 2.2rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 18px;
    background:
      radial-gradient(ellipse 60% 120% at 8% 0%, color-mix(in srgb, var(--color-accent, #3b82f6) 14%, transparent), transparent 70%),
      linear-gradient(180deg, color-mix(in srgb, var(--color-surface, #0d1626) 94%, var(--color-accent)), var(--color-surface, #0d1626));
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.12) inset;
    position: relative;
    overflow: hidden;
  }

  .welcome-panel::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(115deg, rgba(186, 230, 253, 0.05) 0%, transparent 30%);
  }

  .welcome-copy {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    max-width: 760px;
    min-width: 0;
  }

  .eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    width: fit-content;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    font-weight: 500;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-sky, #7dd3fc);
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.16));
    border: 1px solid color-mix(in srgb, var(--color-sky, #7dd3fc) 24%, transparent);
    border-radius: 999px;
    padding: 0.32rem 0.85rem;
  }

  .eyebrow::before {
    content: '';
    width: 14px;
    height: 7px;
    border-radius: 2px;
    background: var(--grad-span);
  }

  .eyebrow em {
    font-style: normal;
    color: var(--color-text-faint, #5b6b84);
  }

  .welcome-copy h1 {
    color: var(--color-text, #e9eff8);
    font-size: clamp(1.8rem, 3.4vw, 2.9rem);
    line-height: 1.02;
    letter-spacing: -0.03em;
    font-weight: 750;
  }

  .welcome-copy h1 .lens {
    background: linear-gradient(92deg, #bae6fd 0%, #60a5fa 45%, #38bdf8 100%);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    font-style: italic;
    font-weight: 800;
    padding-right: 0.05em;
  }

  :global([data-theme='light']) .welcome-copy h1 .lens {
    background: linear-gradient(92deg, #1d4ed8 0%, #2563eb 50%, #0284c7 100%);
    -webkit-background-clip: text;
    background-clip: text;
  }

  .welcome-copy p {
    color: var(--color-text-muted, #9aa8bd);
    font-size: 1rem;
    line-height: 1.55;
    max-width: 56ch;
  }

  /* mini flame graph, straight off the landing hero */
  .welcome-flames {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-top: 0.55rem;
    max-width: 420px;
  }

  .welcome-flames i {
    display: block;
    height: 10px;
    border-radius: 3px;
    background: var(--grad-span);
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.18) inset;
    width: var(--w, 40%);
    transform-origin: left center;
    animation: flame-grow 0.8s var(--ease-out) both;
    animation-delay: var(--d, 0s);
  }

  .welcome-flames i.hot { background: var(--grad-hot); }
  .welcome-flames i.err { background: linear-gradient(180deg, #fca5a5, #ef4444); }

  .wf-r {
    display: flex;
    gap: 5px;
  }

  @keyframes flame-grow {
    from { transform: scaleX(0); }
    to { transform: scaleX(1); }
  }

  .privacy-note {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-success, #34d399);
    padding: 0.3rem 0.7rem;
    border: 1px solid color-mix(in srgb, var(--color-success, #34d399) 30%, transparent);
    border-radius: 999px;
    white-space: nowrap;
  }

  .welcome-actions {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-width: 210px;
  }

  .welcome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.8rem 1.1rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 11px;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    color: var(--color-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    font-weight: 500;
    letter-spacing: 0.04em;
    cursor: pointer;
    text-align: center;
    position: relative;
    overflow: hidden;
    transition: transform 0.2s var(--ease-out), border-color 0.2s var(--ease-spring), background 0.2s var(--ease-spring), box-shadow 0.2s var(--ease-spring);
  }

  .welcome-btn kbd {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 0.3rem;
    background: var(--color-panel-subtle);
  }

  .welcome-btn:hover {
    transform: translateY(-2px);
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 40%, transparent);
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
  }

  .welcome-btn--primary {
    border-color: transparent;
    background: var(--grad-cta);
    color: #fff;
    box-shadow: var(--shadow-cta);
  }

  .welcome-btn--primary::before {
    content: '';
    position: absolute;
    top: 0;
    left: -80%;
    width: 50%;
    height: 100%;
    background: linear-gradient(100deg, transparent, rgba(255, 255, 255, 0.32), transparent);
    transform: skewX(-20deg);
    transition: left 0.55s var(--ease-out);
  }

  .welcome-btn--primary:hover {
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.28) inset, 0 16px 40px -10px rgba(37, 99, 235, 0.7);
  }

  .welcome-btn--primary:hover::before {
    left: 130%;
  }

  .format-row {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    color: var(--color-text-muted, #9aa8bd);
    font-size: 0.78rem;
  }

  .format-chip,
  .drop-hint {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    padding: 0.32rem 0.75rem;
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-surface, #0d1626) 60%, transparent);
    color: var(--color-text-muted, #9aa8bd);
  }

  .format-chip b {
    color: var(--color-sky, #7dd3fc);
    font-weight: 500;
  }

  .format-chip--link {
    color: var(--color-sky, #7dd3fc);
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 32%, transparent);
    text-decoration: none;
    transition: border-color 0.2s var(--ease-spring), background 0.2s var(--ease-spring);
  }

  .format-chip--link:hover {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
  }

  .drop-hint {
    margin-left: auto;
    border-style: dashed;
    color: var(--color-text-faint, #5b6b84);
  }

  .editor-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.1rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 14px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface, #0d1626) 96%, var(--color-accent)), var(--color-surface, #0d1626));
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.1) inset;
    transition: padding 0.28s var(--ease-spring), gap 0.28s var(--ease-spring);
  }

  .editor-panel--collapsed {
    gap: 0.5rem;
    padding: 0.875rem 1.1rem;
  }

  .editor-panel--empty {
    border-style: dashed;
    box-shadow: none;
    opacity: 0.96;
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
    align-items: flex-start;
    gap: 0.8rem;
    min-width: 0;
  }

  .editor-dots {
    display: flex;
    gap: 6px;
    padding-top: 5px;
    flex-shrink: 0;
  }

  .editor-dots i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-panel-subtle);
  }

  .editor-dots i:nth-child(1) { background: rgba(248, 113, 113, 0.65); }
  .editor-dots i:nth-child(2) { background: rgba(252, 211, 77, 0.65); }
  .editor-dots i:nth-child(3) { background: rgba(52, 211, 153, 0.65); }

  .editor-titles {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }

  .editor-title {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    color: var(--color-sky, #7dd3fc);
  }

  .editor-subtitle {
    font-size: 0.85rem;
    color: var(--color-text-muted, #9aa8bd);
    max-width: 720px;
  }

  .editor-panel--collapsed .editor-subtitle {
    display: none;
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .editor-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.5rem 0.95rem;
    background: var(--grad-cta);
    color: #fff;
    border: 1px solid transparent;
    border-radius: 9px;
    font-family: var(--font-mono);
    font-size: 0.76rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    box-shadow: var(--shadow-cta);
    transition: transform 0.18s var(--ease-out), box-shadow 0.18s var(--ease-spring), border-color 0.15s var(--ease-spring), color 0.15s var(--ease-spring), opacity 0.15s var(--ease-spring), background 0.15s var(--ease-spring);
  }

  .editor-btn kbd {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    border: 1px solid rgba(255, 255, 255, 0.35);
    border-radius: 4px;
    padding: 0 0.28rem;
    background: rgba(255, 255, 255, 0.12);
  }

  .editor-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.28) inset, 0 14px 34px -10px rgba(37, 99, 235, 0.65);
  }

  .editor-btn:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .editor-btn--ghost {
    background: transparent;
    color: var(--color-text, #e9eff8);
    border-color: var(--color-border, rgba(125, 211, 252, 0.13));
    box-shadow: none;
  }

  .editor-btn--ghost:hover:not(:disabled) {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    box-shadow: none;
  }

  .editor-input-shell {
    position: relative;
    padding-bottom: 12px;
  }

  .editor-input {
    width: 100%;
    min-height: 88px;
    resize: none;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 10px;
    padding: 1rem;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-text, #e9eff8);
    font: 400 0.84rem/1.65 var(--font-mono);
    outline: none;
    transition: min-height 0.28s var(--ease-spring), max-height 0.28s var(--ease-spring), padding 0.28s var(--ease-spring), border-color 0.18s var(--ease-spring), box-shadow 0.18s var(--ease-spring);
  }

  .editor-input::placeholder {
    color: var(--color-text-faint, #5b6b84);
  }

  .editor-input:focus {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent, #3b82f6) 18%, transparent);
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
    transition: opacity 0.18s var(--ease-spring), transform 0.18s var(--ease-bounce), border-color 0.15s var(--ease-spring), background 0.15s var(--ease-spring);
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
    transition: background 0.15s var(--ease-spring), width 0.15s var(--ease-spring);
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
    font-family: var(--font-mono);
    font-size: 0.7rem;
    letter-spacing: 0.05em;
    color: var(--color-text-faint, #5b6b84);
  }

  .editor-hint b {
    color: var(--color-text-muted, #9aa8bd);
    font-weight: 500;
  }

  .editor-message {
    font-family: var(--font-mono);
    font-size: 0.74rem;
    color: var(--color-danger, #f87171);
  }

  .feature-hint {
    position: absolute;
    top: 10px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    max-width: min(680px, calc(100% - 24px));
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--color-text-muted, #9aa8bd);
    background: var(--color-surface, #0d1626);
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 10px;
    box-shadow: var(--shadow-panel), 0 4px 16px rgba(0, 0, 0, 0.25);
  }
  .feature-hint code {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11.5px;
    color: var(--color-text, #e9eff8);
    background: var(--color-border-soft, rgba(125, 211, 252, 0.07));
    padding: 1px 5px;
    border-radius: 5px;
  }
  .feature-hint-key {
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--color-text, #e9eff8);
    background: var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 5px;
    padding: 0 6px;
    cursor: pointer;
  }
  .feature-hint-key:hover { color: var(--color-accent, #3b82f6); }
  .feature-hint-close {
    flex: none;
    color: var(--color-text-faint, #5b6b84);
    background: none;
    border: none;
    font-size: 13px;
    cursor: pointer;
    padding: 2px 4px;
    line-height: 1;
  }
  .feature-hint-close:hover { color: var(--color-text, #e9eff8); }

  .workspace {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
    position: relative;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 16px;
    background: var(--color-canvas-bg, #070c16);
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.08) inset;
  }

  .view-wrapper {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .agent-view {
    flex-direction: column;
  }

  .agent-subview-toggle {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    margin: 0.5rem 0.5rem 0;
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 7px;
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 60%, transparent);
    align-self: flex-start;
  }

  .agent-subview-toggle button {
    background: none;
    border: 0;
    border-radius: 5px;
    padding: 0.2rem 0.8rem;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-text-muted, #9aa8bd);
  }

  .agent-subview-toggle button.active {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.16));
    color: var(--color-sky, #7dd3fc);
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
    font-size: 0.74rem;
    font-family: var(--font-mono);
    letter-spacing: 0.03em;
    max-width: 46ch;
    text-align: center;
    line-height: 1.7;
  }

  .loading-progress {
    width: min(320px, 70vw);
    height: 8px;
    overflow: hidden;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 999px;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
  }

  .loading-progress-fill {
    height: 100%;
    min-width: 8px;
    border-radius: inherit;
    background: linear-gradient(90deg, #1d4ed8, #3b82f6 55%, #38bdf8);
    box-shadow: 0 0 12px rgba(56, 189, 248, 0.55);
    transition: width 0.18s var(--ease-smooth, ease);
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
    background: radial-gradient(ellipse 70% 55% at 50% 38%, #0b1322 0%, #05080f 75%);
  }

  .splash-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.55rem;
    color: #e9eff8;
    position: relative;
  }

  .splash-ring {
    position: absolute;
    top: -28px;
    width: 120px;
    height: 120px;
    border-radius: 50%;
    border: 1px solid rgba(125, 211, 252, 0.35);
    border-top-color: #7dd3fc;
    animation: splash-spin 1.1s linear infinite;
  }

  @keyframes splash-spin {
    to { transform: rotate(360deg); }
  }

  .splash-logo {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    box-shadow: 0 18px 50px -12px rgba(29, 78, 216, 0.6);
  }

  .splash-name {
    margin-top: 1.6rem;
    font-size: 1.55rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    font-family: var(--font-display);
  }

  .splash-loading {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #5b6b84;
    animation: splash-pulse 1.8s ease-in-out infinite;
  }

  @keyframes splash-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

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

  .app--fullscreen .main {
    padding: 0;
  }

  .app--fullscreen .workspace {
    border-radius: 0;
    border: none;
  }

  .fullscreen-exit {
    display: none;
    position: fixed;
    top: 12px;
    right: 12px;
    z-index: 100;
    width: 36px;
    height: 36px;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 1.1rem;
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.15s ease;
  }

  .fullscreen-exit:hover {
    opacity: 1;
  }

  .app--fullscreen .fullscreen-exit {
    display: flex;
  }

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
