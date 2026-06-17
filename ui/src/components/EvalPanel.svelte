<script lang="ts">
  import type { EvalScore } from '../lib/types';

  export let scores: EvalScore[];

  // pass/fail coloring takes priority; fall back to value banding when no
  // threshold/passed info is available.
  function statusClass(s: EvalScore): string {
    if (s.passed === true) return 'score score--high';
    if (s.passed === false) return 'score score--low';
    if (s.value >= 0.7) return 'score score--high';
    if (s.value >= 0.4) return 'score score--mid';
    return 'score score--low';
  }

  function barWidth(value: number): string {
    const clamped = Math.max(0, Math.min(1, value));
    return `${(clamped * 100).toFixed(1)}%`;
  }

  function thresholdLeft(threshold: number | null): string | null {
    if (threshold === null) return null;
    const clamped = Math.max(0, Math.min(1, threshold));
    return `${(clamped * 100).toFixed(1)}%`;
  }

  function fmt(n: number): string {
    return n.toFixed(3);
  }
</script>

{#if scores.length > 0}
  <div class="eval-list">
    {#each scores as s (s.name)}
      <div class="eval">
        <div class="eval-header">
          <span class="name">{s.name}</span>
          {#if s.passed !== null}
            <span class={s.passed ? 'badge badge--pass' : 'badge badge--fail'}>
              {s.passed ? 'PASS' : 'FAIL'}
            </span>
          {/if}
          <span class={statusClass(s)}>{fmt(s.value)}</span>
        </div>
        <div class="bar-track" aria-hidden="true">
          <div class={statusClass(s) + ' bar-fill'} style={`width: ${barWidth(s.value)};`}></div>
          {#if thresholdLeft(s.threshold)}
            <div
              class="threshold-mark"
              style={`left: ${thresholdLeft(s.threshold)};`}
              title={`threshold ${fmt(s.threshold ?? 0)}`}
            ></div>
          {/if}
        </div>
        {#if s.threshold !== null}
          <div class="threshold-label">threshold {fmt(s.threshold)}</div>
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

  .eval {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    padding: 0.45rem 0.55rem;
  }

  .eval-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .name {
    flex: 1;
    font-size: 0.8rem;
    color: var(--color-text, #e2e8f0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    font-family: monospace;
    font-size: 0.66rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
  }

  .badge--pass {
    background: rgba(34, 197, 94, 0.18);
    color: var(--color-success, #4ade80);
  }

  .badge--fail {
    background: rgba(248, 113, 113, 0.18);
    color: var(--color-danger, #f87171);
  }

  .score {
    font-family: monospace;
    font-size: 0.78rem;
    font-weight: 600;
    padding: 0.05rem 0.4rem;
    border-radius: 3px;
  }

  .score--high {
    background: rgba(34, 197, 94, 0.18);
    color: var(--color-success, #4ade80);
  }

  .score--mid {
    background: rgba(251, 191, 36, 0.18);
    color: #fbbf24;
  }

  .score--low {
    background: rgba(248, 113, 113, 0.18);
    color: var(--color-danger, #f87171);
  }

  .bar-track {
    position: relative;
    margin-top: 0.35rem;
    height: 5px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    padding: 0;
  }

  .bar-fill.score--high { background: var(--color-success, #4ade80); }
  .bar-fill.score--mid { background: #fbbf24; }
  .bar-fill.score--low { background: var(--color-danger, #f87171); }

  .threshold-mark {
    position: absolute;
    top: -1px;
    width: 2px;
    height: 7px;
    background: var(--color-text, #e2e8f0);
    opacity: 0.85;
  }

  .threshold-label {
    margin-top: 0.25rem;
    font-family: monospace;
    font-size: 0.68rem;
    color: var(--color-text-muted, #94a3b8);
  }
</style>
