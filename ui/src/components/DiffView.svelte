<script lang="ts">
  import { traceState } from '../stores/trace';
  import { comparisonState } from '../stores/comparison';
  import { handleFile } from '../lib/input';

  let comparisonInputEl: HTMLTextAreaElement;

  function loadComparisonFile(): void {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        comparisonState.setLoading();
        try {
          const text = await file.text();
          comparisonState.loadComparison(text);
        } catch {
          comparisonState.setError('Failed to read file');
        }
      }
    };
    input.click();
  }

  function pasteComparison(): void {
    navigator.clipboard.readText().then((text) => {
      if (text.trim()) {
        comparisonState.setLoading();
        comparisonState.loadComparison(text);
      }
    }).catch(() => { /* clipboard denied */ });
  }

  function diffNumber(a: number | null, b: number | null): string {
    if (a === null || b === null) return '–';
    const delta = b - a;
    if (delta === 0) return '0';
    const sign = delta > 0 ? '+' : '';
    return `${sign}${delta.toLocaleString()}`;
  }

  function diffDuration(a: string | null, b: string | null): string {
    return a && b ? a + ' / ' + b : '–';
  }

  $: primary = $traceState.summary;
  $: comp = $comparisonState.summary;
  $: status = $comparisonState.status;
</script>

{#if status === 'empty'}
  <div class="diff-empty">
    <div class="diff-empty-title">Compare two traces</div>
    <p>Load a second trace to compare span counts, services, duration, and errors side by side.</p>
    <div class="diff-actions">
      <button class="diff-btn diff-btn--primary" on:click={loadComparisonFile}>Open comparison file</button>
      <button class="diff-btn" on:click={pasteComparison}>Paste comparison JSON</button>
    </div>
  </div>
{:else if status === 'loading'}
  <div class="diff-empty"><div class="diff-empty-title">Parsing comparison…</div></div>
{:else if status === 'error'}
  <div class="diff-empty">
    <div class="diff-empty-title">Comparison error</div>
    <p class="diff-error">{$comparisonState.error}</p>
    <button class="diff-btn" on:click={loadComparisonFile}>Try again</button>
  </div>
{:else if comp}
  <div class="diff-table-wrap">
    <table class="diff-table">
      <thead>
        <tr>
          <th></th>
          <th class="primary-head">Trace A</th>
          <th class="comp-head">Trace B</th>
          <th>Δ</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td class="diff-label">Spans</td>
          <td>{primary?.span_count.toLocaleString() ?? '–'}</td>
          <td>{comp.span_count.toLocaleString()}</td>
          <td class="diff-delta" class:diff-delta--pos={(comp.span_count ?? 0) > (primary?.span_count ?? 0)} class:diff-delta--neg={(comp.span_count ?? 0) < (primary?.span_count ?? 0)}>
            {diffNumber(primary?.span_count ?? null, comp.span_count)}
          </td>
        </tr>
        <tr>
          <td class="diff-label">Services</td>
          <td>{primary?.service_count.toLocaleString() ?? '–'}</td>
          <td>{comp.service_count.toLocaleString()}</td>
          <td>{diffNumber(primary?.service_count ?? null, comp.service_count)}</td>
        </tr>
        <tr>
          <td class="diff-label">Duration</td>
          <td>{primary?.total_duration_display ?? '–'}</td>
          <td>{comp.total_duration_display}</td>
          <td class="diff-delta--pos">{diffDuration(primary?.total_duration_display ?? null, comp.total_duration_display)}</td>
        </tr>
        <tr>
          <td class="diff-label">Errors</td>
          <td>{primary?.error_count.toLocaleString() ?? '–'}</td>
          <td>{comp.error_count.toLocaleString()}</td>
          <td class="diff-delta" class:diff-delta--pos={(comp.error_count) > (primary?.error_count ?? 0)} class:diff-delta--neg={(comp.error_count) < (primary?.error_count ?? 0)}>
            {diffNumber(primary?.error_count ?? null, comp.error_count)}
          </td>
        </tr>
        <tr>
          <td class="diff-label">LLM spans</td>
          <td>{primary?.llm_span_count.toLocaleString() ?? '–'}</td>
          <td>{comp.llm_span_count.toLocaleString()}</td>
          <td>{diffNumber(primary?.llm_span_count ?? null, comp.llm_span_count)}</td>
        </tr>
        <tr>
          <td class="diff-label">P50 latency</td>
          <td>{primary?.latency_p50_display ?? '–'}</td>
          <td>–</td>
          <td>–</td>
        </tr>
      </tbody>
    </table>
    <button class="diff-clear" on:click={() => comparisonState.clear()}>Remove comparison</button>
  </div>
{/if}

<style>
  .diff-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 2rem;
    text-align: center;
    background: var(--color-canvas-bg, #0f172a);
    color: var(--color-text-muted, #94a3b8);
  }

  .diff-empty-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-text, #e2e8f0);
  }

  .diff-empty p { font-size: 0.875rem; max-width: 360px; }

  .diff-error { color: #f87171; font-family: monospace; font-size: 0.8rem; }

  .diff-actions {
    display: flex;
    gap: 0.5rem;
  }

  .diff-btn {
    padding: 0.55rem 1rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 8px;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    color: var(--color-text, #e2e8f0);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  .diff-btn:hover { border-color: var(--color-accent, #3b82f6); }

  .diff-btn--primary {
    background: var(--color-accent, #3b82f6);
    border-color: transparent;
    color: #fff;
  }

  .diff-btn--primary:hover { background: var(--color-accent-hover, #2563eb); }

  .diff-table-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    overflow: auto;
    background: var(--color-canvas-bg, #0f172a);
    gap: 1rem;
  }

  .diff-table {
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .diff-table th, .diff-table td {
    padding: 0.5rem 1.25rem;
    text-align: left;
    border-bottom: 1px solid var(--color-border, #334155);
  }

  .diff-table th {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-muted, #94a3b8);
  }

  .diff-table td {
    color: var(--color-text, #e2e8f0);
    font-family: monospace;
  }

  .diff-label {
    color: var(--color-text-muted, #94a3b8) !important;
    font-family: system-ui, sans-serif !important;
  }

  .primary-head { color: var(--color-accent, #3b82f6) !important; }

  .comp-head { color: #f59e0b !important; }

  .diff-delta { font-weight: 600; }

  .diff-delta--pos { color: #4ade80; }

  .diff-delta--neg { color: #f87171; }

  .diff-clear {
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .diff-clear:hover {
    border-color: #f87171;
    color: #f87171;
  }
</style>
