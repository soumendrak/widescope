<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { loadWasm, getInitWarnings, getSpanDetail } from './lib/wasm';
  import { installKeyboardRouter } from './lib/keyboard';
  import { viewOrder, resolveView, isViewName } from './lib/views';
  import { openFilePicker, handleFile, handleRawInputAsync } from './lib/input';
  import { parsePermalink, decodeTrace } from './lib/permalink';
  import { SAMPLE_TRACE } from './lib/sample';
  import { traceState } from './stores/trace';
  import { theme, resolveTheme } from './lib/theme';
  import { viewSlideIn, viewSlideOut } from './lib/animation';

  import Toolbar from './components/Toolbar.svelte';
  import DropZone from './components/DropZone.svelte';
  import ErrorBanner from './components/ErrorBanner.svelte';
  import Footer from './components/Footer.svelte';
  import KeyboardHelp from './components/KeyboardHelp.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import Icon from './components/ui/Icon.svelte';
  import WasmBoot from './components/WasmBoot.svelte';
  import Workspace from './components/Workspace.svelte';
  import EditorDrawer from './components/EditorDrawer.svelte';
  import { activeView, focusedSpanId, fullscreen, hoveredSpanId, searchQuery, searchResults, selectedSpanId } from './stores/selection';

  let wasmReady = false;
  let wasmError: string | null = null;
  let editorValue = '';
  let editorMessage: string | null = null;
  let editorCollapsed = false;
  let editorDrawer: EditorDrawer | null = null;
  let workspace: Workspace | null = null;
  let liveParseTimer: ReturnType<typeof setTimeout> | null = null;
  let showKeyboardHelp = false;
  let showPalette = false;
  let lastViewIdx = 0;
  let slideDirection: 1 | -1 = 1;

  const STORAGE_KEY_THEME = 'widescope:theme';
  const STORAGE_KEY_VIEW = 'widescope:view';
  const STORAGE_KEY_EDITOR = 'widescope:editor';

  $: isEmbedded = new URLSearchParams(window.location.search).get('embed') === '1';

  $: hasLlmSpans = ($traceState.summary?.llm_span_count ?? 0) > 0;
  $: VIEW_ORDER = viewOrder(hasLlmSpans);

  $: {
    const currentIdx = VIEW_ORDER.indexOf($activeView);
    slideDirection = currentIdx >= lastViewIdx ? 1 : -1;
    lastViewIdx = currentIdx;
  }

  const LIVE_PARSE_DELAY_MS = 150;

  onMount(async () => {
    theme.apply(resolveTheme(localStorage.getItem(STORAGE_KEY_THEME)));

    const storedView = localStorage.getItem(STORAGE_KEY_VIEW);
    if (isViewName(storedView)) activeView.set(storedView);

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
        await loadEditorText(await decodeTrace(permalink.traceData), false);
        permalinkLoaded = true;
      } catch {
        editorMessage = 'Failed to load the shared trace from the link.';
      }
    } else if (permalink.traceUrl) {
      try {
        await loadTraceFromUrl(permalink.traceUrl);
        permalinkLoaded = true;
      } catch {
        editorMessage = 'Failed to load trace from URL.';
      }
    }

    if (permalinkLoaded) {
      if (permalink.view) activeView.set(permalink.view);
      if (permalink.spanId) applyPermalinkSpan(permalink.spanId);
    } else if (new URLSearchParams(window.location.search).get('sample') === '1') {
      // Landing-page "Try the sample trace" CTA deep link.
      loadSampleJson(false);
    }
  });

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
    clearLiveParseTimer();
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

  /** Focus the stage after a trace lands, defaulting to the first view. */
  async function focusActiveView(): Promise<void> {
    activeView.set(resolveView($activeView || VIEW_ORDER[0], hasLlmSpans));
    await tick();
    workspace?.focusActiveLens();
  }

  async function loadEditorText(text: string, moveFocus = true): Promise<boolean> {
    editorValue = text;
    clearLiveParseTimer();
    const parsed = await applyEditorValue(true);
    // ponytail: collapse only for load actions (sample/file/paste/link), never
    // mid-typing — live parse deliberately leaves the editor where it is.
    if (parsed) {
      editorDrawer?.collapse();
      // Focus follows a click, never a page load: focusing the stage during
      // boot put the caret past the skip link, which is meant to be the first
      // tab stop.
      if (moveFocus) await focusActiveView();
    } else {
      editorDrawer?.expand();
    }
    return parsed;
  }

  function openEditorFilePicker(): void {
    openFilePicker((text) => { void loadEditorText(text); });
  }

  function loadSampleJson(moveFocus = true): void {
    void loadEditorText(SAMPLE_TRACE, moveFocus);
  }

  /** Save the current payload to disk — the palette's export command. */
  function downloadTraceJson(): void {
    if (!editorValue.trim()) return;
    const blob = new Blob([editorValue], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `widescope-trace-${state.summary?.trace_id ?? 'export'}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  function clearEditorJson(): void {
    clearLiveParseTimer();
    editorMessage = null;
    editorValue = '';
    editorDrawer?.expand();
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
      editorMessage = 'Clipboard access was blocked — paste into the editor below.';
      editorDrawer?.expand();
      await tick();
      editorDrawer?.focusLine(1);
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

  /**
   * Finish editing: flush any pending live parse, then hand the screen to the
   * trace. Parsing itself is not this function's job — live parse already did
   * it, which is why there is no Submit button.
   */
  async function dismissEditor(): Promise<void> {
    clearLiveParseTimer();
    if (!(await applyEditorValue(true))) return;
    editorDrawer?.collapse();
    await focusActiveView();
  }

  async function loadTraceFromUrl(url: string): Promise<void> {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const text = await response.text();
    await loadEditorText(text);
  }

  onMount(() => installKeyboardRouter({
    toggleHelp: () => (showKeyboardHelp = !showKeyboardHelp),
    closeHelp: () => { showKeyboardHelp = false; showPalette = false; },
    isHelpOpen: () => showKeyboardHelp || showPalette,
    openFile: openEditorFilePicker,
    focusSearch: () => (showPalette = true),
    submitEditor: () => void dismissEditor(),
    pasteFromClipboard: () => void pasteFromClipboard(),
    selectView: (i) => activeView.set(VIEW_ORDER[i]),
    viewCount: VIEW_ORDER.length,
    toggleFullscreen: () => fullscreen.update((v) => !v),
    exitFullscreen: () => fullscreen.set(false),
    isFullscreen: () => $fullscreen,
  }));

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

  $: if (state.status === 'error' && state.error?.context && editorDrawer) {
    const line = state.error.context.line;
    if (typeof line === 'number' && line > 0) editorDrawer.focusLine(line);
  }
</script>

<div class="app" class:app--fullscreen={$fullscreen} data-theme={$theme}>
  {#if wasmError || !wasmReady}
    <WasmBoot error={wasmError} />
  {:else}
    <div class="layout">
      <a class="skip-link" href="#main">Skip to trace view</a>
      {#if !isEmbedded && !$fullscreen}
        <Toolbar onOpenFile={openEditorFilePicker} />
      {/if}
      <ErrorBanner
        error={state.status === 'error' ? state.error : null}
        warnings={allWarnings}
        isSample={state.isSampleTrace}
      />
      <main class="main" id="main">
        <h1 class="sr-only">WideScope trace viewer</h1>
        <Workspace
          bind:this={workspace}
          {state}
          {slideDirection}
          empty={!editorValue.trim()}
          onLoadSample={loadSampleJson}
          onOpenFile={openEditorFilePicker}
          onPaste={pasteFromClipboard}
        />

        {#if !isEmbedded && !$fullscreen}
          <EditorDrawer
            bind:this={editorDrawer}
            bind:value={editorValue}
            bind:collapsed={editorCollapsed}
            message={editorMessage}
            onInput={onEditorInput}
            onSubmit={dismissEditor}
            onClear={clearEditorJson}
            onLoadSample={loadSampleJson}
            onPaste={pasteFromClipboard}
            onFormat={formatEditorJson}
          />
        {/if}
      </main>
      {#if !isEmbedded && !$fullscreen}
        <Footer />
      {/if}
    </div>
    {#if $fullscreen}
      <button class="fullscreen-exit" aria-label="Exit fullscreen" on:click={() => fullscreen.set(false)}>
        <Icon name="collapse" size={16} />
      </button>
    {/if}
    <DropZone onFileDrop={onDroppedFile} />
    {#if showKeyboardHelp}
      <KeyboardHelp on:close={() => (showKeyboardHelp = false)} />
    {/if}
    <CommandPalette
      open={showPalette}
      onLoadSample={loadSampleJson}
      onOpenFile={openEditorFilePicker}
      onPaste={pasteFromClipboard}
      onExport={downloadTraceJson}
      on:close={() => (showPalette = false)}
    />
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
    background: var(--color-selection-bg);
    color: var(--color-selection-text);
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

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }

  .skip-link {
    position: absolute;
    left: var(--space-2);
    top: -3rem;
    z-index: 200;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--color-accent);
    color: #fff;
    font-size: var(--text-sm);
    font-weight: 600;
    text-decoration: none;
    transition: top var(--dur-fast) var(--ease-out);
  }

  .skip-link:focus { top: var(--space-2); }

  .app {
    height: 100vh;
    height: 100dvh;
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
    height: 100dvh;
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


  /* mini flame graph, straight off the landing hero */






  .app--fullscreen .main {
    padding: 0;
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
      min-height: 100dvh;
      height: auto;
      overflow: auto;
    }

    .main {
      overflow: visible;
      padding: 0.65rem;
    }
  }
</style>
