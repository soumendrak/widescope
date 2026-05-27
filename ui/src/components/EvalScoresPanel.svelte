<script lang="ts">
  import type { EvalScore } from '../lib/types';

  export let scores: EvalScore[];

  let expanded: Record<string, boolean> = {};

  function toggle(name: string) {
    expanded = { ...expanded, [name]: !expanded[name] };
  }

  function fmtValue(score: EvalScore): string {
    if (score.value !== null) {
      // Compact 3-decimal precision; full precision shown in expanded view.
      return score.value.toFixed(3);
    }
    return score.label ?? '–';
  }

  function statusClass(score: EvalScore): string {
    if (score.passed === true) return 'status-pass';
    if (score.passed === false) return 'status-fail';
    return 'status-neutral';
  }

  function statusLabel(score: EvalScore): string {
    if (score.passed === true) return 'PASS';
    if (score.passed === false) return 'FAIL';
    return '';
  }

  /**
   * Width of the score bar as a percentage. Treats values in [0,1] as a
   * direct percentage; values outside that range fall back to a fixed width.
   */
  function barWidth(score: EvalScore): string {
    if (score.value === null) return '0%';
    if (score.value >= 0 && score.value <= 1) {
      return `${(score.value * 100).toFixed(1)}%`;
    }
    // Unknown scale — render a half-width neutral bar so the row still has
    // visual weight without misleading the user about the value.
    return '50%';
  }

  function thresholdPos(score: EvalScore): string | null {
    if (score.threshold === null) return null;
    if (score.threshold >= 0 && score.threshold <= 1) {
      return `${(score.threshold * 100).toFixed(1)}%`;
    }
    return null;
  }
</script>

{#if scores.length > 0}
  <div class="eval-list">
    {#each scores as score (score.name)}
      <div class="eval-row" class:has-explanation={!!score.explanation}>
        <div class="eval-header">
          <button
            type="button"
            class="name-button"
            on:click={() => toggle(score.name)}
            disabled={!score.explanation}
            aria-expanded={expanded[score.name] ?? false}
          >
            <span class="caret" class:hidden={!score.explanation}>
              {expanded[score.name] ? '▾' : '▸'}
            </span>
            <span class="name">{score.name}</span>
          </button>
          <span class="value-tag {statusClass(score)}">
            {fmtValue(score)}
            {#if statusLabel(score)}<span class="badge">{statusLabel(score)}</span>{/if}
          </span>
        </div>
        {#if score.value !== null && score.value >= 0 && score.value <= 1}
          <div class="bar-track" aria-hidden="true">
            <div class="bar-fill {statusClass(score)}" style={`width: ${barWidth(score)};`}></div>
            {#if thresholdPos(score)}
              <div class="threshold-marker" style={`left: ${thresholdPos(score)};`} title={`Threshold ${score.threshold}`}></div>
            {/if}
          </div>
        {/if}
        {#if score.threshold !== null && (score.value === null || score.value < 0 || score.value > 1)}
          <div class="threshold-note">Threshold: {score.threshold}</div>
        {/if}
        {#if expanded[score.name] && score.explanation}
          <div class="explanation">{score.explanation}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .eval-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .eval-row {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    padding: 0.4rem 0.5rem;
  }

  .eval-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .name-button {
    background: none;
    border: 0;
    padding: 0;
    color: inherit;
    font: inherit;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    cursor: pointer;
    text-align: left;
  }

  .name-button:disabled {
    cursor: default;
  }

  .name-button:disabled .caret {
    color: transparent;
  }

  .caret {
    font-size: 0.7rem;
    color: var(--color-text-muted, #94a3b8);
    width: 0.7rem;
    display: inline-block;
  }

  .caret.hidden {
    visibility: hidden;
  }

  .name {
    font-weight: 600;
    font-size: 0.82rem;
    color: var(--color-text, #e2e8f0);
  }

  .value-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-family: monospace;
    font-size: 0.8rem;
    font-weight: 600;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
  }

  .value-tag.status-pass {
    background: rgba(34, 197, 94, 0.18);
    color: var(--color-success, #4ade80);
  }

  .value-tag.status-fail {
    background: rgba(248, 113, 113, 0.18);
    color: var(--color-danger, #f87171);
  }

  .value-tag.status-neutral {
    color: var(--color-text, #e2e8f0);
  }

  .badge {
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 0 0.25rem;
    border-radius: 3px;
    background: rgba(0, 0, 0, 0.25);
  }

  .bar-track {
    position: relative;
    margin-top: 0.4rem;
    height: 6px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 3px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.15s ease-out;
  }

  .bar-fill.status-pass {
    background: var(--color-success, #4ade80);
  }
  .bar-fill.status-fail {
    background: var(--color-danger, #f87171);
  }
  .bar-fill.status-neutral {
    background: var(--color-link, #60a5fa);
  }

  .threshold-marker {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 1.5px;
    background: rgba(226, 232, 240, 0.8);
    transform: translateX(-50%);
  }

  .threshold-note {
    margin-top: 0.25rem;
    font-size: 0.7rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .explanation {
    margin-top: 0.4rem;
    padding: 0.35rem 0.45rem;
    background: var(--color-bg, #0f172a);
    border-radius: 4px;
    font-size: 0.75rem;
    color: var(--color-code-muted, #cbd5e1);
    white-space: pre-wrap;
  }
</style>
