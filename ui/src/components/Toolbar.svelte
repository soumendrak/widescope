<script lang="ts">
  import { traceState } from '../stores/trace';
  import { openFilePicker } from '../lib/input';
  import { theme } from '../lib/theme';
  import { searchSpans, filterSpans, type SpanFilters } from '../lib/wasm';
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
  $: themeLabel = $theme === 'dark' ? '☀ Light' : '🌙 Dark';
  $: searchMessage = $searchQuery.trim()
    ? ($searchResults.length > 0
        ? `${$searchResults.length} match${$searchResults.length === 1 ? '' : 'es'}`
        : `No spans match '${$searchQuery.trim()}'`)
    : '';
  $: hasFilters = $filterStatus || $filterService || $filterKind || $filterLlmOnly;
  $: isFilterActive = $filteredSpanIds.length > 0 && hasFilters;

  $: applyFilters($filterStatus, $filterService, $filterKind, $filterLlmOnly);

  function applyFilters(status: string, service: string, kind: string, llmOnly: boolean): void {
    if (status !== 'loaded') return;
    const filters: SpanFilters = {};
    if (status) filters.status = status;
    if (service) filters.service = service;
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

  function formatCode(code: string): string {
    return code.charAt(0).toUpperCase() + code.slice(1).toLowerCase();
  }

  function applySearch(nextQuery: string): void {
    searchQuery.set(nextQuery);

    if (status !== 'loaded') {
      searchResults.set([]);
      return;
    }

    const normalizedQuery = nextQuery.trim();
    if (!normalizedQuery) {
      searchResults.set([]);
      focusedSpanId.set(null);
      return;
    }

    const matches = searchSpans(normalizedQuery);
    searchResults.set(matches);

    if (matches.length === 0) {
      selectedSpanId.set(null);
      focusedSpanId.set(null);
      return;
    }

    const currentSpanId = $selectedSpanId ?? $focusedSpanId;
    if (!currentSpanId || !matches.includes(currentSpanId)) {
      selectedSpanId.set(matches[0]);
      focusedSpanId.set(matches[0]);
    }
  }

  function focusSearchResult(offset: number): void {
    if ($searchResults.length === 0) {
      return;
    }

    const currentSpanId = $selectedSpanId ?? $focusedSpanId;
    const currentIndex = currentSpanId ? $searchResults.indexOf(currentSpanId) : -1;
    const nextIndex = currentIndex === -1
      ? (offset >= 0 ? 0 : $searchResults.length - 1)
      : (currentIndex + offset + $searchResults.length) % $searchResults.length;
    const nextSpanId = $searchResults[nextIndex];

    selectedSpanId.set(nextSpanId);
    focusedSpanId.set(nextSpanId);
  }

  function onSearchKeyDown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      focusSearchResult(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      focusSearchResult(-1);
    }
  }

  function onSearchInput(event: Event): void {
    applySearch((event.currentTarget as HTMLInputElement).value);
  }
</script>

<header class="toolbar">
  <div class="left">
    <div class="brand">
      <span class="logo">🔭</span>
      <span class="name">WideScope</span>
    </div>

    <button type="button" class="btn-open" on:click={onOpenFile}>
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
        {#if summary.llm_span_count > 0}
          · <span class="llm-dot" title="LLM spans">⚡{summary.llm_span_count}</span>
        {/if}
        {#if summary.has_errors}
          · <span class="error-dot" title="{summary.error_count} error span{summary.error_count !== 1 ? 's' : ''}">⚠{summary.error_count}</span>
        {/if}
        · <span class="latency-stat" title="P50 / P95 latency">P50 {summary.latency_p50_display} / P95 {summary.latency_p95_display}</span>
      </span>

      {#if hasFilters}
        <div class="filter-bar">
          {#if $filterStatus}
            <button class="filter-chip" on:click={() => filterStatus.set('')}>
              status:{formatCode($filterStatus)} ✕
            </button>
          {/if}
          {#if $filterService}
            <button class="filter-chip" on:click={() => filterService.set('')}>
              svc:{$filterService} ✕
            </button>
          {/if}
          {#if $filterKind}
            <button class="filter-chip" on:click={() => filterKind.set('')}>
              kind:{$filterKind} ✕
            </button>
          {/if}
          {#if $filterLlmOnly}
            <button class="filter-chip" on:click={() => filterLlmOnly.set(false)}>
              LLM only ✕
            </button>
          {/if}
          <button class="filter-clear" on:click={clearFilters}>Clear all</button>
        </div>
      {/if}
    {/if}
  </div>

  <div class="right">
    {#if status === 'loaded'}
      <div class="search-shell">
        <input
          type="search"
          class="search-input"
          value={$searchQuery}
          placeholder="Search spans"
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
          <span class="search-status" class:search-status--empty={$searchQuery.trim() !== '' && $searchResults.length === 0}>
            {searchMessage}
          </span>
        {/if}
      </div>

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
        title="LLM spans only"
        on:click={() => filterLlmOnly.update(v => !v)}
      >⚡</button>

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
          class:view-tab--active={$activeView === 'timeline'}
          role="tab"
          aria-selected={$activeView === 'timeline'}
          on:click={() => activeView.set('timeline')}
        >≋ Timeline</button>
        <button
          type="button"
          class="view-tab"
          class:view-tab--active={$activeView === 'waterfall'}
          role="tab"
          aria-selected={$activeView === 'waterfall'}
          on:click={() => activeView.set('waterfall')}
        >☰ Waterfall</button>
      </div>
      <button
        type="button"
        class="theme-btn"
        aria-label="Toggle theme"
        on:click={() => theme.toggle()}
      >{themeLabel}</button>
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
    min-width: 0;
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
    font-weight: 600;
  }

  .llm-dot {
    color: #c4b5fd;
    font-weight: 600;
  }

  .latency-stat {
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.8rem;
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

  .search-shell {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }

  .search-input {
    width: 180px;
    min-width: 0;
    padding: 0.35rem 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 6px;
    background: rgba(15, 23, 42, 0.55);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.8rem;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--color-toolbar-muted, #94a3b8);
  }

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

  .search-nav:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .search-nav:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .search-status {
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.78rem;
  }

  .search-status--empty {
    color: #fbbf24;
  }

  .theme-btn {
    padding: 0.25rem 0.65rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .theme-btn:hover {
    background: rgba(255, 255, 255, 0.12);
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

  .filter-select {
    padding: 0.2rem 0.4rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 5px;
    background: rgba(15, 23, 42, 0.55);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.75rem;
    outline: none;
    cursor: pointer;
    max-width: 90px;
  }

  .filter-select:focus {
    border-color: var(--color-accent, #3b82f6);
  }

  .filter-btn {
    padding: 0.2rem 0.45rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 5px;
    background: rgba(15, 23, 42, 0.55);
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .filter-btn--active {
    border-color: var(--color-accent, #3b82f6);
    color: #fff;
    background: rgba(59, 130, 246, 0.3);
  }

  .filter-bar {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .filter-chip {
    padding: 0.1rem 0.4rem;
    border: 1px solid var(--color-accent, #3b82f6);
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.15);
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.7rem;
    font-family: monospace;
    cursor: pointer;
    white-space: nowrap;
  }

  .filter-chip:hover {
    background: rgba(59, 130, 246, 0.3);
  }

  .filter-clear {
    background: none;
    border: none;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.7rem;
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }

  .filter-clear:hover {
    color: var(--color-toolbar-text, #f1f5f9);
  }
</style>
