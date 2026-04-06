<script lang="ts">
  import { onDestroy } from 'svelte';
  import { selectedSpanId } from '../stores/selection';
  import { traceState } from '../stores/trace';
  import { getSpanDetail } from '../lib/wasm';
  import type { SpanDetail, LlmDetail } from '../lib/types';

  let detail: SpanDetail | null = null;
  let loading = false;
  let expandedAttrs = false;
  let expandedEvents = false;

  const unsubSel = selectedSpanId.subscribe(async (id) => {
    if (!id || $traceState.status !== 'loaded') {
      detail = null;
      return;
    }
    loading = true;
    try {
      detail = getSpanDetail(id);
    } catch {
      detail = null;
    }
    loading = false;
  });

  onDestroy(unsubSel);

  function pct(ns: number, totalNs: number): string {
    if (!totalNs) return '0%';
    return ((ns / totalNs) * 100).toFixed(1) + '%';
  }

  const STATUS_ICONS: Record<string, string> = { Ok: '✓', Error: '✕', Unset: '–' };

  function kindBadge(kind: string): string {
    const map: Record<string, string> = {
      Server: 'SRV', Client: 'CLI', Internal: 'INT', Producer: 'PRD', Consumer: 'CNS',
    };
    return map[kind] ?? kind;
  }
</script>

{#if detail || loading}
  <aside class="sidebar">
    {#if loading}
      <div class="loading">Loading…</div>
    {:else if detail}
      <!-- Header -->
      <div class="section header">
        <div class="op-name" title={detail.operation_name}>{detail.operation_name}</div>
        <div class="meta-row">
          <span class="svc-badge">{detail.service_name}</span>
          <span class="kind-badge">{kindBadge(detail.span_kind)}</span>
          <span class="status" class:ok={detail.status === 'Ok'} class:err={detail.status === 'Error'}>
            {STATUS_ICONS[detail.status] ?? detail.status} {detail.status}
          </span>
        </div>
        {#if detail.error_message}
          <div class="error-msg">{detail.error_message}</div>
        {/if}
      </div>

      <!-- Timing -->
      <div class="section">
        <div class="section-title">Timing</div>
        <table class="kv-table">
          <tr><td>Start</td><td>{detail.start_time_display}</td></tr>
          <tr><td>Duration</td><td>{detail.duration_display}</td></tr>
          <tr><td>Self time</td><td>{detail.self_time_display}</td></tr>
          {#if $traceState.summary}
            <tr><td>% of trace</td>
              <td>{pct(detail.duration_ns, $traceState.summary.total_duration_ns)}</td>
            </tr>
          {/if}
        </table>
      </div>

      <!-- LLM Panel -->
      {#if detail.llm}
        {@const llm = detail.llm}
        <div class="section llm-section">
          <div class="section-title">🤖 LLM</div>
          <div class="llm-meta">
            {#if llm.model_provider}<span class="provider-badge">{llm.model_provider}</span>{/if}
            {#if llm.model_name}<span class="model-name">{llm.model_name}</span>{/if}
            <span class="op-type">{llm.operation_type}</span>
          </div>
          {#if llm.input_tokens !== null || llm.output_tokens !== null}
            <div class="token-row">
              <div class="token-item">
                <div class="token-label">Input</div>
                <div class="token-val">{llm.input_tokens?.toLocaleString() ?? '–'}</div>
              </div>
              <div class="token-item">
                <div class="token-label">Output</div>
                <div class="token-val">{llm.output_tokens?.toLocaleString() ?? '–'}</div>
              </div>
              <div class="token-item">
                <div class="token-label">Total</div>
                <div class="token-val">{llm.total_tokens?.toLocaleString() ?? '–'}</div>
              </div>
            </div>
          {/if}
          {#if llm.temperature !== null}
            <div class="kv-inline"><span>Temp</span><span>{llm.temperature}</span></div>
          {/if}
          {#if llm.input_messages.length > 0}
            <div class="msg-group">
              <div class="msg-label">Prompt</div>
              {#each llm.input_messages as m}
                <div class="message"><span class="role">{m.role}</span><span class="content">{m.content ?? ''}</span></div>
              {/each}
            </div>
          {/if}
          {#if llm.output_messages.length > 0}
            <div class="msg-group">
              <div class="msg-label">Completion</div>
              {#each llm.output_messages as m}
                <div class="message"><span class="role">{m.role}</span><span class="content">{m.content ?? ''}</span></div>
              {/each}
            </div>
          {/if}
          {#if llm.tool_calls.length > 0}
            <div class="msg-group">
              <div class="msg-label">Tool calls ({llm.tool_calls.length})</div>
              {#each llm.tool_calls as tc}
                <div class="tool-call">
                  <div class="tool-name">{tc.name}</div>
                  {#if tc.arguments}<div class="tool-args">{tc.arguments}</div>{/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Attributes -->
      {#if detail.attributes.length > 0}
        <div class="section">
          <div class="section-title" role="button" tabindex="0"
            on:click={() => (expandedAttrs = !expandedAttrs)}
            on:keydown={(e) => e.key === 'Enter' && (expandedAttrs = !expandedAttrs)}>
            Attributes ({detail.attributes.length}) {expandedAttrs ? '▾' : '▸'}
          </div>
          {#if expandedAttrs}
            <table class="kv-table attr-table">
              {#each detail.attributes as [k, v]}
                <tr>
                  <td class="attr-key">{k}</td>
                  <td class="attr-val">{v}</td>
                </tr>
              {/each}
            </table>
          {/if}
        </div>
      {/if}

      <!-- Events -->
      {#if detail.events.length > 0}
        <div class="section">
          <div class="section-title" role="button" tabindex="0"
            on:click={() => (expandedEvents = !expandedEvents)}
            on:keydown={(e) => e.key === 'Enter' && (expandedEvents = !expandedEvents)}>
            Events ({detail.events.length}) {expandedEvents ? '▾' : '▸'}
          </div>
          {#if expandedEvents}
            {#each detail.events as ev}
              <div class="event-item">
                <div class="event-name">{ev.name}</div>
                <div class="event-ts">{ev.timestamp_display}</div>
                {#if ev.attributes.length > 0}
                  <table class="kv-table event-attrs">
                    {#each ev.attributes as [k, v]}
                      <tr><td>{k}</td><td>{v}</td></tr>
                    {/each}
                  </table>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      {/if}

      <!-- Children -->
      {#if detail.children_ids.length > 0}
        <div class="section">
          <div class="section-title">Children ({detail.children_ids.length})</div>
          <div class="children-list">
            {#each detail.children_ids as cid}
              <button class="child-link" on:click={() => selectedSpanId.set(cid)}>
                {cid.slice(0, 16)}…
              </button>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </aside>
{/if}

<style>
  .sidebar {
    width: 380px;
    min-width: 300px;
    max-width: 580px;
    flex-shrink: 0;
    overflow-y: auto;
    background: var(--color-sidebar, #1e293b);
    color: var(--color-sidebar-text, #e2e8f0);
    border-left: 1px solid var(--color-border, #334155);
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
  }

  .loading {
    padding: 1rem;
    color: var(--color-text-muted, #94a3b8);
    font-style: italic;
  }

  .section {
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid var(--color-border, #334155);
  }

  .section-title {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, #94a3b8);
    margin-bottom: 0.45rem;
    cursor: pointer;
    user-select: none;
  }

  .header {
    background: rgba(255, 255, 255, 0.04);
  }

  .op-name {
    font-size: 0.95rem;
    font-weight: 600;
    word-break: break-word;
    margin-bottom: 0.35rem;
    color: #f1f5f9;
  }

  .meta-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .svc-badge {
    background: rgba(59, 130, 246, 0.2);
    color: #93c5fd;
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
    font-size: 0.78rem;
  }

  .kind-badge {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.72rem;
    font-family: monospace;
  }

  .status { font-size: 0.8rem; }
  .status.ok { color: #4ade80; }
  .status.err { color: #f87171; }

  .error-msg {
    margin-top: 0.35rem;
    color: #f87171;
    font-size: 0.8rem;
    font-family: monospace;
    word-break: break-word;
  }

  .kv-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
  }

  .kv-table td {
    padding: 0.18rem 0;
    vertical-align: top;
  }

  .kv-table td:first-child {
    color: var(--color-text-muted, #94a3b8);
    width: 40%;
    padding-right: 0.5rem;
    white-space: nowrap;
  }

  .kv-table td:last-child {
    word-break: break-all;
    font-family: monospace;
    font-size: 0.78rem;
  }

  .attr-table td:first-child { width: 45%; }
  .attr-key { font-size: 0.78rem !important; }
  .attr-val { color: #cbd5e1; }

  /* LLM */
  .llm-section { background: rgba(139, 92, 246, 0.07); }

  .llm-meta {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;
  }

  .provider-badge {
    background: rgba(139, 92, 246, 0.25);
    color: #c4b5fd;
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
    font-size: 0.75rem;
  }

  .model-name {
    font-family: monospace;
    font-size: 0.8rem;
    color: #e2e8f0;
  }

  .op-type {
    font-size: 0.75rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .token-row {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 0.45rem;
  }

  .token-item {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
    padding: 0.3rem 0.4rem;
    text-align: center;
  }

  .token-label {
    font-size: 0.68rem;
    color: var(--color-text-muted, #94a3b8);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .token-val {
    font-size: 0.9rem;
    font-weight: 600;
    font-family: monospace;
    margin-top: 0.1rem;
  }

  .kv-inline {
    display: flex;
    gap: 0.5rem;
    font-size: 0.8rem;
    margin-bottom: 0.25rem;
  }

  .kv-inline span:first-child { color: var(--color-text-muted, #94a3b8); }

  .msg-group {
    margin-top: 0.5rem;
  }

  .msg-label {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-muted, #94a3b8);
    margin-bottom: 0.25rem;
  }

  .message {
    display: flex;
    gap: 0.4rem;
    font-size: 0.78rem;
    margin-bottom: 0.25rem;
    word-break: break-word;
  }

  .role {
    flex-shrink: 0;
    color: #93c5fd;
    font-weight: 600;
    min-width: 4rem;
  }

  .content {
    color: #cbd5e1;
    font-family: monospace;
    font-size: 0.75rem;
    white-space: pre-wrap;
    max-height: 120px;
    overflow-y: auto;
  }

  .tool-call {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
    padding: 0.3rem 0.4rem;
    margin-bottom: 0.3rem;
  }

  .tool-name { font-weight: 600; font-size: 0.8rem; margin-bottom: 0.15rem; }
  .tool-args { font-family: monospace; font-size: 0.75rem; color: #94a3b8; white-space: pre-wrap; }

  .event-item {
    margin-bottom: 0.5rem;
    border-left: 2px solid var(--color-border, #334155);
    padding-left: 0.5rem;
  }

  .event-name { font-weight: 500; font-size: 0.82rem; }
  .event-ts { font-size: 0.72rem; color: var(--color-text-muted, #94a3b8); margin-bottom: 0.2rem; }
  .event-attrs { margin-top: 0.2rem; }

  .children-list {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .child-link {
    background: none;
    border: 1px solid var(--color-border, #334155);
    color: #93c5fd;
    font-family: monospace;
    font-size: 0.78rem;
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    cursor: pointer;
    text-align: left;
  }

  .child-link:hover {
    background: rgba(59, 130, 246, 0.1);
  }
</style>
