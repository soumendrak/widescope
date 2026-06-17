<script lang="ts">
  import { traceList } from '../stores/traceList';
  import { computeTokenTrends } from '../lib/wasm';
  import type { TokenTrends, TokenGroup } from '../lib/types';

  // Recompute whenever the set of loaded traces changes.
  $: trends = computeTokenTrends($traceList) as TokenTrends | null;

  const fmt = (n: number) => n.toLocaleString();
  const usd = (n: number) =>
    n >= 0.01 || n === 0 ? `$${n.toFixed(2)}` : `$${n.toFixed(4)}`;

  // Bar width as a percentage of the largest total in a group.
  function pct(value: number, rows: TokenGroup[]): number {
    const max = Math.max(1, ...rows.map((r) => r.total_tokens));
    return (value / max) * 100;
  }
</script>

<div class="trends">
  {#if !trends || trends.per_trace.length === 0}
    <div class="empty">No traces loaded.</div>
  {:else if trends.total_tokens === 0}
    <div class="empty">No LLM token usage found across {trends.trace_count} trace{trends.trace_count === 1 ? '' : 's'}.</div>
  {:else}
    <div class="summary">
      <div class="stat"><span class="stat-label">Traces</span><span class="stat-value">{trends.trace_count}</span></div>
      <div class="stat"><span class="stat-label">Input tokens</span><span class="stat-value">{fmt(trends.total_input_tokens)}</span></div>
      <div class="stat"><span class="stat-label">Output tokens</span><span class="stat-value">{fmt(trends.total_output_tokens)}</span></div>
      <div class="stat"><span class="stat-label">Total tokens</span><span class="stat-value">{fmt(trends.total_tokens)}</span></div>
      <div class="stat"><span class="stat-label">Est. cost</span><span class="stat-value">{usd(trends.total_cost_usd)}</span></div>
    </div>

    {#each [{ title: 'Tokens per model', rows: trends.per_model }, { title: 'Tokens per service', rows: trends.per_service }] as group}
      <section class="panel">
        <h3>{group.title}</h3>
        <div class="legend"><span class="swatch in"></span>input<span class="swatch out"></span>output</div>
        {#each group.rows as row}
          <div class="bar-row">
            <div class="bar-name" title={row.name}>{row.name}</div>
            <div class="bar-track" style={`width:${pct(row.total_tokens, group.rows)}%`}>
              <div class="bar in" style={`flex:${row.input_tokens}`}></div>
              <div class="bar out" style={`flex:${row.output_tokens}`}></div>
            </div>
            <div class="bar-meta">{fmt(row.total_tokens)} · {usd(row.cost_usd)}</div>
          </div>
        {/each}
      </section>
    {/each}

    <section class="panel">
      <h3>Per-trace metrics</h3>
      <table>
        <thead>
          <tr><th>Trace</th><th class="num">Input</th><th class="num">Output</th><th class="num">Total</th><th class="num">LLM spans</th><th class="num">Cost</th></tr>
        </thead>
        <tbody>
          {#each trends.per_trace as t}
            <tr>
              <td title={t.trace_id}>{t.name}</td>
              <td class="num">{fmt(t.input_tokens)}</td>
              <td class="num">{fmt(t.output_tokens)}</td>
              <td class="num">{fmt(t.total_tokens)}</td>
              <td class="num">{fmt(t.llm_span_count)}</td>
              <td class="num">{usd(t.cost_usd)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}
</div>

<style>
  .trends {
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
    margin-bottom: 1.25rem;
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
  .panel {
    background: var(--color-surface, #0d1626);
    border: 1px solid var(--color-border, #334155);
    border-radius: 8px;
    padding: 0.85rem 1rem;
    margin-bottom: 1rem;
  }
  .panel h3 {
    margin: 0 0 0.6rem;
    font-size: 0.9rem;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.72rem;
    color: var(--color-text-muted, #94a3b8);
    margin-bottom: 0.6rem;
  }
  .swatch {
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 2px;
    display: inline-block;
  }
  .swatch.in,
  .bar.in {
    background: var(--color-accent, #3b82f6);
  }
  .swatch.out,
  .bar.out {
    background: #f59e0b;
  }
  .swatch.out {
    margin-left: 0.5rem;
  }
  .bar-row {
    display: grid;
    grid-template-columns: minmax(6rem, 12rem) 1fr minmax(7rem, auto);
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.35rem;
    min-width: 0;
  }
  .bar-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .bar-track {
    display: flex;
    height: 0.9rem;
    min-width: 2px;
    border-radius: 3px;
    overflow: hidden;
    background: var(--color-canvas-bg, #070c16);
  }
  .bar {
    height: 100%;
  }
  .bar-meta {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--color-text-muted, #94a3b8);
    white-space: nowrap;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #334155);
    text-align: left;
  }
  th {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--color-text-muted, #94a3b8);
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  td {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 16rem;
    white-space: nowrap;
  }
</style>
