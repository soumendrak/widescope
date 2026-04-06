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
  let sidebar: HTMLDivElement;
  let sidebarWidth = 420;
  let isResizing = false;

  const MIN_SIDEBAR_WIDTH = 320;
  const SIDEBAR_EDGE_GAP = 24;

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

  function clampSidebarWidth(nextWidth: number): number {
    const parentWidth = sidebar?.parentElement?.clientWidth ?? window.innerWidth;
    const maxWidth = Math.max(MIN_SIDEBAR_WIDTH, parentWidth - SIDEBAR_EDGE_GAP);
    return Math.min(Math.max(nextWidth, MIN_SIDEBAR_WIDTH), maxWidth);
  }

  function beginResize(event: PointerEvent): void {
    event.preventDefault();
    isResizing = true;
    sidebar?.setPointerCapture(event.pointerId);
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';
  }

  function onPointerMove(event: PointerEvent): void {
    if (!isResizing || !sidebar?.parentElement) return;
    const parentRect = sidebar.parentElement.getBoundingClientRect();
    sidebarWidth = clampSidebarWidth(parentRect.right - event.clientX);
  }

  function endResize(event?: PointerEvent): void {
    if (event && sidebar?.hasPointerCapture(event.pointerId)) {
      sidebar.releasePointerCapture(event.pointerId);
    }
    if (!isResizing) return;
    isResizing = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }

  function onWindowPointerDown(event: PointerEvent): void {
    if (!$selectedSpanId || isResizing || !sidebar) return;
    const target = event.target;
    if (target instanceof Node && sidebar.contains(target)) return;
    selectedSpanId.set(null);
  }

  onDestroy(() => {
    endResize();
    unsubSel();
  });
</script>

<svelte:window on:pointermove={onPointerMove} on:pointerup={endResize} on:pointercancel={endResize} on:pointerdown={onWindowPointerDown} />

{#if $selectedSpanId}
<aside class="sidebar" class:sidebar--resizing={isResizing} bind:this={sidebar} style={`width: ${sidebarWidth}px;`}>
  <div
    class="sidebar-resize-handle"
    role="separator"
    aria-label="Resize details sidebar"
    aria-orientation="vertical"
    on:pointerdown={beginResize}
  ></div>
    <div class="sidebar-toolbar">
      <div class="sidebar-toolbar-title">Span details</div>
    </div>
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
          <tbody>
            <tr><td>Start</td><td>{detail.start_time_display}</td></tr>
            <tr><td>Duration</td><td>{detail.duration_display}</td></tr>
            <tr><td>Self time</td><td>{detail.self_time_display}</td></tr>
            {#if $traceState.summary}
              <tr><td>% of trace</td>
                <td>{pct(detail.duration_ns, $traceState.summary.total_duration_ns)}</td>
              </tr>
            {/if}
          </tbody>
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
              <tbody>
                {#each detail.attributes as [k, v]}
                  <tr>
                    <td class="attr-key">{k}</td>
                    <td class="attr-val">{v}</td>
                  </tr>
                {/each}
              </tbody>
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
                    <tbody>
                      {#each ev.attributes as [k, v]}
                        <tr><td>{k}</td><td>{v}</td></tr>
                      {/each}
                    </tbody>
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
    {:else}
      <div class="empty-state">
        <div class="empty-state-title">Span details unavailable</div>
        <div class="empty-state-subtitle">WideScope could not load the selected span details.</div>
      </div>
    {/if}
</aside>
{/if}

<style>
  .sidebar {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 420px;
    min-width: 320px;
    max-width: calc(100% - 24px);
    overflow-y: auto;
    background: var(--color-sidebar, #1e293b);
    color: var(--color-sidebar-text, #e2e8f0);
    border-left: 1px solid var(--color-border, #334155);
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    box-shadow: -18px 0 36px rgba(15, 23, 42, 0.28);
    z-index: 2;
  }

  .sidebar--resizing {
    transition: none;
  }

  .sidebar-resize-handle {
    position: absolute;
    top: 0;
    left: -6px;
    bottom: 0;
    width: 12px;
    cursor: ew-resize;
    z-index: 3;
  }

  .sidebar-resize-handle::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 3px;
    height: 48px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-text-muted, #94a3b8) 45%, transparent);
    box-shadow: -4px 0 0 color-mix(in srgb, var(--color-text-muted, #94a3b8) 25%, transparent), 4px 0 0 color-mix(in srgb, var(--color-text-muted, #94a3b8) 25%, transparent);
  }

  .sidebar-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid var(--color-border, #334155);
    background: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .sidebar-toolbar-title {
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--color-text, #e2e8f0);
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
    background: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .op-name {
    font-size: 0.95rem;
    font-weight: 600;
    word-break: break-word;
    margin-bottom: 0.35rem;
    color: var(--color-text, #e2e8f0);
  }

  .meta-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .svc-badge {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.2));
    color: var(--color-badge-text, #93c5fd);
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
    font-size: 0.78rem;
  }

  .kind-badge {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    color: var(--color-text, #e2e8f0);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.72rem;
    font-family: monospace;
  }

  .status { font-size: 0.8rem; }
  .status.ok { color: var(--color-success, #4ade80); }
  .status.err { color: var(--color-danger, #f87171); }

  .error-msg {
    margin-top: 0.35rem;
    color: var(--color-danger, #f87171);
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
  .attr-val { color: var(--color-code-muted, #cbd5e1); }

  /* LLM */
  .llm-section { background: var(--color-llm-panel-bg, rgba(139, 92, 246, 0.07)); }

  .llm-meta {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;
  }

  .provider-badge {
    background: var(--color-llm-badge-bg, rgba(139, 92, 246, 0.25));
    color: var(--color-llm-badge-text, #c4b5fd);
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
    font-size: 0.75rem;
  }

  .model-name {
    font-family: monospace;
    font-size: 0.8rem;
    color: var(--color-code-text, #e2e8f0);
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
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
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
    color: var(--color-link, #93c5fd);
    font-weight: 600;
    min-width: 4rem;
  }

  .content {
    color: var(--color-code-muted, #cbd5e1);
    font-family: monospace;
    font-size: 0.75rem;
    white-space: pre-wrap;
    max-height: 120px;
    overflow-y: auto;
  }

  .tool-call {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    border-radius: 4px;
    padding: 0.3rem 0.4rem;
    margin-bottom: 0.3rem;
  }

  .tool-name { font-weight: 600; font-size: 0.8rem; margin-bottom: 0.15rem; color: var(--color-text, #e2e8f0); }
  .tool-args { font-family: monospace; font-size: 0.75rem; color: var(--color-code-muted, #94a3b8); white-space: pre-wrap; }

  .event-item {
    margin-bottom: 0.5rem;
    border-left: 2px solid var(--color-border, #334155);
    padding-left: 0.5rem;
  }

  .event-name { font-weight: 500; font-size: 0.82rem; color: var(--color-text, #e2e8f0); }
  .event-ts { font-size: 0.72rem; color: var(--color-text-muted, #94a3b8); margin-bottom: 0.2rem; }
  .event-attrs { margin-top: 0.2rem; }

  .children-list {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .empty-state {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1.5rem;
    text-align: center;
    color: var(--color-text-muted, #94a3b8);
  }

  .empty-state-title {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--color-text, #e2e8f0);
  }

  .empty-state-subtitle {
    max-width: 24rem;
    line-height: 1.5;
  }

  .child-link {
    background: none;
    border: 1px solid var(--color-border, #334155);
    color: var(--color-link, #93c5fd);
    font-family: monospace;
    font-size: 0.78rem;
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    cursor: pointer;
    text-align: left;
  }

  .child-link:hover {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.1));
  }
</style>
