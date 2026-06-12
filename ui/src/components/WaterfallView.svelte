<script lang="ts">
  import { onMount } from 'svelte';
  import type { WaterfallLayout, WaterfallRow } from '../lib/types';
  import { selectedSpanId, hoveredSpanId } from '../stores/selection';
  import { SERVICE_COLORS } from '../lib/palette';

  export let layout: WaterfallLayout;

  const ROW_HEIGHT = 28;
  const LABEL_WIDTH = 320;
  const INDENT_PX = 16;
  const MIN_BAR_PX = 2;

  // ── Color map ────────────────────────────────────────────────────
  let colorMap = new Map<string, string>();

  // ── Collapse state ────────────────────────────────────────────────
  let collapsed = new Set<string>();

  // ── Tooltip ───────────────────────────────────────────────────────
  let tooltip: { x: number; y: number; row: WaterfallRow } | null = null;

  let zoom = 1;
  let panX = 0;

  let rootEl: HTMLDivElement;

  // ── Derived view state (assigned in the reactive block below) ─────
  let visibleRows: WaterfallRow[] = [];
  let ticks: { px: number; label: string }[] = [];

  export function focusView(): void {
    if (!rootEl) return;
    rootEl.scrollIntoView({ block: 'nearest' });
    rootEl.focus();
  }

  // ── Reactive: build color map and visible rows ────────────────────
  $: {
    colorMap = new Map();
    let idx = 0;
    for (const r of layout.rows) {
      if (!colorMap.has(r.color_key)) {
        colorMap.set(r.color_key, SERVICE_COLORS[idx % SERVICE_COLORS.length]);
        idx++;
      }
    }
    visibleRows = computeVisible(layout.rows, collapsed);
    ticks = buildTicks(barContainerW, zoom, panX, layout.trace_duration_ns);
  }

  function computeVisible(rows: WaterfallRow[], col: Set<string>): WaterfallRow[] {
    const result: WaterfallRow[] = [];
    let hideDepth = Infinity; // hide rows with depth > hideDepth

    for (const row of rows) {
      // Exiting a collapsed subtree when depth <= the collapse point
      if (row.depth <= hideDepth) {
        hideDepth = Infinity;
      }
      // Skip if inside a collapsed subtree
      if (row.depth > hideDepth) continue;

      result.push(row);

      if (col.has(row.span_id) && row.has_children) {
        hideDepth = row.depth;
      }
    }
    return result;
  }

  function toggleCollapse(spanId: string) {
    collapsed = new Set(collapsed);
    if (collapsed.has(spanId)) {
      collapsed.delete(spanId);
    } else {
      collapsed.add(spanId);
    }
  }

  // ── Icon helpers ──────────────────────────────────────────────────
  function getKindIcon(row: WaterfallRow): string {
    if (row.is_llm) return '⚡';
    switch (row.span_kind) {
      case 'Client':   return '↔';
      case 'Server':   return '⇥';
      case 'Producer': return '→';
      case 'Consumer': return '←';
      default:         return '';
    }
  }

  // ── Format helpers ────────────────────────────────────────────────
  function fmtCost(v: number | null): string {
    if (v === null || v === undefined) return '';
    if (v === 0) return '$0';
    if (v < 0.0001) return `$${v.toExponential(2)}`;
    return `$${v.toFixed(6)}`;
  }

  function fmtTokens(row: WaterfallRow): string {
    const i = row.input_tokens;
    const o = row.output_tokens;
    const t = row.total_tokens;
    if (i !== null && o !== null) {
      const total = t !== null ? t : (i + o);
      return `${i.toLocaleString()} → ${o.toLocaleString()} (Σ ${total.toLocaleString()})`;
    }
    if (t !== null) return `Σ ${t!.toLocaleString()}`;
    return '';
  }

  // ── Bar helpers ───────────────────────────────────────────────────
  function barLeft(row: WaterfallRow, containerW: number): number {
    return Math.max(0, (row.x_start - panX) * zoom * containerW);
  }

  function barWidth(row: WaterfallRow, containerW: number): number {
    return Math.max(MIN_BAR_PX, (row.x_end - row.x_start) * zoom * containerW);
  }

  // ── Time axis ticks ───────────────────────────────────────────────
  let barContainerW = 0;

  function formatNs(ns: number): string {
    if (ns < 1_000) return `${Math.round(ns)}ns`;
    if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)}μs`;
    if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(1)}ms`;
    return `${(ns / 1_000_000_000).toFixed(2)}s`;
  }

  function buildTicks(
    containerW: number,
    z: number,
    pan: number,
    totalNs: number
  ): { px: number; label: string }[] {
    if (!containerW || !totalNs) return [];
    const count = Math.max(3, Math.floor(containerW / 100));
    const result = [];
    for (let i = 0; i <= count; i++) {
      const normX = i / count;
      const px = (normX - pan) * z * containerW;
      if (px < 0 || px > containerW) continue;
      const ns = (normX / z + pan) * totalNs;
      result.push({ px, label: formatNs(ns) });
    }
    return result;
  }

  // ── Zoom / wheel ──────────────────────────────────────────────────
  function onBarWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const factor = e.deltaY < 0 ? 1.25 : 1 / 1.25;
      const cell = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const mouseX = e.clientX - cell.left;
      const normAtMouse = mouseX / (barContainerW * zoom) + panX;
      zoom = Math.max(1, Math.min(zoom * factor, 200));
      panX = Math.max(0, Math.min(normAtMouse - mouseX / (barContainerW * zoom), 1 - 1 / zoom));
    }
  }

  function resetZoom() {
    zoom = 1;
    panX = 0;
  }

  // ── Hover / click on bar ──────────────────────────────────────────
  function onBarMouseMove(e: MouseEvent, row: WaterfallRow) {
    hoveredSpanId.set(row.span_id);
    tooltip = { x: e.clientX + 12, y: e.clientY + 12, row };
  }

  function onBarMouseLeave() {
    hoveredSpanId.set(null);
    tooltip = null;
  }

  function onRowClick(row: WaterfallRow) {
    selectedSpanId.set(row.span_id);
  }

  // ── ResizeObserver for bar container ─────────────────────────────
  let barContainerEl: HTMLDivElement;

  onMount(() => {
    if (barContainerEl) {
      const ro = new ResizeObserver((entries) => {
        barContainerW = entries[0].contentRect.width;
      });
      ro.observe(barContainerEl);
      barContainerW = barContainerEl.getBoundingClientRect().width;
      return () => ro.disconnect();
    }
  });
</script>

<div class="wf-root" bind:this={rootEl} tabindex="-1" role="region" aria-label="Waterfall graph">
  <!-- ── Header ─────────────────────────────────────────────────── -->
  <div class="wf-header">
    <div class="wf-header-label">Span</div>
    <div class="wf-header-bars" bind:this={barContainerEl}>
      {#each ticks as tick}
        <span class="wf-tick" style="left:{tick.px}px">{tick.label}</span>
      {/each}
      <div class="wf-zoom-controls">
        <button class="wf-ctrl-btn" title="Reset zoom" on:click={resetZoom}>Reset</button>
      </div>
    </div>
  </div>

  <!-- ── Body ───────────────────────────────────────────────────── -->
  <div class="wf-body">
    {#each visibleRows as row (row.span_id)}
      {@const sel = $selectedSpanId === row.span_id}
      {@const hov = $hoveredSpanId === row.span_id}
      {@const color = colorMap.get(row.color_key) ?? '#64748b'}
      {@const icon = getKindIcon(row)}
      {@const tokens = fmtTokens(row)}
      {@const cost = fmtCost(row.estimated_cost_usd)}
      <div
        class="wf-row"
        class:wf-row--selected={sel}
        class:wf-row--hovered={hov && !sel}
        class:wf-row--error={row.is_error}
        role="row"
        aria-selected={sel}
        on:click={() => onRowClick(row)}
        on:keydown={(e) => e.key === 'Enter' && onRowClick(row)}
        tabindex="0"
      >
        <!-- Label cell (fixed width, may grow taller for LLM meta) -->
        <div
          class="wf-label"
          style="padding-left: {row.depth * INDENT_PX + 8}px;"
        >
          <div class="wf-label-main">
            {#if row.has_children}
              <button
                class="wf-expand"
                title={collapsed.has(row.span_id) ? 'Expand' : 'Collapse'}
                on:click|stopPropagation={() => toggleCollapse(row.span_id)}
              >
                {collapsed.has(row.span_id) ? '▶' : '▼'}
              </button>
            {:else}
              <span class="wf-expand-spacer"></span>
            {/if}

            {#if icon}
              <span class="wf-kind-icon" title={row.span_kind}>{icon}</span>
            {/if}

            <span class="wf-op-name" title="{row.service_name}: {row.operation_name}">
              {row.operation_name}
            </span>

            <span class="wf-dur">{row.duration_display}</span>
          </div>

          {#if row.is_llm && (tokens || cost)}
            <div class="wf-llm-meta">
              {#if tokens}<span class="wf-tokens">{tokens}</span>{/if}
              {#if cost}<span class="wf-cost" class:wf-cost--large={row.estimated_cost_usd !== null && row.estimated_cost_usd > 0.01}>{cost}</span>{/if}
            </div>
          {/if}
        </div>

        <!-- Bar cell (stretches to full row height) -->
        <div
          class="wf-bar-cell"
          role="presentation"
          on:wheel={onBarWheel}
          on:mouseleave={onBarMouseLeave}
        >
          {#if barContainerW > 0}
            {@const bl = barLeft(row, barContainerW)}
            {@const bw = barWidth(row, barContainerW)}
            <div
              class="wf-bar"
              role="presentation"
              style="left:{bl}px; width:{bw}px; background:{color};"
              class:wf-bar--error={row.is_error}
              on:mousemove={(e) => onBarMouseMove(e, row)}
            >
              {#if bw > 48}
                <span class="wf-bar-label">{row.duration_display}</span>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<!-- Tooltip -->
{#if tooltip}
  <div
    class="wf-tooltip"
    style="left:{tooltip.x}px; top:{tooltip.y}px;"
  >
    <div class="wf-tt-title">{tooltip.row.service_name}: {tooltip.row.operation_name}</div>
    <div class="wf-tt-row"><span>Duration</span><span>{tooltip.row.duration_display}</span></div>
    <div class="wf-tt-row"><span>Self-time</span><span>{tooltip.row.self_time_display}</span></div>
    {#if fmtTokens(tooltip.row)}
      <div class="wf-tt-row"><span>Tokens</span><span>{fmtTokens(tooltip.row)}</span></div>
    {/if}
    {#if fmtCost(tooltip.row.estimated_cost_usd)}
      <div class="wf-tt-row"><span>Cost</span><span>{fmtCost(tooltip.row.estimated_cost_usd)}</span></div>
    {/if}
    {#if tooltip.row.is_error}
      <div class="wf-tt-error">⚠ Error span</div>
    {/if}
  </div>
{/if}

<style>
  .wf-root {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background: var(--color-canvas-bg, #0f172a);
    font-size: 12.5px;
    outline: none;
  }

  .wf-root:focus-visible {
    outline: 2px solid var(--color-accent, #3b82f6);
    outline-offset: -2px;
  }

  /* ── Header ──────────────────────────────────────────────────── */
  .wf-header {
    display: flex;
    flex-shrink: 0;
    height: 28px;
    border-bottom: 1px solid var(--color-border, #334155);
    background: var(--color-surface, #1e293b);
  }

  .wf-header-label {
    width: 320px;
    min-width: 320px;
    padding: 0 8px;
    display: flex;
    align-items: center;
    font-family: var(--font-mono);
    font-weight: 500;
    font-size: 10px;
    color: var(--color-sky, #7dd3fc);
    border-right: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    text-transform: uppercase;
    letter-spacing: 0.14em;
  }

  .wf-header-bars {
    flex: 1;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
  }

  .wf-tick {
    position: absolute;
    font-size: 10px;
    font-family: monospace;
    color: var(--color-text-muted, #64748b);
    user-select: none;
    white-space: nowrap;
    top: 50%;
    transform: translateY(-50%);
    padding-left: 2px;
  }

  .wf-zoom-controls {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    gap: 4px;
  }

  .wf-ctrl-btn {
    padding: 1px 10px;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    color: var(--color-text-muted, #9aa8bd);
    border-radius: 999px;
    font-family: var(--font-mono);
    font-size: 9.5px;
    letter-spacing: 0.08em;
    cursor: pointer;
    line-height: 1.7;
    transition: border-color 0.15s var(--ease-spring), color 0.15s var(--ease-spring);
  }

  .wf-ctrl-btn:hover {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 40%, transparent);
    color: var(--color-text, #e9eff8);
  }

  /* ── Body ─────────────────────────────────────────────────────── */
  .wf-body {
    flex: 1;
    overflow: auto;
  }

  /* ── Row ──────────────────────────────────────────────────────── */
  .wf-row {
    display: grid;
    grid-template-columns: 320px 1fr;
    min-height: 28px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    cursor: pointer;
    outline: none;
  }

  .wf-row:hover,
  .wf-row--hovered {
    background: rgba(255, 255, 255, 0.04);
  }

  .wf-row--selected {
    background: rgba(59, 130, 246, 0.15);
  }

  .wf-row--selected:hover {
    background: rgba(59, 130, 246, 0.2);
  }

  .wf-row--error .wf-label {
    color: #fca5a5;
  }

  .wf-row:focus-visible {
    outline: 1px solid var(--color-accent, #3b82f6);
    outline-offset: -1px;
  }

  /* ── Label cell ───────────────────────────────────────────────── */
  .wf-label {
    display: flex;
    flex-direction: column;
    justify-content: center;
    border-right: 1px solid var(--color-border, #334155);
    color: var(--color-text, #e2e8f0);
    overflow: hidden;
    min-height: 28px;
  }

  .wf-label-main {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    overflow: hidden;
  }

  .wf-expand {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    background: none;
    border: none;
    color: var(--color-text-muted, #94a3b8);
    font-size: 9px;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
    line-height: 1;
  }

  .wf-expand:hover {
    color: var(--color-text, #e2e8f0);
    background: rgba(255, 255, 255, 0.1);
  }

  .wf-expand-spacer {
    flex-shrink: 0;
    width: 16px;
    display: inline-block;
  }

  .wf-kind-icon {
    flex-shrink: 0;
    font-size: 11px;
    width: 14px;
    text-align: center;
    color: var(--color-text-muted, #94a3b8);
  }

  .wf-op-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
  }

  .wf-dur {
    flex-shrink: 0;
    font-size: 11px;
    font-family: monospace;
    color: var(--color-text-muted, #94a3b8);
    margin-right: 6px;
  }

  /* ── LLM meta (inside label cell, below main line) ────────────── */
  .wf-llm-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 18px;
    padding-left: 20px;
    padding-bottom: 2px;
  }

  .wf-tokens {
    font-size: 10.5px;
    font-family: monospace;
    color: var(--color-text-muted, #94a3b8);
  }

  .wf-cost {
    font-size: 10.5px;
    font-family: monospace;
    color: #86efac;
  }

  .wf-cost--large {
    color: #fca5a5;
    font-weight: 600;
  }

  /* ── Bar cell ─────────────────────────────────────────────────── */
  .wf-bar-cell {
    position: relative;
    overflow: hidden;
    min-width: 0;
  }

  .wf-bar {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    height: 18px;
    border-radius: 5px;
    min-width: 2px;
    display: flex;
    align-items: center;
    overflow: hidden;
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.2) inset;
    transition: filter 0.1s;
  }

  .wf-bar:hover {
    filter: brightness(1.18) saturate(1.08);
  }

  .wf-bar--error {
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.2) inset, inset 0 0 0 1.5px rgba(239, 68, 68, 0.85);
  }

  .wf-bar-label {
    padding: 0 5px;
    font-family: var(--font-mono);
    font-size: 9.5px;
    color: rgba(255, 255, 255, 0.92);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  /* ── Tooltip ──────────────────────────────────────────────────── */
  .wf-tooltip {
    position: fixed;
    z-index: 1000;
    background: color-mix(in srgb, var(--color-surface, #0d1626) 96%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid color-mix(in srgb, var(--color-sky, #7dd3fc) 28%, transparent);
    border-radius: 10px;
    padding: 9px 12px;
    font-size: 12px;
    color: var(--color-text, #e9eff8);
    pointer-events: none;
    min-width: 200px;
    box-shadow: var(--shadow-panel, 0 14px 40px -10px rgba(2, 6, 18, 0.9));
  }

  .wf-tt-title {
    font-family: var(--font-mono);
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--color-ice, #bae6fd);
    word-break: break-all;
    font-size: 11.5px;
  }

  .wf-tt-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 1.5px 0;
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .wf-tt-row span:first-child {
    color: var(--color-text-faint, #5b6b84);
  }

  .wf-tt-row span:last-child {
    color: var(--color-gold, #fcd34d);
  }

  .wf-tt-error {
    margin-top: 4px;
    color: var(--color-danger, #f87171);
    font-size: 10.5px;
    font-family: var(--font-mono);
  }
</style>
