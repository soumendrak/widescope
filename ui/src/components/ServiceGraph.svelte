<script lang="ts">
  import { onMount } from 'svelte';
  import type { ServiceGraph } from '../lib/types';
  import { SERVICE_COLORS } from '../lib/palette';

  export let graph: ServiceGraph;

  const PADDING_X = 92;
  const PADDING_Y = 76;
  const NODE_RADIUS_BASE = 16;
  const NODE_RADIUS_MAX = 44;
  const CURVE = 0.16; // how far edges bow off the straight line

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
    path: string;
    lx: number;
    ly: number;
    flowDur: number;
  }

  const COLORS = SERVICE_COLORS;

  $: maxSpans = Math.max(1, ...(graph?.nodes ?? []).map((n) => n.span_count));
  $: maxEdges = Math.max(1, ...(graph?.edges ?? []).map((e) => e.call_count));
  $: hasErrors = (graph?.nodes ?? []).some((n) => n.error_count > 0);
  // width/height/maxSpans are passed explicitly so Svelte re-lays-out on resize
  $: nodes = layoutNodes(graph?.nodes ?? [], width, height, maxSpans);
  $: edges = layoutEdges(graph?.edges ?? [], nodes);

  function layoutNodes(
    gNodes: ServiceGraph['nodes'],
    w: number,
    h: number,
    maxSpanCount: number
  ): LayoutNode[] {
    if (gNodes.length === 0) return [];
    const cx = w / 2;
    const cy = h / 2;
    const rx = Math.max(20, (w - PADDING_X * 2) / 2);
    const ry = Math.max(20, (h - PADDING_Y * 2) / 2);
    const single = gNodes.length === 1;

    return gNodes.map((n, i) => {
      const angle = (2 * Math.PI * i) / gNodes.length - Math.PI / 2;
      const ratio = n.span_count / maxSpanCount;
      const r = NODE_RADIUS_BASE + Math.sqrt(ratio) * (NODE_RADIUS_MAX - NODE_RADIUS_BASE);
      return {
        service: n.service,
        x: single ? cx : cx + rx * Math.cos(angle),
        y: single ? cy : cy + ry * Math.sin(angle),
        r,
        span_count: n.span_count,
        error_count: n.error_count,
        llm_count: n.llm_count,
        total_duration_display: n.total_duration_display,
      };
    });
  }

  function layoutEdges(gEdges: ServiceGraph['edges'], lNodes: LayoutNode[]): LayoutEdge[] {
    const nodeMap = new Map(lNodes.map((n) => [n.service, n]));
    return gEdges
      .map((e, i) => {
        const src = nodeMap.get(e.source);
        const tgt = nodeMap.get(e.target);
        if (!src || !tgt || src === tgt) return null; // ponytail: skip self-loops, rare in service graphs

        const dx = tgt.x - src.x;
        const dy = tgt.y - src.y;
        const len = Math.hypot(dx, dy) || 1;
        // control point: midpoint pushed along the perpendicular so reciprocal
        // edges (A→B and B→A) bow to opposite sides and stay readable
        const px = -dy / len;
        const py = dx / len;
        const off = len * CURVE;
        const cpx = (src.x + tgt.x) / 2 + px * off;
        const cpy = (src.y + tgt.y) / 2 + py * off;

        const sl = Math.hypot(cpx - src.x, cpy - src.y) || 1;
        const x1 = src.x + ((cpx - src.x) / sl) * (src.r + 2);
        const y1 = src.y + ((cpy - src.y) / sl) * (src.r + 2);
        const el = Math.hypot(cpx - tgt.x, cpy - tgt.y) || 1;
        const x2 = tgt.x + ((cpx - tgt.x) / el) * (tgt.r + 10); // room for arrowhead
        const y2 = tgt.y + ((cpy - tgt.y) / el) * (tgt.r + 10);

        return {
          ...e,
          path: `M${x1.toFixed(1)},${y1.toFixed(1)} Q${cpx.toFixed(1)},${cpy.toFixed(1)} ${x2.toFixed(1)},${y2.toFixed(1)}`,
          lx: 0.25 * x1 + 0.5 * cpx + 0.25 * x2,
          ly: 0.25 * y1 + 0.5 * cpy + 0.25 * y2,
          flowDur: 2 + (i % 5) * 0.35,
        };
      })
      .filter(Boolean) as LayoutEdge[];
  }

  onMount(() => {
    const ro = new ResizeObserver((entries) => {
      width = entries[0].contentRect.width;
      height = entries[0].contentRect.height;
    });
    const parent = svg.parentElement;
    if (parent) ro.observe(parent);
    return () => ro.disconnect();
  });

  function edgeWidth(calls: number): number {
    return 1 + (calls / maxEdges) * 3.5;
  }

  function edgeOpacity(calls: number): number {
    return 0.3 + (calls / maxEdges) * 0.5;
  }
</script>

{#if !graph || graph.nodes.length === 0}
  <div class="graph-wrap empty">No service relationships to display</div>
{:else}
  <div class="graph-wrap">
    <div class="legend" aria-hidden="true">
      <span class="legend-title">Service map</span>
      <span class="legend-sub"
        >{nodes.length} service{nodes.length !== 1 ? 's' : ''} · {edges.length} call path{edges.length !== 1
          ? 's'
          : ''}</span
      >
      <span class="legend-hint">orb size ∝ span volume{hasErrors ? ' · ring = errors' : ''}</span>
    </div>

    <svg
      bind:this={svg}
      class="graph-svg"
      {width}
      {height}
      viewBox="0 0 {width} {height}"
      role="img"
      aria-label="Service dependency graph"
    >
      <defs>
        <radialGradient id="sg-sheen" cx="0.32" cy="0.26" r="0.75">
          <stop offset="0%" stop-color="#fff" stop-opacity="0.55" />
          <stop offset="45%" stop-color="#fff" stop-opacity="0.08" />
          <stop offset="100%" stop-color="#fff" stop-opacity="0" />
        </radialGradient>
        <filter id="sg-glow" x="-40%" y="-40%" width="180%" height="180%">
          <feGaussianBlur stdDeviation="3.5" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <marker
          id="sg-arrow"
          viewBox="0 0 10 10"
          refX="8.5"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          markerUnits="userSpaceOnUse"
          orient="auto"
        >
          <path class="arrowhead" d="M0,0 L10,5 L0,10 z" />
        </marker>
      </defs>

      <g class="edges" filter="url(#sg-glow)">
        {#each edges as edge (edge.source + '>' + edge.target)}
          <path
            class="edge"
            d={edge.path}
            marker-end="url(#sg-arrow)"
            style="stroke-width:{edgeWidth(edge.call_count)}px;opacity:{edgeOpacity(edge.call_count)};"
          >
            <title
              >{edge.source} → {edge.target}
{edge.call_count} call{edge.call_count !== 1 ? 's' : ''} · {edge.total_duration_display}</title
            >
          </path>
        {/each}
      </g>

      <g class="flows">
        {#each edges as edge (edge.source + '>' + edge.target)}
          <path class="edge-flow" d={edge.path} style="animation-duration:{edge.flowDur}s" />
        {/each}
      </g>

      <g class="orbs" filter="url(#sg-glow)">
        {#each nodes as node, i (node.service)}
          <g
            class="node-group"
            role="button"
            tabindex="0"
            aria-label="{node.service} — {node.span_count} spans"
          >
            <circle class="orb" cx={node.x} cy={node.y} r={node.r} style="fill:{COLORS[i % COLORS.length]};" />
            <circle class="orb-sheen" cx={node.x} cy={node.y} r={node.r} />
            {#if node.error_count > 0}
              <circle class="orb-error" cx={node.x} cy={node.y} r={node.r + 3.5} />
            {/if}
            <title
              >{node.service}
{node.span_count} spans · {node.total_duration_display}
{node.error_count} errors · {node.llm_count} LLM</title
            >
          </g>
        {/each}
      </g>

      <g class="labels">
        {#each edges as edge (edge.source + '>' + edge.target)}
          <text class="edge-label" x={edge.lx} y={edge.ly}>{edge.call_count}</text>
        {/each}
        {#each nodes as node (node.service)}
          {#if node.r >= 20}
            <text class="node-count" x={node.x} y={node.y}>{node.span_count}</text>
          {/if}
          {#if node.llm_count > 0}
            <text class="node-llm" x={node.x + node.r * 0.64} y={node.y - node.r * 0.64}>✦</text>
          {/if}
          <text class="node-name" x={node.x} y={node.y + node.r + 16}>{node.service}</text>
        {/each}
      </g>
    </svg>
  </div>
{/if}

<style>
  .graph-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--color-canvas-bg, #070c16);
  }

  /* gridded + glowing atmosphere, echoing the landing page hero */
  .graph-wrap::before {
    content: '';
    position: absolute;
    inset: 0;
    background-image:
      linear-gradient(color-mix(in srgb, var(--color-accent, #3b82f6) 8%, transparent) 1px, transparent 1px),
      linear-gradient(90deg, color-mix(in srgb, var(--color-accent, #3b82f6) 8%, transparent) 1px, transparent 1px);
    background-size: 38px 38px;
    -webkit-mask-image: radial-gradient(ellipse 78% 70% at 50% 44%, #000 0%, transparent 82%);
    mask-image: radial-gradient(ellipse 78% 70% at 50% 44%, #000 0%, transparent 82%);
    pointer-events: none;
  }
  .graph-wrap::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      closest-side,
      color-mix(in srgb, var(--color-accent, #3b82f6) 16%, transparent),
      transparent 70%
    );
    transform: translate(-22%, -10%) scale(1.1);
    pointer-events: none;
  }

  .graph-svg {
    position: relative;
    display: block;
    width: 100%;
    height: 100%;
  }

  .legend {
    position: absolute;
    top: 14px;
    left: 14px;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 9px 13px;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-surface, #0d1626) 72%, transparent);
    backdrop-filter: blur(8px);
    pointer-events: none;
  }
  .legend-title {
    font-family: var(--font-display, sans-serif);
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--color-text, #e9eff8);
  }
  .legend-sub {
    font-family: var(--font-mono, monospace);
    font-size: 0.68rem;
    color: var(--color-text-muted, #9aa8bd);
  }
  .legend-hint {
    font-family: var(--font-mono, monospace);
    font-size: 0.62rem;
    letter-spacing: 0.02em;
    color: var(--color-text-faint, #5b6b84);
  }

  .edge {
    fill: none;
    stroke: var(--color-accent, #3b82f6);
    stroke-linecap: round;
  }
  .arrowhead {
    fill: var(--color-accent, #3b82f6);
  }
  .edge-flow {
    fill: none;
    stroke: color-mix(in srgb, var(--color-accent, #3b82f6) 70%, #fff);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-dasharray: 1 16;
    opacity: 0.7;
    pointer-events: none;
    animation: sg-flow linear infinite;
  }
  @keyframes sg-flow {
    to {
      stroke-dashoffset: -34;
    }
  }

  .orb {
    stroke: rgba(0, 0, 0, 0.18);
    stroke-width: 1;
    cursor: pointer;
    transform-box: fill-box;
    transform-origin: center;
    transition: transform 0.18s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .orb-sheen {
    fill: url(#sg-sheen);
    pointer-events: none;
    transform-box: fill-box;
    transform-origin: center;
    transition: transform 0.18s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .node-group:hover .orb,
  .node-group:hover .orb-sheen,
  .node-group:focus-visible .orb,
  .node-group:focus-visible .orb-sheen {
    transform: scale(1.08);
  }
  .node-group:hover .orb {
    filter: brightness(1.15);
  }

  .orb-error {
    fill: none;
    stroke: #f87171;
    stroke-width: 2;
    stroke-dasharray: 4 3;
    pointer-events: none;
    animation: sg-pulse 1.8s ease-in-out infinite;
  }
  @keyframes sg-pulse {
    0%,
    100% {
      opacity: 0.55;
    }
    50% {
      opacity: 1;
    }
  }

  .node-group:focus-visible {
    outline: none;
  }
  .node-group:focus-visible .orb {
    stroke: var(--color-accent, #3b82f6);
    stroke-width: 2.5;
  }

  .labels {
    pointer-events: none;
  }
  .node-count {
    fill: #fff;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 600;
    text-anchor: middle;
    dominant-baseline: central;
    paint-order: stroke;
    stroke: rgba(0, 0, 0, 0.35);
    stroke-width: 2.5px;
  }
  .node-name {
    fill: var(--color-text, #e9eff8);
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.01em;
    text-anchor: middle;
    paint-order: stroke;
    stroke: var(--color-canvas-bg, #070c16);
    stroke-width: 3.5px;
  }
  .node-llm {
    fill: #f59e0b;
    font-size: 11px;
    text-anchor: middle;
    dominant-baseline: central;
  }
  .edge-label {
    fill: var(--color-text-muted, #9aa8bd);
    font-family: var(--font-mono, monospace);
    font-size: 9px;
    text-anchor: middle;
    dominant-baseline: central;
    paint-order: stroke;
    stroke: var(--color-canvas-bg, #070c16);
    stroke-width: 3px;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.9rem;
  }

  @media (prefers-reduced-motion: reduce) {
    .edge-flow,
    .orb-error {
      animation: none;
    }
    .edge-flow {
      opacity: 0.4;
    }
  }
</style>
