<script lang="ts">
  import type { AgentFlow } from '../lib/types';
  import { selectedSpanId } from '../stores/selection';

  export let flow: AgentFlow;

  // ponytail: linearize the agent flow by start order — each agentic span is one
  // row (thought → tool call → result). Reuses the flow data the DAG view already
  // loads; no extra WASM call. Layer drives left indentation so nested loops read.
  const KIND_ICON: Record<string, string> = {
    agent: '🤔',
    tool: '🔧',
    chain: '⛓️',
    retrieval: '📄',
    llm: '💬',
  };

  $: steps = [...(flow?.nodes ?? [])].sort((a, b) => a.order - b.order);

  function select(spanId: string) {
    selectedSpanId.set(spanId);
  }
</script>

{#if !steps.length}
  <div class="empty">No agentic spans detected in this trace</div>
{:else}
  <div class="timeline-scroll">
    <div class="timeline">
      {#each steps as step, i (step.span_id)}
        <div
          class="step"
          class:step--selected={$selectedSpanId === step.span_id}
          class:step--error={step.status === 'Error'}
          style={`margin-left: ${step.layer * 1.5}rem;`}
          role="button"
          tabindex="0"
          aria-label="{step.kind} {step.label}"
          on:click={() => select(step.span_id)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), select(step.span_id))}
        >
          <div class="step-head">
            <span class="step-num">{i + 1}</span>
            <span class="step-icon" aria-hidden="true">{KIND_ICON[step.kind] ?? '•'}</span>
            <span class="step-kind">{step.kind}</span>
            <span class="step-label" title={step.label}>{step.label}</span>
            {#if step.iteration !== null && step.iteration > 0}
              <span class="step-iter">⟳{step.iteration + 1}</span>
            {/if}
            <span class="step-dur">{step.duration_display}</span>
            {#if step.status === 'Error'}<span class="step-err">error</span>{/if}
          </div>
          {#if step.detail}
            <div class="step-thought">{step.detail}</div>
          {/if}
          {#if step.arguments}
            <div class="step-field"><span class="field-label">args</span><code>{step.arguments}</code></div>
          {/if}
          {#if step.result}
            <div class="step-field"><span class="field-label">result</span><code>{step.result}</code></div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .timeline-scroll {
    flex: 1;
    overflow: auto;
    background: var(--color-canvas-bg, #0f172a);
    padding: 1rem;
  }

  .timeline {
    list-style: none;
    margin: 0;
    padding: 0;
    max-width: 720px;
  }

  .step {
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-left: 3px solid var(--color-accent, #3b82f6);
    border-radius: 8px;
    padding: 0.5rem 0.7rem;
    margin-bottom: 0.5rem;
    background: color-mix(in srgb, var(--color-sidebar, #0a111e) 70%, transparent);
    cursor: pointer;
    transition: border-color 0.15s ease, transform 0.15s ease;
  }

  .step:hover { transform: translateX(2px); }

  .step--selected { border-color: var(--color-sky, #7dd3fc); }
  .step--error { border-left-color: var(--color-danger, #f87171); }

  .step:focus-visible {
    outline: 2px solid var(--color-accent, #3b82f6);
    outline-offset: 2px;
  }

  .step-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--color-text, #e9eff8);
  }

  .step-num {
    color: var(--color-text-faint, #5b6b84);
    font-size: 0.68rem;
    min-width: 1.4rem;
  }

  .step-kind {
    text-transform: uppercase;
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    color: var(--color-text-muted, #94a3b8);
  }

  .step-label {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .step-iter { color: var(--color-gold, #fcd34d); font-size: 0.7rem; }

  .step-dur {
    margin-left: auto;
    color: var(--color-text-faint, #5b6b84);
    font-size: 0.68rem;
  }

  .step-err {
    color: var(--color-danger, #f87171);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .step-thought {
    margin-top: 0.35rem;
    font-size: 0.74rem;
    line-height: 1.5;
    color: var(--color-code-muted, #b9c5d8);
    white-space: pre-wrap;
    max-height: 120px;
    overflow-y: auto;
  }

  .step-field {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.3rem;
    font-family: var(--font-mono);
    font-size: 0.7rem;
  }

  .field-label {
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.58rem;
    color: var(--color-text-faint, #5b6b84);
    min-width: 3.2rem;
    padding-top: 0.1rem;
  }

  .step-field code {
    color: var(--color-code-muted, #b9c5d8);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 120px;
    overflow-y: auto;
  }

  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.9rem;
    background: var(--color-canvas-bg, #0f172a);
  }
</style>
