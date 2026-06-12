<script lang="ts">
  import { traceState } from '../stores/trace';
  import { traceList } from '../stores/traceList';
  import { openFilePicker } from '../lib/input';
  import { theme } from '../lib/theme';
  import { buildShareUrl, isShareSupported } from '../lib/permalink';
  import { searchSpans, filterSpans, getCostBreakdown, type SpanFilters } from '../lib/wasm';
  import {
    activeView,
    focusedSpanId,
    searchQuery,
    searchResults,
    filteredSpanIds,
    filterStatus,
    filterService,
    filterKind,
    filterLlmOnly,
    selectedSpanId,
    fullscreen,
  } from '../stores/selection';
  import { budgets, checkViolations } from '../stores/budgets';
  import BudgetsDialog from './BudgetsDialog.svelte';

  export let onOpenFile: () => void = () => openFilePicker();

  const FORMAT_LABELS: Record<string, string> = {
    OtlpJson: 'OTLP JSON',
    JaegerJson: 'Jaeger JSON',
    OpenInferenceJson: 'OpenInference',
  };

  $: summary = $traceState.summary;
  $: isSample = $traceState.isSampleTrace;
  $: status = $traceState.status;
  $: themeLabel = $theme === 'dark' ? '☀' : '🌙';
  $: searchMessage = $searchQuery.trim()
    ? ($searchResults.length > 0
        ? `${$searchResults.length} match${$searchResults.length === 1 ? '' : 'es'}`
        : `No spans match '${$searchQuery.trim()}'`)
    : '';
  $: hasFilters = $filterStatus || $filterService || $filterKind || $filterLlmOnly;
  $: isFilterActive = $filteredSpanIds.length > 0 && hasFilters;
  $: costBreakdown = status === 'loaded' ? getCostBreakdown() : null;
  $: costDisplay = costBreakdown?.total_cost_usd
    ? `$${costBreakdown.total_cost_usd < 0.01 ? costBreakdown.total_cost_usd.toFixed(6) : costBreakdown.total_cost_usd.toFixed(4)}`
    : '';
  $: violations = checkViolations($budgets, summary, costBreakdown?.total_cost_usd ?? null);
  let budgetsOpen = false;
  $: traceCount = $traceList.length;
  $: activeTraceIdx = $traceList.findIndex(e => $traceState.summary && (e.json.includes($traceState.summary.trace_id)));

  function switchTrace(index: number): void {
    traceList.switchTo(index);
  }

  $: applyFilters($filterStatus, $filterService, $filterKind, $filterLlmOnly);

  function applyFilters(s: string, svc: string, kind: string, llmOnly: boolean): void {
    if (status !== 'loaded') return;
    const filters: SpanFilters = {};
    if (s) filters.status = s;
    if (svc) filters.service = svc;
    if (kind) filters.kind = kind;
    if (llmOnly) filters.llm_only = true;
    if (Object.keys(filters).length === 0) {
      filteredSpanIds.set([]);
      return;
    }
    filteredSpanIds.set(filterSpans(filters));
  }

  function clearFilters(): void {
    filterStatus.set('');
    filterService.set('');
    filterKind.set('');
    filterLlmOnly.set(false);
  }

  function applySearch(nextQuery: string): void {
    searchQuery.set(nextQuery);
    if (status !== 'loaded') { searchResults.set([]); return; }
    const q = nextQuery.trim();
    if (!q) { searchResults.set([]); focusedSpanId.set(null); return; }
    const matches = searchSpans(q);
    searchResults.set(matches);
    if (matches.length === 0) { selectedSpanId.set(null); focusedSpanId.set(null); return; }
    const current = $selectedSpanId ?? $focusedSpanId;
    if (!current || !matches.includes(current)) {
      selectedSpanId.set(matches[0]);
      focusedSpanId.set(matches[0]);
    }
  }

  function focusSearchResult(offset: number): void {
    if ($searchResults.length === 0) return;
    const current = $selectedSpanId ?? $focusedSpanId;
    const idx = current ? $searchResults.indexOf(current) : -1;
    const nextIdx = idx === -1
      ? (offset >= 0 ? 0 : $searchResults.length - 1)
      : (idx + offset + $searchResults.length) % $searchResults.length;
    selectedSpanId.set($searchResults[nextIdx]);
    focusedSpanId.set($searchResults[nextIdx]);
  }

  function onSearchKeyDown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') { event.preventDefault(); focusSearchResult(1); }
    else if (event.key === 'ArrowUp') { event.preventDefault(); focusSearchResult(-1); }
  }

  function onSearchInput(event: Event): void {
    applySearch((event.currentTarget as HTMLInputElement).value);
  }

  // --- Share / permalink ---

  type SharePopover = { text: string; kind: 'success' | 'warn' | 'error'; download: boolean };

  let sharePopover: SharePopover | null = null;
  let sharePopoverTimer: ReturnType<typeof setTimeout> | null = null;

  $: activeTraceJson = activeTraceIdx >= 0 ? ($traceList[activeTraceIdx]?.json ?? null) : null;

  function dismissSharePopover(): void {
    if (sharePopoverTimer) { clearTimeout(sharePopoverTimer); sharePopoverTimer = null; }
    sharePopover = null;
  }

  function showSharePopover(popover: SharePopover, autoDismissMs = 0): void {
    if (sharePopoverTimer) clearTimeout(sharePopoverTimer);
    sharePopover = popover;
    sharePopoverTimer = autoDismissMs > 0
      ? setTimeout(() => { sharePopover = null; sharePopoverTimer = null; }, autoDismissMs)
      : null;
  }

  async function shareTrace(): Promise<void> {
    if (sharePopover) { dismissSharePopover(); return; }
    const json = activeTraceJson;
    if (!json) {
      showSharePopover({ text: 'No trace loaded to share.', kind: 'error', download: false }, 3500);
      return;
    }
    if (!isShareSupported()) {
      showSharePopover({
        text: 'This browser cannot build share links. Download the trace and share the file instead.',
        kind: 'warn',
        download: true,
      });
      return;
    }
    try {
      const result = await buildShareUrl({ json, view: $activeView, spanId: $selectedSpanId });
      if (result.tooLarge) {
        const kb = Math.round(result.dataChars / 1024);
        showSharePopover({
          text: `Trace is too large for a self-contained link (~${kb} KB). Download the trace and share the file, or host it and open it with ?trace=<url>.`,
          kind: 'warn',
          download: true,
        });
        return;
      }
      await navigator.clipboard.writeText(result.url);
      showSharePopover({ text: 'Share link copied to clipboard.', kind: 'success', download: false }, 2800);
    } catch {
      showSharePopover({
        text: 'Could not create or copy a share link. Download the trace instead.',
        kind: 'error',
        download: true,
      }, 5000);
    }
  }

  function downloadTrace(): void {
    const json = activeTraceJson;
    if (!json) return;
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `widescope-trace-${summary?.trace_id ?? 'export'}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    dismissSharePopover();
  }
</script>

<header class="toolbar" class:toolbar--loaded={status === 'loaded'}>
  <div class="top-bar">
    <div class="top-left">
      <div class="brand">
        <img class="logo" src="/widescope-logo.svg" alt="" width="26" height="26" />
        <span class="name">WideScope</span>
      </div>
      <button type="button" class="btn-open" on:click={onOpenFile}>Open file <kbd>⌘O</kbd></button>

      {#if traceCount > 1}
        <select class="trace-select" value={activeTraceIdx} on:change={(e) => switchTrace(parseInt(e.currentTarget.value))} aria-label="Switch trace">
          {#each $traceList as entry, i}
            <option value={i}>{entry.name}</option>
          {/each}
        </select>
        <span class="trace-count">{traceCount} traces</span>
      {/if}
    </div>

    <div class="top-center">
      {#if status === 'loading'}
        <span class="status-loading">parsing…</span>
      {/if}
    </div>

    <div class="top-right">
      {#if status === 'loaded'}
        <div class="search-shell">
          <span class="search-icon" aria-hidden="true">⌕</span>
          <input
            type="search"
            class="search-input"
            value={$searchQuery}
            placeholder="search spans · duration>100ms"
            aria-label="Search spans"
            on:input={onSearchInput}
            on:keydown={onSearchKeyDown}
          />
          <kbd class="search-kbd" aria-hidden="true">⌘K</kbd>
          <button
            type="button"
            class="search-nav"
            aria-label="Previous search result"
            disabled={$searchResults.length === 0}
            on:click={() => focusSearchResult(-1)}
          >↑</button>
          <button
            type="button"
            class="search-nav"
            aria-label="Next search result"
            disabled={$searchResults.length === 0}
            on:click={() => focusSearchResult(1)}
          >↓</button>
          {#if searchMessage}
            <span class="search-status" class:search-status--empty={$searchQuery.trim() && $searchResults.length === 0}>
              {searchMessage}
            </span>
          {/if}
        </div>

        <div class="view-tabs" role="tablist" aria-label="View mode">
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'flame'} role="tab" aria-selected={$activeView === 'flame'} title="Flame graph (1)" on:click={() => activeView.set('flame')}><span class="vt-ic">▲</span><span class="vt-label">Flame</span></button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'timeline'} role="tab" aria-selected={$activeView === 'timeline'} title="Timeline (2)" on:click={() => activeView.set('timeline')}><span class="vt-ic">≡</span><span class="vt-label">Timeline</span></button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'waterfall'} role="tab" aria-selected={$activeView === 'waterfall'} title="Waterfall (3)" on:click={() => activeView.set('waterfall')}><span class="vt-ic">≋</span><span class="vt-label">Waterfall</span></button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'graph'} role="tab" aria-selected={$activeView === 'graph'} title="Service graph (4)" on:click={() => activeView.set('graph')}><span class="vt-ic">◉</span><span class="vt-label">Graph</span></button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'diff'} role="tab" aria-selected={$activeView === 'diff'} title="Trace diff (5)" on:click={() => activeView.set('diff')}><span class="vt-ic">⇆</span><span class="vt-label">Diff</span></button>
        </div>
      {/if}

      {#if status === 'loaded'}
        <div class="share-wrap">
          <button
            type="button"
            class="share-btn"
            class:share-btn--active={sharePopover !== null}
            aria-label="Share this trace"
            title="Copy a self-contained share link"
            on:click={shareTrace}
          >🔗 Share</button>
          {#if sharePopover}
            <div class="share-popover share-popover--{sharePopover.kind}" role="status">
              <span class="share-popover-text">{sharePopover.text}</span>
              <div class="share-popover-actions">
                {#if sharePopover.download}
                  <button type="button" class="share-popover-btn" on:click={downloadTrace}>Download trace</button>
                {/if}
                <button type="button" class="share-popover-close" aria-label="Dismiss" on:click={dismissSharePopover}>✕</button>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <button
        type="button"
        class="budgets-btn"
        class:budgets-btn--violated={violations.length > 0}
        aria-label="Performance budgets"
        title={violations.length > 0 ? `${violations.length} budget violation${violations.length === 1 ? '' : 's'}` : 'Performance budgets'}
        on:click={() => (budgetsOpen = true)}
      >
        🎯
        {#if $budgets.length > 0}
          <span class="budgets-count" class:budgets-count--violated={violations.length > 0}>
            {violations.length > 0 ? violations.length : $budgets.length}
          </span>
        {/if}
      </button>
      <button type="button" class="theme-btn" aria-label="Toggle theme" on:click={() => theme.toggle()}>{themeLabel}</button>
      <button
        type="button"
        class="fullscreen-btn"
        aria-label={$fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
        title={$fullscreen ? 'Exit fullscreen (Esc)' : 'Fullscreen (Shift+F)'}
        on:click={() => fullscreen.update(v => !v)}
      >{$fullscreen ? '↙' : '↗'}</button>
    </div>
  </div>

  {#if status === 'loaded' && summary}
    <div class="stats-bar">
      <div class="stats-left">
        {#if isSample}
          <span class="sample-badge">sample</span>
        {/if}
        <span class="format-badge">{FORMAT_LABELS[summary.detected_format] ?? summary.detected_format}</span>
        <span class="stat">spans <b>{summary.span_count}</b></span>
        <span class="stat-sep">·</span>
        <span class="stat">services <b>{summary.service_count}</b></span>
        <span class="stat-sep">·</span>
        <span class="stat">total <b>{summary.total_duration_display}</b></span>
        {#if summary.llm_span_count > 0}
          <span class="stat-sep">·</span>
          <span class="stat stat--llm" title="LLM spans">⚡ llm <b>{summary.llm_span_count}</b></span>
        {/if}
        {#if costDisplay}
          <span class="stat-sep">·</span>
          <span class="stat stat--cost" title="Estimated LLM cost">est <b>{costDisplay}</b></span>
        {/if}
        {#if summary.has_errors}
          <span class="stat-sep">·</span>
          <span class="stat stat--err" title="{summary.error_count} error spans">errors <b>{summary.error_count}</b></span>
        {/if}
        <span class="stat-sep">·</span>
        <span class="stat stat--muted" title="P50 / P95 latency">p50 <b>{summary.latency_p50_display}</b> p95 <b>{summary.latency_p95_display}</b></span>
        {#if violations.length > 0}
          <span class="stat-sep">·</span>
          <button
            type="button"
            class="stat stat--violation"
            title={violations.map((v) => `${v.budget.field} ${v.budget.operator} ${v.budget.value}`).join(', ')}
            on:click={() => (budgetsOpen = true)}
          >⚠ {violations.length} budget{violations.length === 1 ? '' : 's'} violated</button>
        {/if}
      </div>

      <div class="stats-right">
        <div class="filter-group">
          <select class="filter-select" bind:value={$filterStatus} aria-label="Filter by status">
            <option value="">All status</option>
            <option value="ok">OK</option>
            <option value="error">Error</option>
            <option value="unset">Unset</option>
          </select>
          <select class="filter-select" bind:value={$filterKind} aria-label="Filter by span kind">
            <option value="">All kinds</option>
            <option value="internal">Internal</option>
            <option value="server">Server</option>
            <option value="client">Client</option>
            <option value="producer">Producer</option>
            <option value="consumer">Consumer</option>
          </select>
          <button
            type="button"
            class="filter-btn"
            class:filter-btn--active={$filterLlmOnly}
            aria-label="Show LLM spans only"
            title="LLM only"
            on:click={() => filterLlmOnly.update(v => !v)}
          >⚡ LLM</button>
        </div>

        {#if hasFilters}
          <div class="filter-chips">
            {#if $filterStatus}
              <button class="filter-chip" on:click={() => filterStatus.set('')}>status:{$filterStatus} ✕</button>
            {/if}
            {#if $filterKind}
              <button class="filter-chip" on:click={() => filterKind.set('')}>kind:{$filterKind} ✕</button>
            {/if}
            {#if $filterLlmOnly}
              <button class="filter-chip" on:click={() => filterLlmOnly.set(false)}>LLM ✕</button>
            {/if}
            <button class="filter-clear" on:click={clearFilters}>Clear</button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</header>

<BudgetsDialog
  open={budgetsOpen}
  {violations}
  on:close={() => (budgetsOpen = false)}
/>

<style>
  .toolbar {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    background: color-mix(in srgb, var(--color-toolbar, #070c16) 88%, transparent);
    backdrop-filter: blur(14px);
    color: var(--color-toolbar-text, #e9eff8);
    border-bottom: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    z-index: 10;
  }

  .top-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 0.9rem;
    height: 50px;
  }

  .top-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-shrink: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.02rem;
    letter-spacing: -0.01em;
  }

  .logo {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    box-shadow: 0 4px 14px -4px rgba(29, 78, 216, 0.55);
  }

  .top-center {
    flex: 1;
    min-width: 0;
  }

  .top-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .btn-open {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.38rem 0.8rem;
    background: var(--grad-cta);
    color: #fff;
    border: none;
    border-radius: 8px;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    white-space: nowrap;
    box-shadow: var(--shadow-cta);
    transition: transform 0.18s var(--ease-out), box-shadow 0.18s var(--ease-spring);
  }

  .btn-open kbd {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    border: 1px solid rgba(255, 255, 255, 0.35);
    border-radius: 4px;
    padding: 0 0.26rem;
    background: rgba(255, 255, 255, 0.12);
  }

  .btn-open:hover {
    transform: translateY(-1px);
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.28) inset, 0 12px 30px -8px rgba(37, 99, 235, 0.65);
  }

  .trace-select {
    padding: 0.26rem 0.45rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    outline: none;
    cursor: pointer;
    max-width: 180px;
  }

  .trace-select:focus { border-color: var(--color-sky, #7dd3fc); }

  .trace-count {
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.66rem;
    letter-spacing: 0.08em;
    white-space: nowrap;
  }

  .status-loading {
    color: var(--color-sky, #7dd3fc);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    animation: loading-pulse 1.6s ease-in-out infinite;
  }

  @keyframes loading-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }

  .search-shell {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    position: relative;
  }

  .search-icon {
    position: absolute;
    left: 0.6rem;
    color: var(--color-text-faint, #5b6b84);
    font-size: 0.9rem;
    pointer-events: none;
  }

  .search-kbd {
    position: absolute;
    right: 4.2rem;
    font-family: var(--font-mono);
    font-size: 0.58rem;
    color: var(--color-text-faint, #5b6b84);
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 0.28rem;
    pointer-events: none;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
  }

  .search-input {
    width: 230px;
    padding: 0.42rem 2.6rem 0.42rem 1.8rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 9px;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.74rem;
    outline: none;
    transition: border-color 0.18s var(--ease-spring), box-shadow 0.18s var(--ease-spring), width 0.25s var(--ease-out);
  }

  .search-input::placeholder { color: var(--color-text-faint, #5b6b84); }

  .search-input:focus {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent, #3b82f6) 16%, transparent);
  }

  .search-nav {
    padding: 0.3rem 0.5rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    cursor: pointer;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring);
  }

  .search-nav:hover:not(:disabled) {
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
  }
  .search-nav:disabled { cursor: not-allowed; opacity: 0.4; }

  .search-status {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.68rem;
  }

  .search-status--empty { color: var(--color-gold, #fcd34d); }

  .view-tabs {
    display: flex;
    gap: 2px;
    background: var(--color-canvas-bg, #070c16);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 9px;
    padding: 3px;
  }

  .view-tab {
    display: inline-flex;
    align-items: center;
    gap: 0.36rem;
    height: 26px;
    padding: 0 0.65rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: background 0.15s var(--ease-spring), color 0.15s var(--ease-spring);
  }

  .vt-ic { font-size: 0.72rem; opacity: 0.8; }

  .view-tab:hover {
    color: var(--color-toolbar-text, #e9eff8);
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
  }

  .view-tab--active {
    background: var(--grad-cta);
    color: #fff;
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.25) inset, 0 4px 12px -4px rgba(37, 99, 235, 0.6);
  }

  .view-tab--active .vt-ic { opacity: 1; }

  .theme-btn {
    padding: 0.32rem 0.55rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-text, #e9eff8);
    font-size: 0.8rem;
    cursor: pointer;
    line-height: 1;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-bounce);
  }

  .theme-btn:hover {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    transform: scale(1.06);
  }

  .budgets-btn {
    position: relative;
    padding: 0.32rem 0.55rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-text, #e9eff8);
    font-size: 0.8rem;
    cursor: pointer;
    line-height: 1;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-bounce);
  }

  .budgets-btn:hover {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    transform: scale(1.06);
  }

  .budgets-btn--violated { border-color: rgba(248, 113, 113, 0.55); }

  .budgets-count {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 7px;
    background: var(--color-accent, #3b82f6);
    color: white;
    font-size: 0.6rem;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
  }

  .budgets-count--violated { background: #f87171; }

  .stat--violation {
    color: #f87171;
    background: transparent;
    border: 1px solid rgba(248, 113, 113, 0.4);
    border-radius: 4px;
    padding: 0.05rem 0.4rem;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }

  .stat--violation:hover { background: rgba(248, 113, 113, 0.1); }

  .fullscreen-btn {
    padding: 0.32rem 0.55rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-muted, #8b9cb5);
    font-size: 0.8rem;
    cursor: pointer;
    line-height: 1;
    transition: color 0.15s var(--ease-spring), background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring);
  }

  .fullscreen-btn:hover {
    color: var(--color-toolbar-text, #e9eff8);
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
  }

  .share-wrap {
    position: relative;
    display: flex;
  }

  .share-btn {
    padding: 0.32rem 0.6rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    cursor: pointer;
    line-height: 1;
    white-space: nowrap;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-bounce);
  }

  .share-btn:hover {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    transform: scale(1.04);
  }
  .share-btn--active { background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05)); }

  .share-popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 30;
    width: 270px;
    padding: 0.7rem 0.8rem;
    border-radius: 10px;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    background: color-mix(in srgb, var(--color-surface, #0d1626) 96%, transparent);
    backdrop-filter: blur(10px);
    box-shadow: var(--shadow-panel);
    font-size: 0.78rem;
    line-height: 1.5;
  }

  .share-popover--success { border-left: 3px solid #22c55e; }
  .share-popover--warn { border-left: 3px solid #f59e0b; }
  .share-popover--error { border-left: 3px solid #ef4444; }

  .share-popover-text {
    display: block;
    color: var(--color-toolbar-text, #f1f5f9);
  }

  .share-popover-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }

  .share-popover-btn {
    padding: 0.22rem 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 5px;
    background: transparent;
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.75rem;
    cursor: pointer;
  }

  .share-popover-btn:hover { background: rgba(255, 255, 255, 0.1); }

  .share-popover-close {
    padding: 0.1rem 0.35rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .share-popover-close:hover { color: var(--color-toolbar-text, #f1f5f9); background: rgba(255, 255, 255, 0.1); }

  /* ── Stats bar ────────────────────────────────────────────────── */

  .stats-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0 0.9rem;
    height: 33px;
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 72%, transparent);
    border-top: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    font-family: var(--font-mono);
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    overflow: hidden;
  }

  .stats-left {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
    overflow: hidden;
    flex-shrink: 1;
    flex-wrap: nowrap;
  }

  .stats-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .stat {
    color: var(--color-text-faint, #5b6b84);
    white-space: nowrap;
  }

  .stat b {
    color: var(--color-toolbar-muted, #8b9cb5);
    font-weight: 500;
  }

  .stat-sep {
    color: var(--color-border, rgba(125, 211, 252, 0.13));
    font-size: 0.65rem;
  }

  .stat--llm, .stat--llm b { color: var(--color-gold, #fcd34d); }
  .stat--err, .stat--err b { color: var(--color-danger, #f87171); }
  .stat--muted { font-size: 0.64rem; }
  .stat--cost, .stat--cost b { color: var(--color-success, #34d399); }

  .format-badge {
    position: relative;
    color: var(--color-sky, #7dd3fc);
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.16));
    border: 1px solid color-mix(in srgb, var(--color-sky, #7dd3fc) 26%, transparent);
    border-radius: 999px;
    padding: 0.12rem 0.55rem;
    font-size: 0.62rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .sample-badge {
    background: var(--color-llm-badge-bg, rgba(245, 158, 11, 0.16));
    color: var(--color-gold, #fcd34d);
    border: 1px solid color-mix(in srgb, var(--color-gold, #fcd34d) 30%, transparent);
    border-radius: 999px;
    padding: 0.12rem 0.55rem;
    font-size: 0.62rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .filter-select {
    padding: 0.18rem 0.35rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 6px;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.66rem;
    outline: none;
    cursor: pointer;
    transition: border-color 0.15s var(--ease-spring), color 0.15s var(--ease-spring);
  }

  .filter-select:hover { color: var(--color-toolbar-text, #e9eff8); }
  .filter-select:focus { border-color: var(--color-sky, #7dd3fc); }

  .filter-btn {
    padding: 0.18rem 0.45rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 6px;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.66rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s var(--ease-spring);
  }

  .filter-btn--active {
    border-color: color-mix(in srgb, var(--color-gold, #fcd34d) 55%, transparent);
    color: var(--color-gold, #fcd34d);
    background: var(--color-llm-badge-bg, rgba(245, 158, 11, 0.16));
  }

  .filter-chips {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .filter-chip {
    padding: 0.12rem 0.4rem;
    border: 1px solid color-mix(in srgb, var(--color-sky, #7dd3fc) 40%, transparent);
    border-radius: 999px;
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.16));
    color: var(--color-sky, #7dd3fc);
    font-size: 0.62rem;
    font-family: var(--font-mono);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s var(--ease-spring);
  }

  .filter-chip:hover { background: color-mix(in srgb, var(--color-accent, #3b82f6) 28%, transparent); }

  .filter-clear {
    background: none;
    border: none;
    color: var(--color-text-faint, #5b6b84);
    font-family: var(--font-mono);
    font-size: 0.64rem;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
    padding: 0 0.2rem;
  }

  .filter-clear:hover { color: var(--color-toolbar-text, #e9eff8); }

  /* ── Responsive ───────────────────────────────────────────────── */

  @media (max-width: 1180px) {
    .vt-label { display: none; }
    .view-tab { padding: 0 0.55rem; }
    .vt-ic { font-size: 0.8rem; opacity: 1; }
    .search-input { width: 170px; }
    .search-kbd { display: none; }
  }

  @media (max-width: 820px) {
    .top-bar { flex-wrap: wrap; height: auto; padding: 0.4rem 0.5rem; gap: 0.4rem 0.5rem; }

    /* Let the right-hand cluster wrap onto its own rows instead of
       overflowing the viewport: search takes a full row, the tabs and
       action buttons flow onto the next. */
    .top-right {
      flex-wrap: wrap;
      flex-basis: 100%;
      justify-content: flex-end;
      row-gap: 0.45rem;
    }
    .search-shell { flex: 1 1 100%; }
    .search-input { flex: 1; width: auto; min-width: 0; }

    .stats-bar {
      flex-direction: column;
      height: auto;
      padding: 0.35rem 0.5rem;
      align-items: flex-start;
      gap: 0.3rem;
      overflow: visible;
    }
    /* Wrap the stat chips instead of clipping them off-screen. */
    .stats-left { flex-wrap: wrap; overflow: visible; }
    .stats-right { flex-wrap: wrap; }
  }
</style>
