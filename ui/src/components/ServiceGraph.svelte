<script lang="ts">
  import { onMount } from 'svelte';
  import type { ServiceGraph } from '../lib/types';
  import { selectedSpanId } from '../stores/selection';
  import { SERVICE_COLORS } from '../lib/palette';

  export let graph: ServiceGraph;

  const PADDING = 48;
  const NODE_RADIUS_BASE = 18;
  const NODE_RADIUS_MAX = 48;

  let svg: SVGElement;
  let width = 600;
  let height = 400;

  interface LayoutNode {
    service: string;
    x: number;
    y: number;
    r: number;
    span_count: number;
    error_count: number;
    llm_count: number;
    total_duration_display: string;
  }

  interface LayoutEdge {
    source: string;
    target: string;
    call_count: number;
    total_duration_display: string;
    x1: number;
    y1: number;
    x2: number;
    y2: number;
  }

  const COLORS = SERVICE_COLORS;

  $: nodes = layoutNodes(graph?.nodes ?? []);
  $: edges = layoutEdges(graph?.edges ?? [], nodes);
  $: maxSpans = Math.max(1, ...(graph?.nodes ?? []).map(n => n.span_count));

  function layoutNodes(gNodes: ServiceGraph['nodes']): LayoutNode[] {
    if (gNodes.length === 0) return [];
    const cx = width / 2;
    const cy = height / 2;
    const rx = (width - PADDING * 2) / 2;
    const ry = (height - PADDING * 2) / 2;

    return gNodes.map((n, i) => {
      const angle = (2 * Math.PI * i) / gNodes.length - Math.PI / 2;
      const radius = NODE_RADIUS_BASE + (n.span_count / maxSpans) * (NODE_RADIUS_MAX - NODE_RADIUS_BASE);
      return {
        service: n.service,
        x: cx + rx * Math.cos(angle),
        y: cy + ry * Math.sin(angle),
        r: radius,
        span_count: n.span_count,
        error_count: n.error_count,
        llm_count: n.llm_count,
        total_duration_display: n.total_duration_display,
      };
    });
  }

  function layoutEdges(gEdges: ServiceGraph['edges'], lNodes: LayoutNode[]): LayoutEdge[] {
    const nodeMap = new Map(lNodes.map(n => [n.service, n]));
    return gEdges
      .map(e => {
        const src = nodeMap.get(e.source);
        const tgt = nodeMap.get(e.target);
        if (!src || !tgt) return null;
        return { ...e, x1: src.x, y1: src.y, x2: tgt.x, y2: tgt.y };
      })
      .filter(Boolean) as LayoutEdge[];
  }

  $: maxEdges = Math.max(1, ...(graph?.edges ?? []).map(e => e.call_count));

  onMount(() => {
    const ro = new ResizeObserver(entries => {
      width = entries[0].contentRect.width;
      height = entries[0].contentRect.height;
    });
    const parent = svg.parentElement;
    if (parent) ro.observe(parent);
    return () => ro.disconnect();
  });

  function edgeWidth(calls: number, maxCalls: number): number {
    return 0.5 + (calls / Math.max(1, maxCalls)) * 4;
  }

  function edgeOpacity(calls: number, maxCalls: number): number {
    return 0.4 + (calls / Math.max(1, maxCalls)) * 0.45;
  }
</script>

{#if graph.nodes.length === 0}
  <div class="empty">No service relationships to display</div>
{:else}
  <svg bind:this={svg} class="graph-svg" width={width} height={height} role="group" aria-label="Service dependency graph">
    {#each edges as edge (edge.source + edge.target)}
      <line
        x1={edge.x1} y1={edge.y1} x2={edge.x2} y2={edge.y2}
        class="edge"
        style="stroke-width: {edgeWidth(edge.call_count, maxEdges)}px; opacity: {edgeOpacity(edge.call_count, maxEdges)};"
      >
        <title>{edge.source} → {edge.target}
{edge.call_count} call{edge.call_count !== 1 ? 's' : ''} · {edge.total_duration_display}</title>
      </line>
    {/each}

    {#each edges as edge (edge.source + edge.target)}
      <text
        class="edge-label"
        x={(edge.x1 + edge.x2) / 2}
        y={(edge.y1 + edge.y2) / 2 - 4}
        text-anchor="middle"
        font-size="9"
      >{edge.call_count}</text>
    {/each}

    {#each nodes as node, i (node.service)}
      <g
        class="node-group"
        role="button"
        tabindex="0"
        aria-label="{node.service} — {node.span_count} spans, {node.error_count} errors"
        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') e.preventDefault(); }}
      >
        <circle
          cx={node.x} cy={node.y} r={node.r}
          class="node"
          style="fill: {COLORS[i % COLORS.length]};"
        />
        {#if node.error_count > 0}
          <circle cx={node.x} cy={node.y} r={node.r} class="node-error" />
        {/if}
        <text class="node-label" x={node.x} y={node.y + 4} text-anchor="middle">{node.service}</text>
        <title>{node.service}
{node.span_count} spans · {node.total_duration_display}
{node.error_count} errors · {node.llm_count} LLM</title>
      </g>
    {/each}
  </svg>
{/if}

<style>
  .graph-svg {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--color-canvas-bg, #0f172a);
  }

  .edge {
    /* Themed, not --color-border: that token is a hairline tuned for panel
       edges and disappears against the light canvas. */
    stroke: var(--color-text-faint, #5b6b84);
  }

  .edge-label {
    fill: var(--color-text-muted, #64748b);
    pointer-events: none;
    font-family: var(--font-mono, monospace);
  }

  .node {
    stroke: var(--color-canvas-bg, #0f172a);
    stroke-width: 2;
    cursor: pointer;
  }

  .node:hover {
    filter: brightness(1.2);
  }

  .node-error {
    fill: none;
    stroke: var(--color-danger, #ef4444);
    stroke-width: 2;
    stroke-dasharray: 3 2;
    pointer-events: none;
  }

  .node-label {
    /* Halo keeps the label legible on every service colour (amber, lime,
       yellow) in both themes without per-node contrast maths. */
    fill: #fff;
    stroke: rgba(3, 7, 18, 0.66);
    stroke-width: 2.5px;
    paint-order: stroke fill;
    font-size: 9.5px;
    font-weight: 500;
    font-family: var(--font-mono);
    letter-spacing: 0.02em;
    pointer-events: none;
  }

  .node-group:focus-visible {
    outline: 2px solid var(--color-accent, #3b82f6);
    outline-offset: 4px;
    border-radius: 50%;
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
