<script lang="ts">
  import { filterSpans, getSpanDetail } from '../lib/wasm';
  import { selectedSpanId, hoveredSpanId } from '../stores/selection';
  import { traceState } from '../stores/trace';
  import type { SpanDetail } from '../lib/types';
  import Chip from './ui/Chip.svelte';
  import EmptyState from './ui/EmptyState.svelte';

  /**
   * The LLM-native lens: every LLM span in the trace rendered as one chat
   * transcript, in trace order, with the tokens and cost each turn actually
   * incurred. Generic trace viewers show these as opaque spans; this is the
   * view that reads the way the pipeline was written.
   */

  let turns: SpanDetail[] = [];
  let loadError: string | null = null;

  // Rebuilt whenever a new trace lands.
  $: turns = buildTurns($traceState.status);

  function buildTurns(status: string): SpanDetail[] {
    if (status !== 'loaded') return [];
    loadError = null;
    try {
      return filterSpans({ llm_only: true })
        .map((id) => {
          try {
            return getSpanDetail(id);
          } catch {
            return null;
          }
        })
        .filter((d): d is SpanDetail => d !== null && d.llm !== null)
        // filterSpans does NOT return trace order, and a transcript out of
        // order is worse than none. start_time_ns is a nanosecond string, so
        // compare as BigInt rather than Number.
        .sort((a, b) => {
          const ta = BigInt(a.start_time_ns);
          const tb = BigInt(b.start_time_ns);
          return ta < tb ? -1 : ta > tb ? 1 : 0;
        });
    } catch (e) {
      loadError = String(e);
      return [];
    }
  }

  function fmtCost(usd: number | null): string | null {
    if (usd === null || usd === 0) return null;
    return usd < 0.01 ? `$${usd.toFixed(6)}` : `$${usd.toFixed(4)}`;
  }

  /** Collapse the assorted provider role names onto the three we style. */
  function roleClass(role: string): 'user' | 'assistant' | 'system' {
    const r = role.toLowerCase();
    if (r === 'assistant' || r === 'ai' || r === 'model') return 'assistant';
    if (r === 'system' || r === 'developer') return 'system';
    return 'user';
  }

  function select(id: string): void {
    selectedSpanId.set(id);
  }
</script>

<div class="conversation">
  {#if loadError}
    <EmptyState icon="warning" tone="danger" title="Could not read LLM spans" description={loadError} />
  {:else if turns.length === 0}
    <EmptyState
      icon="bolt"
      tone="muted"
      title="No LLM spans in this trace"
      description="Spans carrying model, prompt, or completion attributes appear here as a transcript."
    />
  {:else}
    <ol class="turns">
      {#each turns as turn, i (turn.span_id)}
        {@const llm = turn.llm}
        <li
          class="turn"
          class:turn--selected={$selectedSpanId === turn.span_id}
          on:mouseenter={() => hoveredSpanId.set(turn.span_id)}
          on:mouseleave={() => hoveredSpanId.set(null)}
        >
          <button type="button" class="turn-head" on:click={() => select(turn.span_id)}>
            <span class="turn-idx">{i + 1}</span>
            <span class="turn-op">{turn.operation_name}</span>
            {#if llm?.model_name}
              <Chip tone="llm">{llm.model_name}</Chip>
            {/if}
            <span class="turn-spacer"></span>
            {#if llm?.input_tokens !== null || llm?.output_tokens !== null}
              <span class="turn-meta" title="input → output tokens">
                {llm?.input_tokens ?? 0} → {llm?.output_tokens ?? 0} tok
              </span>
            {/if}
            {#if fmtCost(llm?.estimated_cost_usd ?? null)}
              <span class="turn-meta turn-meta--cost">{fmtCost(llm?.estimated_cost_usd ?? null)}</span>
            {/if}
            <span class="turn-meta">{turn.duration_display}</span>
            {#if turn.status === 'Error'}
              <Chip tone="danger">error</Chip>
            {/if}
          </button>

          <div class="messages">
            {#each llm?.input_messages ?? [] as msg}
              <div class="msg msg--{roleClass(msg.role)}">
                <div class="msg-role">{msg.role}</div>
                <div class="msg-body">{msg.content ?? '—'}</div>
              </div>
            {/each}

            {#each llm?.tool_calls ?? [] as call}
              <div class="msg msg--tool">
                <div class="msg-role">tool · {call.name}</div>
                {#if call.arguments}<pre class="msg-code">{call.arguments}</pre>{/if}
                {#if call.result}<pre class="msg-code msg-code--result">{call.result}</pre>{/if}
              </div>
            {/each}

            {#each llm?.output_messages ?? [] as msg}
              <div class="msg msg--{roleClass(msg.role)}">
                <div class="msg-role">{msg.role}</div>
                <div class="msg-body">{msg.content ?? '—'}</div>
              </div>
            {/each}

            {#if (llm?.input_messages?.length ?? 0) === 0 && (llm?.output_messages?.length ?? 0) === 0 && (llm?.tool_calls?.length ?? 0) === 0}
              <div class="msg msg--absent">
                No prompt or completion recorded on this span — only usage metadata.
              </div>
            {/if}
          </div>

          {#if llm?.retrieved_documents?.length}
            <div class="rag">
              {llm.retrieved_documents.length} retrieved document{llm.retrieved_documents.length === 1 ? '' : 's'}
              — see the inspector for full text
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .conversation {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .turns {
    list-style: none;
    margin: 0;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .turn {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    overflow: hidden;
  }

  .turn--selected { border-color: var(--color-accent); }

  .turn-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: 0;
    border-bottom: 1px solid var(--color-border-soft);
    background: var(--color-panel-subtle);
    color: var(--color-text);
    font-family: inherit;
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .turn-head:hover { background: var(--color-panel-highlight); }

  .turn-idx {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.35rem;
    height: 1.35rem;
    border-radius: var(--radius-pill);
    background: var(--color-badge-bg);
    color: var(--color-badge-text);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    flex: none;
  }

  .turn-op {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .turn-spacer { flex: 1; }

  .turn-meta {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  .turn-meta--cost { color: var(--color-gold); }

  .messages {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
  }

  .msg {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border-left: 3px solid transparent;
    background: var(--color-panel-subtle);
    max-width: 78ch;
  }

  .msg--system { border-left-color: var(--color-text-faint); }
  .msg--user { border-left-color: var(--color-sky); }
  .msg--assistant {
    border-left-color: var(--color-accent);
    align-self: flex-end;
    background: var(--color-badge-bg);
  }
  .msg--tool { border-left-color: var(--color-amber); }
  .msg--absent {
    border-left-color: transparent;
    background: none;
    color: var(--color-text-faint);
    font-size: var(--text-xs);
    font-style: italic;
  }

  .msg-role {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-text-muted);
    margin-bottom: var(--space-1);
  }

  .msg-body {
    font-size: var(--text-sm);
    line-height: 1.6;
    color: var(--color-text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .msg-code {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    line-height: 1.5;
    color: var(--color-code-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    margin-top: var(--space-1);
  }

  .msg-code--result { color: var(--color-success); }

  .rag {
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--color-border-soft);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-muted);
  }

  @media (max-width: 820px) {
    .msg--assistant { align-self: stretch; }
  }
</style>
