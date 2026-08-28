<script lang="ts">
  import Icon from './ui/Icon.svelte';
  import { traceState } from '../stores/trace';
  import { filterSpans, getCostBreakdown, type SpanFilters } from '../lib/wasm';
  import {
    filteredSpanIds,
    filterStatus,
    filterService,
    filterKind,
    filterLlmOnly,
  } from '../stores/selection';
  import type { BudgetViolation } from '../stores/budgets';

  export let violations: BudgetViolation[] = [];
  export let onOpenBudgets: () => void;

  const FORMAT_LABELS: Record<string, string> = {
    OtlpJson: 'OTLP JSON',
    JaegerJson: 'Jaeger JSON',
    OpenInferenceJson: 'OpenInference',
  };

  $: summary = $traceState.summary;
  $: isSample = $traceState.isSampleTrace;
  $: status = $traceState.status;
  $: hasFilters = $filterStatus || $filterService || $filterKind || $filterLlmOnly;
  $: visibleSpanCount = hasFilters ? $filteredSpanIds.length : (summary?.span_count ?? 0);
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
</script>

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
          <span class="stat stat--llm" title="LLM spans"><Icon name="bolt" size={11} /> llm <b>{summary.llm_span_count}</b></span>
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
            on:click={onOpenBudgets}
          ><Icon name="warning" size={12} /> {violations.length} budget{violations.length === 1 ? '' : 's'} violated</button>
        {/if}
      </div>

      <div class="stats-right">
        {#if summary}
          <span
            class="filter-count"
            class:filter-count--empty={hasFilters && visibleSpanCount === 0}
            aria-live="polite"
          >{visibleSpanCount} of {summary.span_count} spans</span>
        {/if}
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
          ><Icon name="bolt" size={11} /> LLM</button>
        </div>

        {#if hasFilters}
          <div class="filter-chips">
            {#if $filterStatus}
              <button class="filter-chip" on:click={() => filterStatus.set('')}>status:{$filterStatus} <Icon name="close" size=10 /></button>
            {/if}
            {#if $filterKind}
              <button class="filter-chip" on:click={() => filterKind.set('')}>kind:{$filterKind} <Icon name="close" size=10 /></button>
            {/if}
            {#if $filterLlmOnly}
              <button class="filter-chip" on:click={() => filterLlmOnly.set(false)}>LLM <Icon name="close" size=10 /></button>
            {/if}
            <button class="filter-clear" on:click={clearFilters}>Clear</button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

<style>
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
  .filter-count {
    font-family: var(--font-mono, monospace);
    font-size: 0.72rem;
    letter-spacing: 0.02em;
    color: var(--color-toolbar-muted, #8b9cb5);
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
    .stats-bar {
      flex-direction: column;
      height: auto;
      padding: 0.35rem 0.5rem;
      align-items: flex-start;
      gap: 0.3rem;
      overflow: visible;
    }
    .stats-left { flex-wrap: wrap; overflow: visible; }
    .stats-right { flex-wrap: wrap; }

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
  .stat--llm, .stat--llm b { color: var(--color-gold, #fcd34d); }
  .stat--err, .stat--err b { color: var(--color-danger, #f87171); }
  .stat--muted { font-size: 0.64rem; }
  .stat--cost, .stat--cost b { color: var(--color-success, #34d399); }
  .filter-count--empty {
    color: var(--color-danger, #f87171);
  }
  .filter-btn--active {
    border-color: color-mix(in srgb, var(--color-gold, #fcd34d) 55%, transparent);
    color: var(--color-gold, #fcd34d);
    background: var(--color-llm-badge-bg, rgba(245, 158, 11, 0.16));
  }

  @media (max-width: 820px) {
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
