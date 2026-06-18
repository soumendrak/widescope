<script lang="ts">
  import { traceList } from '../stores/traceList';
  import { computeComparisonMatrix } from '../lib/wasm';
  import type { ComparisonMatrix } from '../lib/types';

  let baseline = 0;
  let error: string | null = null;

  // Recompute whenever the loaded trace set changes.
  $: matrix = buildMatrix($traceList);

  function buildMatrix(list: { name: string; json: string }[]): ComparisonMatrix | null {
    error = null;
    if (list.length < 2) return null;
    try {
      const m = computeComparisonMatrix(list);
      if (baseline >= m.traces.length) baseline = 0;
      return m;
    } catch (e) {
      error = String(e);
      return null;
    }
  }

  // Color a cell relative to the baseline column for the row's metric.
  function cellClass(row: ComparisonMatrix['rows'][number], col: number): string {
    if (col === baseline || row.lower_is_better === null) return '';
    const base = row.values[baseline];
    const val = row.values[col];
    if (val === base) return '';
    const better = row.lower_is_better ? val < base : val > base;
    return better ? 'cell--good' : 'cell--bad';
  }
</script>

{#if error}
  <div class="cm-empty"><div class="cm-title">Comparison error</div><p class="cm-error">{error}</p></div>
{:else if !matrix}
  <div class="cm-empty">
    <div class="cm-eyebrow">span · matrix <em>// N traces side by side</em></div>
    <div class="cm-title">Load 2+ traces <span class="dim">to compare.</span></div>
    <p>Open multiple traces (drag several files, or use the trace list) to see a metric-by-metric comparison matrix.</p>
  </div>
{:else}
  <div class="cm-wrap">
    <table class="cm-table">
      <thead>
        <tr>
          <th>Metric</th>
          {#each matrix.traces as t, i}
            <th>
              <button
                type="button"
                class="cm-head-btn"
                class:cm-head-btn--baseline={i === baseline}
                title="Open this trace"
                on:click={() => traceList.switchTo(i)}
              >{t.name}</button>
              <button
                type="button"
                class="cm-base-set"
                class:cm-base-set--active={i === baseline}
                title="Set as baseline"
                on:click={() => (baseline = i)}
              >{i === baseline ? '★ baseline' : 'set baseline'}</button>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each matrix.rows as row}
          <tr>
            <td class="cm-label">{row.label}</td>
            {#each row.display as d, i}
              <td class={cellClass(row, i)}>{d}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .cm-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.8rem;
    padding: 2rem;
    text-align: center;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-text-muted, #9aa8bd);
  }

  .cm-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    font-family: var(--font-mono);
    font-size: 0.64rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-gold, #fcd34d);
    border: 1px solid color-mix(in srgb, var(--color-gold, #fcd34d) 26%, transparent);
    background: var(--color-llm-badge-bg, rgba(245, 158, 11, 0.16));
    border-radius: 999px;
    padding: 0.3rem 0.8rem;
  }

  .cm-eyebrow em { font-style: normal; color: var(--color-text-faint, #5b6b84); }

  .cm-title {
    font-family: var(--font-display);
    font-size: clamp(1.5rem, 3vw, 2.1rem);
    font-weight: 750;
    letter-spacing: -0.025em;
    color: var(--color-text, #e9eff8);
  }

  .cm-title .dim { color: var(--color-text-faint, #5b6b84); }
  .cm-empty p { font-size: 0.875rem; max-width: 400px; line-height: 1.6; }
  .cm-error { color: var(--color-danger, #f87171); font-family: var(--font-mono); font-size: 0.78rem; }

  .cm-wrap {
    flex: 1;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 2rem;
    overflow: auto;
    background: var(--color-canvas-bg, #070c16);
  }

  .cm-table {
    border-collapse: collapse;
    background: color-mix(in srgb, var(--color-surface, #0d1626) 80%, transparent);
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 14px;
    overflow: hidden;
    box-shadow: var(--shadow-panel);
  }

  .cm-table th, .cm-table td {
    padding: 0.6rem 1.2rem;
    text-align: left;
    border-bottom: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--color-text, #e9eff8);
  }

  .cm-table th {
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
    vertical-align: top;
  }

  .cm-table tbody tr:last-child td { border-bottom: none; }

  .cm-label {
    color: var(--color-text-muted, #9aa8bd) !important;
    font-family: var(--font-display) !important;
    font-weight: 600;
  }

  .cm-head-btn {
    display: block;
    background: none;
    border: none;
    padding: 0;
    color: var(--color-sky, #7dd3fc);
    font-family: var(--font-display);
    font-weight: 650;
    font-size: 0.82rem;
    cursor: pointer;
    max-width: 14ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cm-head-btn:hover { text-decoration: underline; }
  .cm-head-btn--baseline { color: var(--color-gold, #fcd34d); }

  .cm-base-set {
    margin-top: 0.25rem;
    background: none;
    border: none;
    padding: 0;
    color: var(--color-text-faint, #5b6b84);
    font-family: var(--font-mono);
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
  }

  .cm-base-set:hover { color: var(--color-text-muted, #9aa8bd); }
  .cm-base-set--active { color: var(--color-gold, #fcd34d); cursor: default; }

  .cell--good { color: var(--color-success, #34d399); font-weight: 600; }
  .cell--bad { color: var(--color-danger, #f87171); font-weight: 600; }
</style>
