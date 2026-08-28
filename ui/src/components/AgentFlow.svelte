<script lang="ts">
  import type { AgentFlow } from '../lib/types';
  import { selectedSpanId } from '../stores/selection';

  export let flow: AgentFlow;

  // ponytail: layered SVG DAG (layer → row, order → column). Scroll for overflow;
  // skipped pinch/drag pan-zoom — add when traces outgrow a scroll viewport.
  const NODE_W = 150;
  const NODE_H = 46;
  const H_GAP = 28;
  const V_GAP = 56;
  const PAD = 32;

  const KIND_COLOR: Record<string, string> = {
    agent: '#8b5cf6',
    tool: '#3b82f6',
    chain: '#14b8a6',
    retrieval: '#f59e0b',
    llm: '#ec4899',
  };

  interface Placed {
    span_id: string;
    label: string;
    kind: string;
    status: string;
    duration_display: string;
    iteration: number | null;
    cx: number;
    cy: number;
  }

  $: placed = layout(flow);
  $: posById = new Map(placed.map((p) => [p.span_id, p]));
  $: width = Math.max(...placed.map((p) => p.cx + NODE_W / 2), 200) + PAD;
  $: height = Math.max(...placed.map((p) => p.cy + NODE_H / 2), 120) + PAD;

  function layout(f: AgentFlow): Placed[] {
    if (!f?.nodes?.length) return [];
    // Bucket by layer, order each layer by `order`, assign a column index.
    const byLayer = new Map<number, AgentFlow['nodes']>();
    for (const n of f.nodes) {
      const arr = byLayer.get(n.layer) ?? [];
      arr.push(n);
      byLayer.set(n.layer, arr);
    }
    const out: Placed[] = [];
    for (const [layer, nodes] of byLayer) {
      nodes.sort((a, b) => a.order - b.order);
      nodes.forEach((n, col) => {
        out.push({
          span_id: n.span_id,
          label: n.label,
          kind: n.kind,
          status: n.status,
          duration_display: n.duration_display,
          iteration: n.iteration,
          cx: PAD + col * (NODE_W + H_GAP) + NODE_W / 2,
          cy: PAD + layer * (NODE_H + V_GAP) + NODE_H / 2,
        });
      });
    }
    return out;
  }

  function select(spanId: string) {
    selectedSpanId.set(spanId);
  }
</script>

{#if !flow?.nodes?.length}
  <div class="empty">No agentic spans detected in this trace</div>
{:else}
  <div class="agent-scroll">
    <svg {width} {height} role="group" aria-label="Agent flow graph">
      {#each flow.edges as edge (edge.source + edge.target + edge.kind)}
        {@const s = posById.get(edge.source)}
        {@const t = posById.get(edge.target)}
        {#if s && t}
          <line
            x1={s.cx} y1={s.cy + NODE_H / 2}
            x2={t.cx} y2={t.cy - NODE_H / 2}
            class="edge"
            class:edge--sequence={edge.kind === 'sequence'}
          />
        {/if}
      {/each}

      {#each placed as node (node.span_id)}
        <g
          class="node-group"
          role="button"
          tabindex="0"
          aria-label="{node.kind} {node.label}"
          on:click={() => select(node.span_id)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), select(node.span_id))}
        >
          <rect
            x={node.cx - NODE_W / 2} y={node.cy - NODE_H / 2}
            width={NODE_W} height={NODE_H} rx="8"
            class="node"
            class:node--selected={$selectedSpanId === node.span_id}
            class:node--error={node.status === 'Error'}
            style="fill: {KIND_COLOR[node.kind] ?? '#64748b'};"
          />
          <text class="node-kind" x={node.cx - NODE_W / 2 + 10} y={node.cy - 6}>{node.kind}</text>
          <text class="node-label" x={node.cx - NODE_W / 2 + 10} y={node.cy + 12}>{node.label}</text>
          {#if node.iteration !== null && node.iteration > 0}
            <text class="node-iter" x={node.cx + NODE_W / 2 - 8} y={node.cy - 6} text-anchor="end">⟳{node.iteration + 1}</text>
          {/if}
          <title>{node.kind} · {node.label}
{node.duration_display}{node.status === 'Error' ? ' · error' : ''}</title>
        </g>
      {/each}
    </svg>
  </div>
{/if}

<style>
  .agent-scroll {
    flex: 1;
    overflow: auto;
    background: var(--color-canvas-bg, #0f172a);
  }

  svg {
    display: block;
  }

  .edge {
    stroke: var(--color-border, #475569);
    stroke-width: 1.5;
    opacity: 0.7;
  }

  .edge--sequence {
    stroke-dasharray: 4 3;
    opacity: 0.45;
  }

  .node {
    stroke: var(--color-canvas-bg, #0f172a);
    stroke-width: 2;
    cursor: pointer;
  }

  .node:hover {
    filter: brightness(1.15);
  }

  .node--selected {
    stroke: #fff;
    stroke-width: 2.5;
  }

  .node--error {
    stroke: #ef4444;
    stroke-width: 2.5;
  }

  .node-kind {
    fill: rgba(255, 255, 255, 0.7);
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-family: var(--font-mono);
    pointer-events: none;
  }

  .node-label {
    fill: #fff;
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-mono);
    pointer-events: none;
  }

  .node-iter {
    fill: #fff;
    font-size: 10px;
    font-weight: 600;
    pointer-events: none;
  }

  .node-group:focus-visible {
    outline: 2px solid var(--color-accent, #3b82f6);
    outline-offset: 2px;
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
