<script lang="ts">
  import Icon from './ui/Icon.svelte';
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
    <div class="diff-eyebrow">span · diff <em>// baseline vs candidate</em></div>
    <div class="diff-empty-title">Totals lie. <span class="dim">Diffs don't.</span></div>
    <p>Load a second trace to compare span counts, services, duration, errors, and LLM usage side by side.</p>
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
          <th class="primary-head"><i class="swatch swatch--a"></i> trace A · baseline</th>
          <th class="comp-head"><i class="swatch swatch--b"></i> trace B · candidate</th>
          <th>Δ</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td class="diff-label">Spans</td>
          <td>{primary?.span_count.toLocaleString() ?? '–'}</td>
          <td>{comp.span_count.toLocaleString()}</td>
          <td class="diff-delta">
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
          <td>{diffDuration(primary?.total_duration_display ?? null, comp.total_duration_display)}</td>
        </tr>
        <tr>
          <td class="diff-label">Errors</td>
          <td>{primary?.error_count.toLocaleString() ?? '–'}</td>
          <td>{comp.error_count.toLocaleString()}</td>
          <td class="diff-delta" class:diff-delta--bad={(comp.error_count) > (primary?.error_count ?? 0)} class:diff-delta--good={(comp.error_count) < (primary?.error_count ?? 0)}>
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
    <button class="diff-clear" on:click={() => comparisonState.clear()}><Icon name="close" size={12} /> Remove comparison</button>
  </div>
{/if}

<style>
  .diff-empty {
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

  .diff-eyebrow {
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

  .diff-eyebrow::before {
    content: '';
    width: 13px;
    height: 7px;
    border-radius: 2px;
    background: var(--grad-hot);
  }

  .diff-eyebrow em { font-style: normal; color: var(--color-text-faint, #5b6b84); }

  .diff-empty-title {
    font-family: var(--font-display);
    font-size: clamp(1.5rem, 3vw, 2.1rem);
    font-weight: 750;
    letter-spacing: -0.025em;
    line-height: 1.05;
    color: var(--color-text, #e9eff8);
  }

  .diff-empty-title .dim { color: var(--color-text-faint, #5b6b84); }

  .diff-empty p { font-size: 0.875rem; max-width: 400px; line-height: 1.6; }

  .diff-error { color: var(--color-danger, #f87171); font-family: var(--font-mono); font-size: 0.78rem; }

  .diff-actions {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    justify-content: center;
  }

  .diff-btn {
    padding: 0.65rem 1.2rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 10px;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    color: var(--color-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.76rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: transform 0.18s var(--ease-out), border-color 0.18s var(--ease-spring), box-shadow 0.18s var(--ease-spring);
  }

  .diff-btn:hover {
    transform: translateY(-2px);
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 40%, transparent);
  }

  .diff-btn--primary {
    background: var(--grad-cta);
    border-color: transparent;
    color: #fff;
    box-shadow: var(--shadow-cta);
  }

  .diff-btn--primary:hover {
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.28) inset, 0 14px 36px -10px rgba(37, 99, 235, 0.65);
  }

  .diff-table-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    overflow: auto;
    background: var(--color-canvas-bg, #070c16);
    gap: 1.2rem;
  }

  .diff-table {
    border-collapse: collapse;
    font-size: 0.85rem;
    background: color-mix(in srgb, var(--color-surface, #0d1626) 80%, transparent);
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 14px;
    overflow: hidden;
    box-shadow: var(--shadow-panel);
  }

  .diff-table th, .diff-table td {
    padding: 0.6rem 1.4rem;
    text-align: left;
    border-bottom: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
  }

  .diff-table th {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--color-text-faint, #5b6b84);
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
  }

  .diff-table tbody tr { transition: background 0.15s ease; }
  .diff-table tbody tr:hover { background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05)); }
  .diff-table tbody tr:last-child td { border-bottom: none; }

  .diff-table td {
    color: var(--color-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  .diff-label {
    color: var(--color-text-muted, #9aa8bd) !important;
    font-family: var(--font-display) !important;
    font-weight: 600;
    font-size: 0.8rem !important;
  }

  .swatch {
    display: inline-block;
    width: 12px;
    height: 6px;
    border-radius: 2px;
    margin-right: 0.4rem;
    vertical-align: 1px;
  }

  .swatch--a { background: var(--grad-span); }
  .swatch--b { background: var(--grad-hot); }

  .primary-head { color: var(--color-sky, #7dd3fc) !important; }

  .comp-head { color: var(--color-gold, #fcd34d) !important; }

  .diff-delta { font-weight: 600; }

  .diff-delta--good { color: var(--color-success, #34d399); }

  .diff-delta--bad { color: var(--color-danger, #f87171); }

  .diff-clear {
    padding: 0.4rem 0.9rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 8px;
    background: transparent;
    color: var(--color-text-muted, #9aa8bd);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: border-color 0.15s var(--ease-spring), color 0.15s var(--ease-spring);
  }

  .diff-clear:hover {
    border-color: color-mix(in srgb, var(--color-danger, #f87171) 55%, transparent);
    color: var(--color-danger, #f87171);
  }
</style>
