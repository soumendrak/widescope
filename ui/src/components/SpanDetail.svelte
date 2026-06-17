<script lang="ts">
  import { onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';
  import { selectedSpanId } from '../stores/selection';
  import { traceState } from '../stores/trace';
  import { getSpanDetail } from '../lib/wasm';
  import { annotations } from '../stores/annotations';
  import type { SpanDetail, LlmDetail } from '../lib/types';
  import { flyRightConfig } from '../lib/animation';
  import RetrievedDocumentsPanel from './RetrievedDocumentsPanel.svelte';
  import LlmChatReplay from './LlmChatReplay.svelte';

  let detail: SpanDetail | null = null;
  let loading = false;
  let llmView: 'chat' | 'raw' = 'chat';
  let expandedAttrs = false;
  let expandedEvents = false;
  let sidebar: HTMLElement;
  let sidebarWidth = 420;
  let isResizing = false;
  let noteText = '';

  const MIN_SIDEBAR_WIDTH = 320;
  const SIDEBAR_EDGE_GAP = 24;

  const unsubSel = selectedSpanId.subscribe(async (id) => {
    if (!id || $traceState.status !== 'loaded') {
      detail = null;
      noteText = '';
      return;
    }
    loading = true;
    try {
      detail = getSpanDetail(id);
      let currentNote = '';
      annotations.subscribe((a) => { currentNote = a[id] ?? ''; })();
      noteText = currentNote;
    } catch {
      detail = null;
    }
    loading = false;
  });

  function saveNote() {
    const id = $selectedSpanId;
    if (!id) return;
    annotations.setNote(id, noteText);
  }

  function onNoteKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      saveNote();
    }
  }

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
<aside class="sidebar" class:sidebar--resizing={isResizing} bind:this={sidebar} style={`width: ${sidebarWidth}px;`} transition:fly={flyRightConfig()}>
  <div
    class="sidebar-resize-handle"
    role="separator"
    aria-label="Resize details sidebar"
    aria-orientation="vertical"
    on:pointerdown={beginResize}
  ></div>
    <div class="sidebar-toolbar">
      <div class="sidebar-toolbar-title"><span class="sq" aria-hidden="true"></span> span inspector</div>
      {#if detail}
        <span class="sidebar-status" class:ok={detail.status === 'Ok'} class:err={detail.status === 'Error'}>
          {detail.status === 'Ok' ? 'OK' : detail.status === 'Error' ? 'ERR' : '—'}
        </span>
      {/if}
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
          <div class="section-title"><span class="llm-sq" aria-hidden="true"></span> LLM · resolved</div>
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
            {#if llm.total_tokens}
              <div class="tokbar" aria-hidden="true">
                <i class="tokbar-in" style={`width: ${Math.min(100, ((llm.input_tokens ?? 0) / llm.total_tokens) * 100)}%;`}></i>
                <i class="tokbar-out" style={`width: ${Math.min(100, ((llm.output_tokens ?? 0) / llm.total_tokens) * 100)}%;`}></i>
              </div>
              <div class="tokbar-legend">
                <span><i class="dot dot--in"></i>prompt</span>
                <span><i class="dot dot--out"></i>completion</span>
              </div>
            {/if}
          {/if}
          {#if llm.temperature !== null}
            <div class="kv-inline"><span>Temp</span><span>{llm.temperature}</span></div>
          {/if}
          {#if llm.input_messages.length > 0 || llm.output_messages.length > 0 || llm.tool_calls.length > 0}
            <div class="llm-view-toggle" role="tablist" aria-label="LLM message view">
              <button type="button" role="tab" class:active={llmView === 'chat'} aria-selected={llmView === 'chat'} on:click={() => (llmView = 'chat')}>Chat</button>
              <button type="button" role="tab" class:active={llmView === 'raw'} aria-selected={llmView === 'raw'} on:click={() => (llmView = 'raw')}>Raw</button>
            </div>
            {#if llmView === 'chat'}
              <LlmChatReplay {llm} />
            {:else}
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
            {/if}
          {/if}
          {#if llm.retrieved_documents.length > 0}
            <div class="msg-group">
              <div class="msg-label">Retrieved documents ({llm.retrieved_documents.length})</div>
              <RetrievedDocumentsPanel documents={llm.retrieved_documents} />
            </div>
          {/if}
        </div>
      {/if}

      <!-- Safety signals -->
      {#if detail.safety.length > 0}
        <div class="section safety-section">
          <div class="section-title"><span class="safety-sq" aria-hidden="true"></span> Safety · {detail.safety.length} signal{detail.safety.length === 1 ? '' : 's'}</div>
          {#each detail.safety as sig}
            <div class="safety-row safety-row--{sig.severity}">
              <span class="safety-cat">{sig.category.replace('_', ' ')}</span>
              <span class="safety-sev">{sig.severity}</span>
              <span class="safety-detail">{sig.detail}</span>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Annotations -->
      <div class="section annotation-section">
        <div class="section-title">📝 Note</div>
        <textarea
          class="annotation-input"
          placeholder="Add a note for this span…"
          bind:value={noteText}
          on:blur={saveNote}
          on:keydown={onNoteKeyDown}
          rows="3"
        ></textarea>
        <div class="annotation-hint">Enter to save, Shift+Enter for newline</div>
      </div>

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
    background: color-mix(in srgb, var(--color-sidebar, #0a111e) 94%, transparent);
    backdrop-filter: blur(12px);
    color: var(--color-sidebar-text, #e9eff8);
    border-left: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    box-shadow: -28px 0 60px rgba(2, 6, 18, 0.55);
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
    border-bottom: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
    position: sticky;
    top: 0;
    z-index: 2;
    backdrop-filter: blur(8px);
  }

  .sidebar-toolbar-title {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    font-weight: 500;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-sky, #7dd3fc);
  }

  .sidebar-toolbar-title .sq {
    width: 12px;
    height: 7px;
    border-radius: 2px;
    background: var(--grad-span);
  }

  .sidebar-status {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    font-weight: 600;
    letter-spacing: 0.14em;
    padding: 0.14rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    color: var(--color-text-muted, #9aa8bd);
  }

  .sidebar-status.ok {
    color: var(--color-success, #34d399);
    border-color: color-mix(in srgb, var(--color-success, #34d399) 38%, transparent);
  }

  .sidebar-status.err {
    color: var(--color-danger, #f87171);
    border-color: color-mix(in srgb, var(--color-danger, #f87171) 42%, transparent);
  }

  .loading {
    padding: 1rem;
    color: var(--color-text-muted, #9aa8bd);
    font-family: var(--font-mono);
    font-size: 0.74rem;
    letter-spacing: 0.08em;
  }

  .section {
    padding: 0.7rem 0.85rem;
    border-bottom: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.18em;
    color: var(--color-text-faint, #5b6b84);
    margin-bottom: 0.5rem;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }

  .section-title::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--color-border-soft, rgba(125, 211, 252, 0.07));
  }

  .header {
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
  }

  .op-name {
    font-family: var(--font-mono);
    font-size: 0.92rem;
    font-weight: 600;
    word-break: break-word;
    margin-bottom: 0.4rem;
    color: var(--color-text, #e9eff8);
  }

  .meta-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .svc-badge {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.16));
    color: var(--color-badge-text, #7dd3fc);
    border: 1px solid color-mix(in srgb, var(--color-sky, #7dd3fc) 22%, transparent);
    border-radius: 999px;
    padding: 0.1rem 0.55rem;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    letter-spacing: 0.04em;
  }

  .kind-badge {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    color: var(--color-text-muted, #9aa8bd);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
    font-size: 0.64rem;
    letter-spacing: 0.1em;
    font-family: var(--font-mono);
  }

  .status { font-family: var(--font-mono); font-size: 0.74rem; letter-spacing: 0.05em; }
  .status.ok { color: var(--color-success, #34d399); }
  .status.err { color: var(--color-danger, #f87171); }

  .error-msg {
    margin-top: 0.4rem;
    color: var(--color-error-text, #fca5a5);
    background: var(--color-error-bg, rgba(69, 10, 10, 0.65));
    border: 1px solid var(--color-error-border, rgba(153, 27, 27, 0.8));
    border-radius: 8px;
    padding: 0.4rem 0.6rem;
    font-size: 0.74rem;
    font-family: var(--font-mono);
    word-break: break-word;
  }

  .kv-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }

  .kv-table tr {
    border-bottom: 1px dashed var(--color-border-soft, rgba(125, 211, 252, 0.07));
  }

  .kv-table tr:last-child { border-bottom: none; }

  .kv-table td {
    padding: 0.26rem 0;
    vertical-align: top;
  }

  .kv-table td:first-child {
    color: var(--color-text-faint, #5b6b84);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    width: 40%;
    padding-right: 0.5rem;
    white-space: nowrap;
  }

  .kv-table td:last-child {
    word-break: break-all;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    color: var(--color-code-text, #e9eff8);
  }

  .attr-table td:first-child { width: 45%; }
  .attr-key { font-size: 0.72rem !important; }
  .attr-val { color: var(--color-code-muted, #b9c5d8); }

  /* LLM — hot amber, matching the landing hot-span treatment */
  .llm-section {
    background: var(--color-llm-panel-bg, rgba(245, 158, 11, 0.06));
    border-left: 2px solid color-mix(in srgb, var(--color-amber, #f59e0b) 55%, transparent);
  }

  .llm-section .section-title { color: var(--color-gold, #fcd34d); }

  .llm-sq {
    width: 12px;
    height: 7px;
    border-radius: 2px;
    background: var(--grad-hot);
    flex: none;
  }

  .llm-meta {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .provider-badge {
    background: var(--color-llm-badge-bg, rgba(245, 158, 11, 0.16));
    color: var(--color-llm-badge-text, #fcd34d);
    border: 1px solid color-mix(in srgb, var(--color-gold, #fcd34d) 28%, transparent);
    border-radius: 999px;
    padding: 0.1rem 0.55rem;
    font-family: var(--font-mono);
    font-size: 0.66rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .model-name {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--color-code-text, #e9eff8);
  }

  .op-type {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    color: var(--color-text-faint, #5b6b84);
    letter-spacing: 0.06em;
  }

  .llm-view-toggle {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    margin-bottom: 0.5rem;
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 7px;
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 60%, transparent);
  }

  .llm-view-toggle button {
    background: none;
    border: 0;
    border-radius: 5px;
    padding: 0.2rem 0.7rem;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-text-muted, #9aa8bd);
    transition: background 0.15s var(--ease-spring, ease), color 0.15s var(--ease-spring, ease);
  }

  .llm-view-toggle button.active {
    background: var(--color-llm-badge-bg, rgba(245, 158, 11, 0.16));
    color: var(--color-gold, #fcd34d);
  }

  .token-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.45rem;
  }

  .token-item {
    flex: 1;
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 65%, transparent);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 8px;
    padding: 0.38rem 0.4rem;
    text-align: center;
  }

  .token-label {
    font-family: var(--font-mono);
    font-size: 0.58rem;
    color: var(--color-text-faint, #5b6b84);
    text-transform: uppercase;
    letter-spacing: 0.14em;
  }

  .token-val {
    font-size: 0.88rem;
    font-weight: 600;
    font-family: var(--font-mono);
    margin-top: 0.12rem;
    color: var(--color-text, #e9eff8);
  }

  /* token budget bar, straight off the landing inspector */
  .tokbar {
    display: flex;
    height: 8px;
    border-radius: 999px;
    overflow: hidden;
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 70%, transparent);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    margin-bottom: 0.3rem;
  }

  .tokbar i {
    display: block;
    height: 100%;
    transition: width 0.7s var(--ease-out, ease);
  }

  .tokbar-in { background: var(--grad-span); }
  .tokbar-out { background: var(--grad-hot); }

  .tokbar-legend {
    display: flex;
    gap: 0.9rem;
    font-family: var(--font-mono);
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-text-faint, #5b6b84);
    margin-bottom: 0.4rem;
  }

  .tokbar-legend .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    margin-right: 0.35rem;
    vertical-align: -1px;
  }

  .dot--in { background: var(--color-accent, #3b82f6); }
  .dot--out { background: var(--color-amber, #f59e0b); }

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
    font-family: var(--font-mono);
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--color-text-faint, #5b6b84);
    margin-bottom: 0.3rem;
  }

  .message {
    display: flex;
    gap: 0.4rem;
    font-size: 0.78rem;
    margin-bottom: 0.3rem;
    word-break: break-word;
  }

  .role {
    flex-shrink: 0;
    color: var(--color-link, #7dd3fc);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    font-weight: 500;
    letter-spacing: 0.04em;
    min-width: 4rem;
  }

  .content {
    color: var(--color-code-muted, #b9c5d8);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    line-height: 1.6;
    white-space: pre-wrap;
    max-height: 120px;
    overflow-y: auto;
  }

  .tool-call {
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 65%, transparent);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 8px;
    padding: 0.4rem 0.55rem;
    margin-bottom: 0.3rem;
  }

  .tool-name {
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 0.74rem;
    margin-bottom: 0.15rem;
    color: var(--color-text, #e9eff8);
  }

  .tool-args {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--color-code-muted, #b9c5d8);
    white-space: pre-wrap;
  }

  /* Safety — red, distinct from LLM amber */
  .safety-section {
    background: color-mix(in srgb, var(--color-danger, #f87171) 7%, transparent);
    border-left: 2px solid color-mix(in srgb, var(--color-danger, #f87171) 55%, transparent);
  }

  .safety-section .section-title { color: var(--color-danger, #f87171); }

  .safety-sq {
    width: 12px;
    height: 7px;
    border-radius: 2px;
    background: var(--color-danger, #f87171);
    flex: none;
  }

  .safety-row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem;
    padding: 0.3rem 0;
    border-bottom: 1px dashed var(--color-border-soft, rgba(125, 211, 252, 0.07));
  }

  .safety-row:last-child { border-bottom: none; }

  .safety-cat {
    font-family: var(--font-mono);
    font-size: 0.66rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-danger, #f87171);
  }

  .safety-sev {
    font-family: var(--font-mono);
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    padding: 0.05rem 0.4rem;
    border-radius: 999px;
    border: 1px solid currentColor;
    color: var(--color-text-muted, #94a3b8);
  }

  .safety-row--high .safety-sev { color: var(--color-danger, #f87171); }
  .safety-row--medium .safety-sev { color: var(--color-amber, #f59e0b); }
  .safety-row--low .safety-sev { color: var(--color-text-muted, #94a3b8); }

  .safety-detail {
    flex-basis: 100%;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-code-muted, #b9c5d8);
    word-break: break-word;
  }

  .annotation-section { background: var(--color-panel-highlight, rgba(255, 255, 255, 0.04)); }

  .annotation-input {
    width: 100%;
    resize: vertical;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 8px;
    padding: 0.5rem 0.65rem;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-text, #e9eff8);
    font: 400 0.8rem/1.5 var(--font-display);
    outline: none;
    transition: border-color 0.15s var(--ease-spring), box-shadow 0.15s var(--ease-spring);
  }

  .annotation-input:focus {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent, #3b82f6) 15%, transparent);
  }

  .annotation-input::placeholder { color: var(--color-text-muted, #94a3b8); }

  .annotation-hint {
    font-size: 0.68rem;
    color: var(--color-text-muted, #94a3b8);
    margin-top: 0.2rem;
  }

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
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    color: var(--color-link, #7dd3fc);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    padding: 0.26rem 0.5rem;
    border-radius: 7px;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-out);
  }

  .child-link:hover {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.16));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 40%, transparent);
    transform: translateX(2px);
  }
</style>
