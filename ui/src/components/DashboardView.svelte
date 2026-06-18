<script lang="ts">
  import { traceList } from '../stores/traceList';
  import { comparisonState } from '../stores/comparison';
  import { activeView } from '../stores/selection';
  import { computeDashboard } from '../lib/wasm';
  import type { Dashboard, DashboardRow } from '../lib/types';

  // Recompute whenever the set of loaded traces changes.
  $: dashboard = computeDashboard($traceList) as Dashboard | null;

  const fmt = (n: number) => n.toLocaleString();
  const usd = (n: number) =>
    n >= 0.01 || n === 0 ? `$${n.toFixed(2)}` : `$${n.toFixed(4)}`;

  type SortKey =
    | 'name'
    | 'span_count'
    | 'service_count'
    | 'error_count'
    | 'llm_span_count'
    | 'total_duration_ns'
    | 'cost_usd';

  let sortKey: SortKey = 'total_duration_ns';
  let sortDir: 1 | -1 = -1; // descending by default

  function setSort(key: SortKey): void {
    if (sortKey === key) sortDir = sortDir === 1 ? -1 : 1;
    else {
      sortKey = key;
      sortDir = key === 'name' ? 1 : -1;
    }
  }

  $: sortedRows = dashboard
    ? [...dashboard.rows].sort((a, b) => {
        const av = a[sortKey];
        const bv = b[sortKey];
        const cmp =
          typeof av === 'string' && typeof bv === 'string'
            ? av.localeCompare(bv)
            : (av as number) - (bv as number);
        return cmp * sortDir;
      })
    : [];

  const arrow = (key: SortKey) =>
    sortKey === key ? (sortDir === 1 ? ' ▲' : ' ▼') : '';

  // --- Row selection for comparison (max two) ---
  let selected: string[] = [];

  function toggle(traceId: string): void {
    if (selected.includes(traceId)) {
      selected = selected.filter((id) => id !== traceId);
    } else if (selected.length < 2) {
      selected = [...selected, traceId];
    }
  }

  /** Find the trace-list index whose payload contains the given trace id. */
  function indexOfTrace(traceId: string): number {
    return $traceList.findIndex((e) => e.json.includes(traceId));
  }

  function openTrace(traceId: string): void {
    const idx = indexOfTrace(traceId);
    if (idx < 0) return;
    traceList.switchTo(idx);
    activeView.set('waterfall');
  }

  function compareSelected(): void {
    if (selected.length !== 2) return;
    const [a, b] = selected;
    const aIdx = indexOfTrace(a);
    const bEntry = $traceList[indexOfTrace(b)];
    if (aIdx < 0 || !bEntry) return;
    traceList.switchTo(aIdx); // trace A becomes the active/primary trace
    comparisonState.loadComparison(bEntry.json); // trace B becomes the candidate
    activeView.set('diff');
  }
</script>

<div class="dash">
  {#if !dashboard || dashboard.rows.length === 0}
    <div class="empty">No traces loaded.</div>
  {:else}
    <div class="summary">
      <div class="stat"><span class="stat-label">Traces</span><span class="stat-value">{dashboard.trace_count}</span></div>
      <div class="stat"><span class="stat-label">Total spans</span><span class="stat-value">{fmt(dashboard.total_spans)}</span></div>
      <div class="stat"><span class="stat-label">Total errors</span><span class="stat-value" class:err={dashboard.total_errors > 0}>{fmt(dashboard.total_errors)}</span></div>
      <div class="stat"><span class="stat-label">LLM spans</span><span class="stat-value">{fmt(dashboard.total_llm_spans)}</span></div>
      <div class="stat"><span class="stat-label">Avg latency</span><span class="stat-value">{dashboard.avg_duration_display}</span></div>
      <div class="stat"><span class="stat-label">Est. cost</span><span class="stat-value">{usd(dashboard.total_cost_usd)}</span></div>
    </div>

    {#if selected.length > 0}
      <div class="compare-bar">
        <span>{selected.length} selected{selected.length === 1 ? ' — pick one more to compare' : ''}</span>
        <button class="cmp-btn" disabled={selected.length !== 2} on:click={compareSelected}>Compare ⇆</button>
        <button class="cmp-clear" on:click={() => (selected = [])}>Clear</button>
      </div>
    {/if}

    <section class="panel">
      <table>
        <thead>
          <tr>
            <th class="pick"></th>
            <th class="sortable" on:click={() => setSort('name')}>Trace{arrow('name')}</th>
            <th>Format</th>
            <th class="num sortable" on:click={() => setSort('span_count')}>Spans{arrow('span_count')}</th>
            <th class="num sortable" on:click={() => setSort('service_count')}>Services{arrow('service_count')}</th>
            <th class="num sortable" on:click={() => setSort('error_count')}>Errors{arrow('error_count')}</th>
            <th class="num sortable" on:click={() => setSort('llm_span_count')}>LLM{arrow('llm_span_count')}</th>
            <th class="num sortable" on:click={() => setSort('total_duration_ns')}>Duration{arrow('total_duration_ns')}</th>
            <th class="num">P95</th>
            <th class="num sortable" on:click={() => setSort('cost_usd')}>Cost{arrow('cost_usd')}</th>
          </tr>
        </thead>
        <tbody>
          {#each sortedRows as row}
            <tr class:selected={selected.includes(row.trace_id)}>
              <td class="pick">
                <input
                  type="checkbox"
                  aria-label={`Select ${row.name} for comparison`}
                  checked={selected.includes(row.trace_id)}
                  disabled={!selected.includes(row.trace_id) && selected.length >= 2}
                  on:change={() => toggle(row.trace_id)}
                />
              </td>
              <td class="link" title={`${row.trace_id}\n${row.root_service ?? ''}`}>
                <button class="open-btn" on:click={() => openTrace(row.trace_id)}>{row.name}</button>
              </td>
              <td class="fmt">{row.detected_format}</td>
              <td class="num">{fmt(row.span_count)}</td>
              <td class="num">{fmt(row.service_count)}</td>
              <td class="num" class:err={row.error_count > 0}>{fmt(row.error_count)}</td>
              <td class="num">{fmt(row.llm_span_count)}</td>
              <td class="num">{row.total_duration_display}</td>
              <td class="num muted">{row.latency_p95_display}</td>
              <td class="num">{usd(row.cost_usd)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    {#if dashboard.top_services.length > 0}
      <section class="panel">
        <h3>Services by trace coverage</h3>
        {#each dashboard.top_services.slice(0, 10) as svc}
          <div class="svc-row">
            <span class="svc-name" title={svc.name}>{svc.name}</span>
            <span class="svc-count">{svc.trace_count} / {dashboard.trace_count} trace{dashboard.trace_count === 1 ? '' : 's'}</span>
          </div>
        {/each}
      </section>
    {/if}
  {/if}
</div>

<style>
  .dash {
    height: 100%;
    overflow: auto;
    padding: 1rem 1.25rem;
    color: var(--color-text, #e9eff8);
    font-size: 0.85rem;
  }
  .empty {
    color: var(--color-text-muted, #94a3b8);
    padding: 2rem;
    text-align: center;
  }
  .summary {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 7rem;
    padding: 0.6rem 0.85rem;
    background: var(--color-surface, #0d1626);
    border: 1px solid var(--color-border, #334155);
    border-radius: 8px;
  }
  .stat-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-muted, #94a3b8);
  }
  .stat-value {
    font-size: 1.1rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .stat-value.err {
    color: var(--color-danger, #f87171);
  }
  .compare-bar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.85rem;
    font-size: 0.78rem;
    color: var(--color-text-muted, #94a3b8);
  }
  .cmp-btn {
    padding: 0.3rem 0.7rem;
    border: none;
    border-radius: 7px;
    background: var(--color-accent, #3b82f6);
    color: #fff;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .cmp-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .cmp-clear {
    background: none;
    border: none;
    color: var(--color-text-faint, #5b6b84);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .panel {
    background: var(--color-surface, #0d1626);
    border: 1px solid var(--color-border, #334155);
    border-radius: 8px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
  }
  .panel h3 {
    margin: 0.25rem 0 0.6rem;
    font-size: 0.9rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 0.4rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #334155);
    text-align: left;
    white-space: nowrap;
  }
  th {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--color-text-muted, #94a3b8);
  }
  th.sortable {
    cursor: pointer;
    user-select: none;
  }
  th.sortable:hover {
    color: var(--color-text, #e9eff8);
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .pick {
    width: 1.5rem;
    text-align: center;
  }
  td.muted {
    color: var(--color-text-muted, #94a3b8);
  }
  td.err {
    color: var(--color-danger, #f87171);
  }
  .fmt {
    color: var(--color-sky, #7dd3fc);
    font-size: 0.76rem;
  }
  tr.selected {
    background: color-mix(in srgb, var(--color-accent, #3b82f6) 12%, transparent);
  }
  .open-btn {
    background: none;
    border: none;
    padding: 0;
    color: var(--color-sky, #7dd3fc);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
    max-width: 16rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .open-btn:hover {
    color: var(--color-text, #e9eff8);
  }
  .svc-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.25rem 0;
    border-bottom: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
  }
  .svc-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .svc-count {
    color: var(--color-text-muted, #94a3b8);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
