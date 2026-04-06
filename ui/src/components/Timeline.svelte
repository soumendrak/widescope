<script lang="ts">
  import { onMount } from 'svelte';
  import type { TimelineBlock, TimelineLayout, TimelineRow } from '../lib/types';
  import { focusedSpanId, hoveredSpanId, searchResults, selectedSpanId } from '../stores/selection';

  export let layout: TimelineLayout;

  const AXIS_HEIGHT = 32;
  const GROUP_HEADER_HEIGHT = 20;
  const GROUP_GAP = 8;
  const LANE_HEIGHT = 28;
  const LABEL_WIDTH = 160;
  const RIGHT_PADDING = 16;
  const MIN_BLOCK_WIDTH = 2;

  const SERVICE_COLORS = [
    '#3b82f6', '#10b981', '#f59e0b', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316',
    '#6366f1', '#14b8a6', '#eab308', '#ef4444',
  ];

  type PositionedRow = {
    row: TimelineRow;
    y: number;
    blocks: TimelineBlock[];
  };

  type TimelineGroup = {
    serviceName: string;
    top: number;
    height: number;
    rows: PositionedRow[];
  };

  let rootEl: HTMLDivElement;
  let viewportEl: HTMLDivElement;
  let viewportWidth = 0;
  let colorMap = new Map<string, string>();
  let groups: TimelineGroup[] = [];
  let chartWidth = 0;
  let svgHeight = AXIS_HEIGHT;
  let ticks: { x: number; label: string }[] = [];
  let hasSearch = false;
  let searchResultSet = new Set<string>();

  export function focusView(): void {
    if (!rootEl) return;
    rootEl.scrollIntoView({ block: 'nearest' });
    rootEl.focus();
  }

  $: colorMap = buildColorMap(layout?.rows ?? []);
  $: groups = buildGroups(layout?.rows ?? [], layout?.blocks ?? []);
  $: chartWidth = Math.max(0, viewportWidth - LABEL_WIDTH - RIGHT_PADDING);
  $: svgHeight = groups.length > 0 ? groups[groups.length - 1].top + groups[groups.length - 1].height : AXIS_HEIGHT;
  $: ticks = buildTicks(chartWidth, layout?.trace_duration_ns ?? 0);
  $: hasSearch = $searchResults.length > 0;
  $: searchResultSet = new Set($searchResults);

  onMount(() => {
    if (!viewportEl) return;

    const observer = new ResizeObserver((entries) => {
      viewportWidth = entries[0]?.contentRect.width ?? 0;
    });

    observer.observe(viewportEl);
    viewportWidth = viewportEl.getBoundingClientRect().width;

    return () => observer.disconnect();
  });

  function buildColorMap(rows: TimelineRow[]): Map<string, string> {
    const next = new Map<string, string>();
    let index = 0;

    for (const row of rows) {
      if (!next.has(row.service_name)) {
        next.set(row.service_name, SERVICE_COLORS[index % SERVICE_COLORS.length]);
        index += 1;
      }
    }

    return next;
  }

  function buildGroups(rows: TimelineRow[], blocks: TimelineBlock[]): TimelineGroup[] {
    const sortedRows = [...rows].sort((a, b) => a.row_index - b.row_index);
    const blocksByRow = new Map<number, TimelineBlock[]>();

    for (const block of blocks) {
      const rowBlocks = blocksByRow.get(block.row_index) ?? [];
      rowBlocks.push(block);
      blocksByRow.set(block.row_index, rowBlocks);
    }

    for (const rowBlocks of blocksByRow.values()) {
      rowBlocks.sort((a, b) => a.x_start - b.x_start);
    }

    const builtGroups: TimelineGroup[] = [];
    let cursorY = AXIS_HEIGHT;
    let index = 0;

    while (index < sortedRows.length) {
      const serviceName = sortedRows[index].service_name;
      const serviceRows: TimelineRow[] = [];

      while (index < sortedRows.length && sortedRows[index].service_name === serviceName) {
        serviceRows.push(sortedRows[index]);
        index += 1;
      }

      const positionedRows = serviceRows.map((row) => ({
        row,
        y: cursorY + GROUP_HEADER_HEIGHT + row.lane_index * LANE_HEIGHT,
        blocks: blocksByRow.get(row.row_index) ?? [],
      }));

      const height = GROUP_HEADER_HEIGHT + Math.max(1, serviceRows.length) * LANE_HEIGHT;
      builtGroups.push({
        serviceName,
        top: cursorY,
        height,
        rows: positionedRows,
      });

      cursorY += height + GROUP_GAP;
    }

    return builtGroups;
  }

  function buildTicks(width: number, totalNs: number): { x: number; label: string }[] {
    if (width <= 0 || totalNs <= 0) return [];

    const tickCount = Math.max(4, Math.floor(width / 120));
    const builtTicks: { x: number; label: string }[] = [];

    for (let index = 0; index <= tickCount; index += 1) {
      const ratio = index / tickCount;
      builtTicks.push({
        x: LABEL_WIDTH + ratio * width,
        label: formatNs(Math.round(ratio * totalNs)),
      });
    }

    return builtTicks;
  }

  function formatNs(ns: number): string {
    if (ns < 1_000) return `${ns}ns`;
    if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)}μs`;
    if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(1)}ms`;
    return `${(ns / 1_000_000_000).toFixed(2)}s`;
  }

  function blockX(block: TimelineBlock): number {
    return LABEL_WIDTH + block.x_start * chartWidth;
  }

  function blockWidth(block: TimelineBlock): number {
    return Math.max(MIN_BLOCK_WIDTH, (block.x_end - block.x_start) * chartWidth);
  }

  function blockLabel(block: TimelineBlock): string {
    return `${block.label} · ${block.duration_display}${block.is_error ? ' · error' : ''}`;
  }

  function selectBlock(block: TimelineBlock): void {
    selectedSpanId.set(block.span_id);
    focusedSpanId.set(block.span_id);
  }

  function focusBlock(block: TimelineBlock): void {
    focusedSpanId.set(block.span_id);
  }

  function setHovered(block: TimelineBlock | null): void {
    hoveredSpanId.set(block?.span_id ?? null);
  }

  function onBlockKeyDown(event: KeyboardEvent, block: TimelineBlock): void {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    selectBlock(block);
  }
</script>

<div
  class="timeline-root"
  bind:this={rootEl}
  tabindex="0"
  role="region"
  aria-label="Timeline swimlane"
  on:mouseleave={() => setHovered(null)}
>
  <div class="timeline-scroll" bind:this={viewportEl}>
    <svg
      class="timeline-svg"
      width={Math.max(viewportWidth, LABEL_WIDTH + RIGHT_PADDING)}
      height={svgHeight}
      role="img"
      aria-label="Timeline swimlane view"
    >
      <g class="time-axis">
        <rect x="0" y="0" width={Math.max(viewportWidth, LABEL_WIDTH + RIGHT_PADDING)} height={AXIS_HEIGHT} class="axis-bg" />
        <text x="12" y="20" class="axis-label">Service</text>
        {#each ticks as tick}
          <line x1={tick.x} y1={AXIS_HEIGHT - 14} x2={tick.x} y2={svgHeight} class="axis-grid" />
          <line x1={tick.x} y1={AXIS_HEIGHT - 10} x2={tick.x} y2={AXIS_HEIGHT - 1} class="axis-tick" />
          <text x={tick.x + 4} y="20" class="axis-tick-label">{tick.label}</text>
        {/each}
        <line x1="0" y1={AXIS_HEIGHT - 1} x2={Math.max(viewportWidth, LABEL_WIDTH + RIGHT_PADDING)} y2={AXIS_HEIGHT - 1} class="axis-border" />
      </g>

      {#each groups as group}
        <g class="service-group">
          <rect x="0" y={group.top} width={Math.max(viewportWidth, LABEL_WIDTH + RIGHT_PADDING)} height={group.height} class="group-bg" />
          <rect x="0" y={group.top} width={LABEL_WIDTH} height={GROUP_HEADER_HEIGHT} class="group-header-bg" />
          <text x="12" y={group.top + 14} class="group-label">{group.serviceName}</text>
          <line x1="0" y1={group.top + GROUP_HEADER_HEIGHT} x2={Math.max(viewportWidth, LABEL_WIDTH + RIGHT_PADDING)} y2={group.top + GROUP_HEADER_HEIGHT} class="group-divider" />

          {#each group.rows as rowMeta}
            <rect x="0" y={rowMeta.y} width={LABEL_WIDTH} height={LANE_HEIGHT} class:lane-label-bg={rowMeta.row.lane_index % 2 === 0} class="lane-label-cell" />
            <rect x={LABEL_WIDTH} y={rowMeta.y} width={Math.max(0, chartWidth)} height={LANE_HEIGHT} class:lane-fill-alt={rowMeta.row.lane_index % 2 === 0} class="lane-fill" />
            <text x={LABEL_WIDTH - 12} y={rowMeta.y + 18} class="lane-label">Lane {rowMeta.row.lane_index + 1}</text>
            <line x1="0" y1={rowMeta.y + LANE_HEIGHT} x2={Math.max(viewportWidth, LABEL_WIDTH + RIGHT_PADDING)} y2={rowMeta.y + LANE_HEIGHT} class="lane-divider" />

            {#each rowMeta.blocks as block (block.span_id)}
              {@const isSelected = $selectedSpanId === block.span_id}
              {@const isHovered = $hoveredSpanId === block.span_id}
              {@const isFocused = $focusedSpanId === block.span_id}
              {@const isSearchMatch = searchResultSet.has(block.span_id)}
              {@const fill = colorMap.get(block.service_name) ?? '#64748b'}
              {@const x = blockX(block)}
              {@const width = blockWidth(block)}
              <g
                class="span-block-group"
                class:span-block-group--dim={hasSearch && !isSearchMatch}
                role="button"
                tabindex="0"
                aria-label={blockLabel(block)}
                on:click={() => selectBlock(block)}
                on:focus={() => focusBlock(block)}
                on:mouseenter={() => setHovered(block)}
                on:keydown={(event) => onBlockKeyDown(event, block)}
              >
                <title>{blockLabel(block)}</title>
                <rect
                  x={x}
                  y={rowMeta.y + 5}
                  width={width}
                  height="18"
                  rx="3"
                  class="span-block"
                  class:span-block--search-match={hasSearch && isSearchMatch}
                  class:span-block--selected={isSelected}
                  class:span-block--hovered={isHovered}
                  class:span-block--focused={isFocused && !isSelected}
                  class:span-block--error={block.is_error}
                  style={`fill: ${fill};`}
                />
                {#if block.is_llm}
                  <text x={x + 6} y={rowMeta.y + 18} class="span-icon">⚡</text>
                {/if}
                {#if width >= 88}
                  <text x={x + (block.is_llm ? 18 : 8)} y={rowMeta.y + 18} class="span-label">{block.label}</text>
                {/if}
              </g>
            {/each}
          {/each}
        </g>
      {/each}
    </svg>
  </div>
</div>

<style>
  .timeline-root {
    flex: 1;
    min-height: 0;
    background: var(--color-canvas-bg, #0f172a);
    outline: none;
    display: flex;
    flex-direction: column;
  }

  .timeline-root:focus-visible {
    outline: 2px solid var(--color-accent, #3b82f6);
    outline-offset: -2px;
  }

  .timeline-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .timeline-svg {
    display: block;
    min-width: 100%;
    background: var(--color-canvas-bg, #0f172a);
  }

  .axis-bg,
  .group-bg,
  .group-header-bg,
  .lane-label-cell,
  .lane-fill {
    fill: transparent;
  }

  .axis-bg {
    fill: var(--color-surface, #1e293b);
  }

  .axis-border,
  .group-divider,
  .lane-divider {
    stroke: var(--color-border, #334155);
    stroke-width: 1;
  }

  .axis-grid {
    stroke: color-mix(in srgb, var(--color-text-muted, #94a3b8) 20%, transparent);
    stroke-width: 1;
  }

  .axis-tick {
    stroke: var(--color-border, #334155);
    stroke-width: 1;
  }

  .axis-label,
  .group-label,
  .lane-label,
  .axis-tick-label {
    user-select: none;
  }

  .axis-label,
  .group-label {
    fill: var(--color-text, #e2e8f0);
    font-size: 12px;
    font-weight: 700;
  }

  .axis-tick-label,
  .lane-label {
    fill: var(--color-text-muted, #94a3b8);
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
  }

  .lane-label {
    text-anchor: end;
  }

  .group-header-bg {
    fill: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .lane-label-bg {
    fill: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .lane-fill-alt {
    fill: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
  }

  .span-block-group {
    cursor: pointer;
  }

  .span-block-group--dim {
    opacity: 0.24;
  }

  .span-block {
    stroke: transparent;
    stroke-width: 1;
    transition: filter 0.12s ease, stroke 0.12s ease, stroke-width 0.12s ease;
  }

  .span-block-group:hover .span-block,
  .span-block--hovered {
    filter: brightness(1.08);
  }

  .span-block--search-match {
    stroke: #fbbf24;
    stroke-width: 2;
  }

  .span-block--selected {
    stroke: var(--color-code-text, #ffffff);
    stroke-width: 2;
  }

  .span-block--focused {
    stroke: color-mix(in srgb, var(--color-text, #e2e8f0) 72%, transparent);
    stroke-dasharray: 4 3;
    stroke-width: 2;
  }

  .span-block--error {
    stroke: var(--color-danger, rgba(239, 68, 68, 0.95));
    stroke-width: 2;
  }

  .span-label,
  .span-icon {
    fill: var(--color-code-text, rgba(255, 255, 255, 0.95));
    pointer-events: none;
    user-select: none;
  }

  .span-label {
    font-size: 11px;
    font-weight: 600;
  }

  .span-icon {
    font-size: 10px;
  }
</style>
