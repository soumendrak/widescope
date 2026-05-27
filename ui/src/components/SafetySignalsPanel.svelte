<script lang="ts">
  import type { SafetySignal, SafetyCategory } from '../lib/types';

  export let signals: SafetySignal[];

  const CATEGORY_LABEL: Record<SafetyCategory, string> = {
    Pii: 'PII',
    Jailbreak: 'Jailbreak',
    Refusal: 'Refusal',
    ContentPolicy: 'Policy',
    Toxicity: 'Toxicity',
    Hallucination: 'Hallucination',
    Other: 'Other',
  };

  const CATEGORY_ICON: Record<SafetyCategory, string> = {
    Pii: '🪪',
    Jailbreak: '🛡',
    Refusal: '⛔',
    ContentPolicy: '📜',
    Toxicity: '☠',
    Hallucination: '👻',
    Other: '⚠',
  };

  function signalClass(s: SafetySignal): string {
    return `signal signal--${s.category.toLowerCase()} ${s.triggered ? 'signal--triggered' : 'signal--inactive'}`;
  }

  function severityBadge(severity: string | null): string {
    if (!severity) return '';
    const s = severity.toLowerCase();
    if (['critical', 'high'].includes(s)) return 'sev sev--high';
    if (['medium', 'moderate'].includes(s)) return 'sev sev--medium';
    if (['low', 'minor'].includes(s)) return 'sev sev--low';
    return 'sev';
  }
</script>

{#if signals.length > 0}
  <div class="signals">
    {#each signals as s (s.name)}
      <div class={signalClass(s)}>
        <div class="header-row">
          <span class="icon" aria-hidden="true">{CATEGORY_ICON[s.category]}</span>
          <span class="name">{s.name}</span>
          <span class="category">{CATEGORY_LABEL[s.category]}</span>
          <span class="status">{s.triggered ? 'TRIGGERED' : 'OK'}</span>
        </div>
        {#if s.score !== null || s.severity}
          <div class="meta-row">
            {#if s.score !== null}
              <span class="score">score {s.score.toFixed(3)}</span>
            {/if}
            {#if s.severity}
              <span class={severityBadge(s.severity)}>{s.severity}</span>
            {/if}
          </div>
        {/if}
        {#if s.detail}
          <div class="detail">{s.detail}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .signals {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .signal {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    padding: 0.4rem 0.5rem;
  }

  .signal--triggered {
    border-color: rgba(248, 113, 113, 0.55);
    background: rgba(248, 113, 113, 0.08);
  }

  .signal--inactive {
    opacity: 0.7;
  }

  .header-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .icon {
    font-size: 0.95rem;
  }

  .name {
    font-weight: 600;
    font-size: 0.82rem;
    color: var(--color-text, #e2e8f0);
  }

  .category {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.06));
    color: var(--color-text-muted, #94a3b8);
    border-radius: 3px;
    padding: 0.08rem 0.35rem;
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status {
    margin-left: auto;
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
  }

  .signal--triggered .status {
    background: var(--color-danger, #f87171);
    color: white;
  }

  .signal--inactive .status {
    background: rgba(34, 197, 94, 0.18);
    color: var(--color-success, #4ade80);
  }

  .meta-row {
    margin-top: 0.3rem;
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
    font-size: 0.75rem;
  }

  .score {
    font-family: monospace;
    color: var(--color-code-muted, #cbd5e1);
  }

  .sev {
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.06));
    color: var(--color-text-muted, #94a3b8);
  }

  .sev--high {
    background: rgba(248, 113, 113, 0.2);
    color: var(--color-danger, #f87171);
  }

  .sev--medium {
    background: rgba(251, 191, 36, 0.2);
    color: #fbbf24;
  }

  .sev--low {
    background: rgba(96, 165, 250, 0.2);
    color: #60a5fa;
  }

  .detail {
    margin-top: 0.3rem;
    padding: 0.3rem 0.4rem;
    background: var(--color-bg, #0f172a);
    border-radius: 4px;
    font-size: 0.74rem;
    color: var(--color-code-muted, #cbd5e1);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
