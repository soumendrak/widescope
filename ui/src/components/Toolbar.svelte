<script lang="ts">
  import { traceState } from '../stores/trace';
  import { openFilePicker } from '../lib/input';
  import { theme } from '../lib/theme';
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
  } from '../stores/selection';

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
</script>

<header class="toolbar" class:toolbar--loaded={status === 'loaded'}>
  <div class="top-bar">
    <div class="top-left">
      <div class="brand">
        <span class="logo">🔭</span>
        <span class="name">WideScope</span>
      </div>
      <button type="button" class="btn-open" on:click={onOpenFile}>Open file</button>
    </div>

    <div class="top-center">
      {#if status === 'loading'}
        <span class="status-loading">Parsing…</span>
      {/if}
    </div>

    <div class="top-right">
      {#if status === 'loaded'}
        <div class="search-shell">
          <input
            type="search"
            class="search-input"
            value={$searchQuery}
            placeholder="Search (e.g. duration>100ms status=error)…"
            aria-label="Search spans"
            on:input={onSearchInput}
            on:keydown={onSearchKeyDown}
          />
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
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'flame'} role="tab" aria-selected={$activeView === 'flame'} on:click={() => activeView.set('flame')}>F</button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'timeline'} role="tab" aria-selected={$activeView === 'timeline'} on:click={() => activeView.set('timeline')}>T</button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'waterfall'} role="tab" aria-selected={$activeView === 'waterfall'} on:click={() => activeView.set('waterfall')}>W</button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'graph'} role="tab" aria-selected={$activeView === 'graph'} on:click={() => activeView.set('graph')} title="Service graph">G</button>
          <button type="button" class="view-tab" class:view-tab--active={$activeView === 'diff'} role="tab" aria-selected={$activeView === 'diff'} on:click={() => activeView.set('diff')} title="Trace diff">D</button>
        </div>
      {/if}

      <button type="button" class="theme-btn" aria-label="Toggle theme" on:click={() => theme.toggle()}>{themeLabel}</button>
    </div>
  </div>

  {#if status === 'loaded' && summary}
    <div class="stats-bar">
      <div class="stats-left">
        {#if isSample}
          <span class="sample-badge">Sample</span>
        {/if}
        <span class="format-badge">{FORMAT_LABELS[summary.detected_format] ?? summary.detected_format}</span>
        <span class="stat">{summary.span_count} spans</span>
        <span class="stat-sep">·</span>
        <span class="stat">{summary.service_count} services</span>
        <span class="stat-sep">·</span>
        <span class="stat">{summary.total_duration_display}</span>
        {#if summary.llm_span_count > 0}
          <span class="stat-sep">·</span>
          <span class="stat stat--llm" title="LLM spans">⚡ {summary.llm_span_count}</span>
        {/if}
        {#if costDisplay}
          <span class="stat-sep">·</span>
          <span class="stat stat--cost" title="Estimated LLM cost">💰 {costDisplay}</span>
        {/if}
        {#if summary.has_errors}
          <span class="stat-sep">·</span>
          <span class="stat stat--err" title="{summary.error_count} error spans">⚠ {summary.error_count}</span>
        {/if}
        <span class="stat-sep">·</span>
        <span class="stat stat--muted" title="P50 / P95 latency">P50 {summary.latency_p50_display} P95 {summary.latency_p95_display}</span>
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

<style>
  .toolbar {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    background: var(--color-toolbar, #1e293b);
    color: var(--color-toolbar-text, #f1f5f9);
    border-bottom: 1px solid var(--color-border, #334155);
    z-index: 10;
  }

  .toolbar--loaded {
    border-bottom: none;
  }

  .top-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 0.75rem;
    height: 44px;
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
    gap: 0.35rem;
    font-weight: 700;
    font-size: 1rem;
    letter-spacing: -0.01em;
  }

  .logo { font-size: 1.1rem; }

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

  .btn-open:hover { background: var(--color-accent-hover, #2563eb); }

  .status-loading {
    color: var(--color-toolbar-muted, #94a3b8);
    font-style: italic;
    font-size: 0.85rem;
  }

  .search-shell {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .search-input {
    width: 160px;
    padding: 0.35rem 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 6px;
    background: rgba(15, 23, 42, 0.55);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.8rem;
    outline: none;
  }

  .search-input::placeholder { color: var(--color-toolbar-muted, #94a3b8); }

  .search-input:focus {
    border-color: var(--color-accent, #3b82f6);
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
  }

  .search-nav {
    padding: 0.2rem 0.45rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.75rem;
    cursor: pointer;
  }

  .search-nav:hover:not(:disabled) { background: rgba(255, 255, 255, 0.12); }
  .search-nav:disabled { cursor: not-allowed; opacity: 0.45; }

  .search-status {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.75rem;
  }

  .search-status--empty { color: #fbbf24; }

  .view-tabs {
    display: flex;
    gap: 1px;
    background: rgba(0, 0, 0, 0.25);
    border-radius: 5px;
    padding: 2px;
  }

  .view-tab {
    width: 26px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.72rem;
    font-weight: 700;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s, color 0.12s;
  }

  .view-tab:hover {
    color: var(--color-toolbar-text, #f1f5f9);
    background: rgba(255, 255, 255, 0.1);
  }

  .view-tab--active { background: var(--color-accent, #3b82f6); color: #fff; }
  .view-tab--active:hover { background: var(--color-accent-hover, #2563eb); }

  .theme-btn {
    padding: 0.2rem 0.45rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 5px;
    background: transparent;
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.85rem;
    cursor: pointer;
    line-height: 1;
  }

  .theme-btn:hover { background: rgba(255, 255, 255, 0.1); }

  /* ── Stats bar ────────────────────────────────────────────────── */

  .stats-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0 0.75rem;
    height: 30px;
    background: color-mix(in srgb, var(--color-toolbar, #1e293b) 92%, transparent);
    border-bottom: 1px solid var(--color-border, #334155);
    font-size: 0.75rem;
    overflow: hidden;
  }

  .stats-left {
    display: flex;
    align-items: center;
    gap: 0.4rem;
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
    color: var(--color-toolbar-muted, #94a3b8);
    white-space: nowrap;
  }

  .stat-sep {
    color: var(--color-border, #334155);
    font-size: 0.7rem;
  }

  .stat--llm { color: #c4b5fd; }
  .stat--err { color: #f87171; }
  .stat--muted { color: var(--color-toolbar-muted, #94a3b8); font-size: 0.7rem; }
  .stat--cost { color: #86efac; }

  .format-badge {
    background: rgba(255, 255, 255, 0.12);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.68rem;
    font-family: monospace;
    white-space: nowrap;
  }

  .sample-badge {
    background: rgba(251, 191, 36, 0.2);
    color: #fbbf24;
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.68rem;
    white-space: nowrap;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .filter-select {
    padding: 0.15rem 0.3rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 4px;
    background: rgba(15, 23, 42, 0.4);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.7rem;
    outline: none;
    cursor: pointer;
  }

  .filter-select:focus { border-color: var(--color-accent, #3b82f6); }

  .filter-btn {
    padding: 0.15rem 0.4rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 4px;
    background: rgba(15, 23, 42, 0.4);
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.7rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .filter-btn--active {
    border-color: var(--color-accent, #3b82f6);
    color: #fff;
    background: rgba(59, 130, 246, 0.25);
  }

  .filter-chips {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .filter-chip {
    padding: 0.1rem 0.35rem;
    border: 1px solid var(--color-accent, #3b82f6);
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.12);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.65rem;
    font-family: monospace;
    cursor: pointer;
    white-space: nowrap;
  }

  .filter-chip:hover { background: rgba(59, 130, 246, 0.25); }

  .filter-clear {
    background: none;
    border: none;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.7rem;
    cursor: pointer;
    text-decoration: underline;
    padding: 0 0.2rem;
  }

  .filter-clear:hover { color: var(--color-toolbar-text, #f1f5f9); }

  /* ── Responsive ───────────────────────────────────────────────── */

  @media (max-width: 820px) {
    .top-bar { flex-wrap: wrap; height: auto; padding: 0.4rem 0.5rem; gap: 0.4rem; }
    .stats-bar { flex-direction: column; height: auto; padding: 0.35rem 0.5rem; align-items: flex-start; gap: 0.3rem; }
    .stats-right { flex-wrap: wrap; }
    .search-input { width: 120px; }
  }
</style>
