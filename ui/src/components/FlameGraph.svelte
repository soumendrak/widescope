<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { FlameGraphLayout, FlameNode } from '../lib/types';
  import { selectedSpanId, hoveredSpanId, focusedSpanId } from '../stores/selection';

  export let layout: FlameGraphLayout;

  const ROW_HEIGHT = 24;
  const MIN_LABEL_PX = 40;
  const BUCKET_COUNT = 64;

  // 12 accessible service colors
  const SERVICE_COLORS = [
    '#3b82f6', '#10b981', '#f59e0b', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316',
    '#6366f1', '#14b8a6', '#eab308', '#ef4444',
  ];

  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let animFrameId = 0;

  // Zoom / pan state
  let zoom = 1;
  let panX = 0; // offset in normalized coords

  // Node cache keyed by span_id
  let nodeMap = new Map<string, FlameNode>();
  // Spatial buckets for hit-testing
  let buckets: FlameNode[][] = [];
  // Service-to-color mapping
  let colorMap = new Map<string, string>();

  // Canvas dimensions
  let canvasW = 0;
  let canvasH = 0;

  // Live-region for screen reader announcements
  let ariaLive = '';

  // Keyboard navigation state (children/siblings derived from layout)
  let childrenOf = new Map<string, string[]>();
  let parentOf = new Map<string, string>();
  let siblingsBefore = new Map<string, string[]>();
  let siblingsAfter = new Map<string, string[]>();
  let rootIds: string[] = [];

  // ── Reactive layout ingestion ─────────────────────────────────────
  $: if (layout) initLayout(layout);

  function initLayout(l: FlameGraphLayout) {
    nodeMap = new Map(l.nodes.map((n) => [n.span_id, n]));

    // Assign colors
    colorMap = new Map();
    let colorIdx = 0;
    for (const n of l.nodes) {
      if (!colorMap.has(n.color_key)) {
        colorMap.set(n.color_key, SERVICE_COLORS[colorIdx % SERVICE_COLORS.length]);
        colorIdx++;
      }
    }

    // Build hierarchy from depths
    childrenOf = new Map();
    parentOf = new Map();
    const nodesByDepth: FlameNode[][] = [];
    for (const n of l.nodes) {
      if (!nodesByDepth[n.depth]) nodesByDepth[n.depth] = [];
      nodesByDepth[n.depth].push(n);
      childrenOf.set(n.span_id, []);
    }

    // Determine parent by finding the deepest ancestor that contains this node's x-range
    for (let d = 1; d < nodesByDepth.length; d++) {
      for (const child of nodesByDepth[d]) {
        let bestParent: FlameNode | null = null;
        for (const parent of (nodesByDepth[d - 1] ?? [])) {
          if (parent.x <= child.x && parent.x + parent.width >= child.x + child.width - 1e-12) {
            if (!bestParent || parent.width < bestParent.width) {
              bestParent = parent;
            }
          }
        }
        if (bestParent) {
          parentOf.set(child.span_id, bestParent.span_id);
          childrenOf.get(bestParent.span_id)!.push(child.span_id);
        }
      }
    }

    rootIds = (nodesByDepth[0] ?? []).map((n) => n.span_id);

    // Build sibling maps
    siblingsBefore = new Map();
    siblingsAfter = new Map();
    function buildSiblings(siblings: string[]) {
      for (let i = 0; i < siblings.length; i++) {
        siblingsBefore.set(siblings[i], siblings.slice(0, i));
        siblingsAfter.set(siblings[i], siblings.slice(i + 1));
      }
    }
    buildSiblings(rootIds);
    for (const [, children] of childrenOf) {
      if (children.length > 0) buildSiblings(children);
    }

    // Build spatial index
    buckets = Array.from({ length: BUCKET_COUNT }, () => []);
    for (const n of l.nodes) {
      const bStart = Math.floor(n.x * BUCKET_COUNT);
      const bEnd = Math.floor((n.x + n.width) * BUCKET_COUNT);
      for (let b = bStart; b <= Math.min(bEnd, BUCKET_COUNT - 1); b++) {
        buckets[b].push(n);
      }
    }

    // Initial focus
    if (rootIds.length > 0 && $focusedSpanId === null) {
      focusedSpanId.set(rootIds[0]);
    }

    scheduleRender();
  }

  // ── Canvas setup ──────────────────────────────────────────────────
  onMount(() => {
    ctx = canvas.getContext('2d');
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(container);
    return () => ro.disconnect();
  });

  function resize() {
    if (!container || !canvas) return;
    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvasW = rect.width;
    canvasH = (layout.max_depth + 1) * ROW_HEIGHT + ROW_HEIGHT; // +1 for time axis
    canvas.width = canvasW * dpr;
    canvas.height = canvasH * dpr;
    canvas.style.width = `${canvasW}px`;
    canvas.style.height = `${canvasH}px`;
    if (ctx) ctx.scale(dpr, dpr);
    scheduleRender();
  }

  // ── Render loop ───────────────────────────────────────────────────
  function scheduleRender() {
    cancelAnimationFrame(animFrameId);
    animFrameId = requestAnimationFrame(render);
  }

  function render() {
    if (!ctx || !layout || canvasW === 0) return;
    ctx.clearRect(0, 0, canvasW, canvasH);

    const sel = $selectedSpanId;
    const hov = $hoveredSpanId;
    const foc = $focusedSpanId;

    for (const node of layout.nodes) {
      drawNode(node, sel, hov, foc);
    }
    drawTimeAxis();
  }

  function toPixelX(normX: number): number {
    return (normX - panX) * zoom * canvasW;
  }

  function toPixelW(normW: number): number {
    return normW * zoom * canvasW;
  }

  function drawNode(
    node: FlameNode,
    sel: string | null,
    hov: string | null,
    foc: string | null
  ) {
    const px = toPixelX(node.x);
    const pw = toPixelW(node.width);
    if (px + pw < 0 || px > canvasW) return; // cull

    const py = node.depth * ROW_HEIGHT;
    const minW = 1;
    const rw = Math.max(pw, minW);

    const baseColor = colorMap.get(node.color_key) ?? '#64748b';
    ctx!.fillStyle = baseColor;
    ctx!.fillRect(px, py, rw, ROW_HEIGHT - 1);

    if (node.is_error) {
      ctx!.fillStyle = 'rgba(239,68,68,0.45)';
      ctx!.fillRect(px, py, rw, ROW_HEIGHT - 1);
    }

    if (node.span_id === sel) {
      ctx!.strokeStyle = '#fff';
      ctx!.lineWidth = 2;
      ctx!.strokeRect(px + 1, py + 1, rw - 2, ROW_HEIGHT - 3);
    } else if (node.span_id === hov) {
      ctx!.strokeStyle = 'rgba(255,255,255,0.6)';
      ctx!.lineWidth = 1;
      ctx!.strokeRect(px + 0.5, py + 0.5, rw - 1, ROW_HEIGHT - 2);
    }

    if (node.span_id === foc && node.span_id !== sel) {
      ctx!.strokeStyle = '#fff';
      ctx!.lineWidth = 2;
      ctx!.setLineDash([3, 3]);
      ctx!.strokeRect(px + 1, py + 1, rw - 2, ROW_HEIGHT - 3);
      ctx!.setLineDash([]);
    }

    if (node.is_llm && pw > 14) {
      ctx!.font = '10px sans-serif';
      ctx!.fillStyle = 'rgba(255,255,255,0.9)';
      ctx!.fillText('⚡', px + 2, py + ROW_HEIGHT - 7);
    }

    if (pw > MIN_LABEL_PX) {
      ctx!.font = '11px system-ui, sans-serif';
      ctx!.fillStyle = '#fff';
      ctx!.save();
      ctx!.rect(px + 2, py, rw - 4, ROW_HEIGHT - 1);
      ctx!.clip();
      ctx!.fillText(node.label, px + (node.is_llm ? 14 : 4), py + ROW_HEIGHT - 7);
      ctx!.restore();
    }
  }

  function drawTimeAxis() {
    const y = (layout.max_depth + 1) * ROW_HEIGHT;
    if (!ctx) return;
    ctx.fillStyle = 'var(--color-border, #e2e8f0)';
    ctx.fillRect(0, y, canvasW, 1);

    const totalNs = layout.trace_duration_ns;
    if (!totalNs) return;

    const tickCount = Math.max(4, Math.floor(canvasW / 120));
    ctx.font = '10px monospace';
    ctx.fillStyle = 'var(--color-text-muted, #94a3b8)';

    for (let i = 0; i <= tickCount; i++) {
      const normX = i / tickCount;
      const px = toPixelX(normX);
      if (px < 0 || px > canvasW) continue;
      const ns = Math.round(normX * totalNs / zoom + panX * totalNs);
      const label = formatNs(ns);
      ctx.fillRect(px, y, 1, 5);
      ctx.fillText(label, px + 2, y + 14);
    }
  }

  function formatNs(ns: number): string {
    if (ns < 1_000) return `${ns}ns`;
    if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)}μs`;
    if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(1)}ms`;
    return `${(ns / 1_000_000_000).toFixed(2)}s`;
  }

  // ── Hit-testing ───────────────────────────────────────────────────
  function hitTest(mouseX: number, mouseY: number): FlameNode | null {
    const depth = Math.floor(mouseY / ROW_HEIGHT);
    const normX = mouseX / (canvasW * zoom) + panX;
    const bucketIdx = Math.floor(normX * BUCKET_COUNT);
    const bucket = buckets[Math.max(0, Math.min(bucketIdx, BUCKET_COUNT - 1))] ?? [];

    let result: FlameNode | null = null;
    for (const node of bucket) {
      if (node.depth === depth && normX >= node.x && normX <= node.x + node.width) {
        result = node;
      }
    }
    return result;
  }

  // ── Mouse handlers ────────────────────────────────────────────────
  let isDragging = false;
  let dragStartX = 0;
  let dragStartPan = 0;

  function onMouseMove(e: MouseEvent) {
    if (isDragging) {
      const dx = (e.offsetX - dragStartX) / (canvasW * zoom);
      panX = Math.max(0, Math.min(dragStartPan - dx, 1 - 1 / zoom));
      scheduleRender();
      return;
    }
    const node = hitTest(e.offsetX, e.offsetY);
    hoveredSpanId.set(node?.span_id ?? null);
    scheduleRender();
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button === 0) {
      isDragging = true;
      dragStartX = e.offsetX;
      dragStartPan = panX;
    }
  }

  function onMouseUp(e: MouseEvent) {
    if (isDragging && Math.abs(e.offsetX - dragStartX) < 4) {
      // treat as click
      const node = hitTest(e.offsetX, e.offsetY);
      if (node) {
        selectedSpanId.set(node.span_id);
        focusedSpanId.set(node.span_id);
      } else {
        selectedSpanId.set(null);
      }
    }
    isDragging = false;
    scheduleRender();
  }

  function onMouseLeave() {
    hoveredSpanId.set(null);
    isDragging = false;
    scheduleRender();
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      const factor = e.deltaY < 0 ? 1.2 : 1 / 1.2;
      const normAtMouse = e.offsetX / (canvasW * zoom) + panX;
      zoom = Math.max(1, Math.min(zoom * factor, 100));
      panX = Math.max(0, Math.min(normAtMouse - e.offsetX / (canvasW * zoom), 1 - 1 / zoom));
    } else {
      // vertical scroll handled by container
    }
    scheduleRender();
  }

  function onDblClick(e: MouseEvent) {
    const node = hitTest(e.offsetX, e.offsetY);
    if (node) zoomToSpan(node);
  }

  function zoomToSpan(node: FlameNode) {
    if (node.width <= 0) return;
    zoom = Math.min(1 / node.width, 100);
    panX = Math.max(0, Math.min(node.x, 1 - 1 / zoom));
    scheduleRender();
  }

  function resetZoom() {
    zoom = 1;
    panX = 0;
    scheduleRender();
  }

  // ── Keyboard navigation ───────────────────────────────────────────
  function onKeyDown(e: KeyboardEvent) {
    const foc = $focusedSpanId;
    if (!foc) {
      if (rootIds.length > 0) {
        focusedSpanId.set(rootIds[0]);
        announceNode(rootIds[0]);
      }
      return;
    }

    switch (e.key) {
      case 'ArrowUp': {
        e.preventDefault();
        const parent = parentOf.get(foc);
        if (parent) { focusedSpanId.set(parent); announceNode(parent); }
        break;
      }
      case 'ArrowDown': {
        e.preventDefault();
        const children = childrenOf.get(foc) ?? [];
        if (children.length > 0) { focusedSpanId.set(children[0]); announceNode(children[0]); }
        break;
      }
      case 'ArrowLeft': {
        e.preventDefault();
        const before = siblingsBefore.get(foc) ?? [];
        const target = before.length > 0 ? before[before.length - 1] : (siblingsAfter.get(foc) ?? []).at(-1);
        if (target) { focusedSpanId.set(target); announceNode(target); }
        break;
      }
      case 'ArrowRight': {
        e.preventDefault();
        const after = siblingsAfter.get(foc) ?? [];
        const target = after.length > 0 ? after[0] : (siblingsBefore.get(foc) ?? [])[0];
        if (target) { focusedSpanId.set(target); announceNode(target); }
        break;
      }
      case 'Enter': {
        e.preventDefault();
        selectedSpanId.set(foc);
        break;
      }
      case 'Escape': {
        e.preventDefault();
        selectedSpanId.set(null);
        break;
      }
      case 'Home': {
        e.preventDefault();
        if (rootIds.length > 0) { focusedSpanId.set(rootIds[0]); announceNode(rootIds[0]); }
        break;
      }
      case 'End': {
        e.preventDefault();
        if (rootIds.length > 0) { const last = rootIds[rootIds.length - 1]; focusedSpanId.set(last); announceNode(last); }
        break;
      }
      case '0': {
        e.preventDefault();
        resetZoom();
        break;
      }
      case 'f':
      case 'F': {
        e.preventDefault();
        const sel = $selectedSpanId;
        const n = sel ? nodeMap.get(sel) : (rootIds[0] ? nodeMap.get(rootIds[0]) : null);
        if (n) zoomToSpan(n); else resetZoom();
        break;
      }
    }
    scheduleRender();
  }

  function announceNode(id: string) {
    const n = nodeMap.get(id);
    if (!n) return;
    const childCount = (childrenOf.get(id) ?? []).length;
    ariaLive = `${n.label}, ${n.duration_display}, ${childCount} children`;
  }

  // ── Store subscriptions ───────────────────────────────────────────
  const unsubSel = selectedSpanId.subscribe(() => scheduleRender());
  const unsubHov = hoveredSpanId.subscribe(() => scheduleRender());
  const unsubFoc = focusedSpanId.subscribe((id) => {
    if (id) ensureVisible(id);
    scheduleRender();
  });

  onDestroy(() => {
    unsubSel();
    unsubHov();
    unsubFoc();
    cancelAnimationFrame(animFrameId);
  });

  function ensureVisible(id: string) {
    const n = nodeMap.get(id);
    if (!n || canvasW === 0) return;
    const pxLeft = toPixelX(n.x);
    const pxRight = toPixelX(n.x + n.width);
    if (pxLeft < 0) panX = Math.max(0, n.x);
    else if (pxRight > canvasW) panX = Math.min(1 - 1 / zoom, n.x + n.width - 1 / zoom);
  }
</script>

<div
  class="flame-wrapper"
  bind:this={container}
  tabindex="0"
  role="tree"
  aria-label="Flame graph — use arrow keys to navigate"
  on:keydown={onKeyDown}
>
  <canvas
    bind:this={canvas}
    class="flame-canvas"
    style="height: {canvasH}px;"
    on:mousemove={onMouseMove}
    on:mousedown={onMouseDown}
    on:mouseup={onMouseUp}
    on:mouseleave={onMouseLeave}
    on:wheel|passive={onWheel}
    on:dblclick={onDblClick}
  />
  <div class="controls">
    <button class="ctrl-btn" title="Reset zoom (0)" on:click={resetZoom}>Reset</button>
    <button class="ctrl-btn" title="Fit selection (F)" on:click={() => {
      const sel = $selectedSpanId;
      const n = sel ? nodeMap.get(sel) : null;
      if (n) zoomToSpan(n); else resetZoom();
    }}>Fit</button>
  </div>
  <!-- Screen reader live region -->
  <div class="sr-only" aria-live="polite" aria-atomic="true">{ariaLive}</div>
</div>

<style>
  .flame-wrapper {
    flex: 1;
    position: relative;
    overflow: auto;
    outline: none;
    background: var(--color-canvas-bg, #0f172a);
    min-height: 0;
  }

  .flame-wrapper:focus-visible {
    outline: 2px solid var(--color-accent, #3b82f6);
    outline-offset: -2px;
  }

  .flame-canvas {
    display: block;
    cursor: crosshair;
    min-width: 100%;
  }

  .controls {
    position: absolute;
    top: 6px;
    right: 10px;
    display: flex;
    gap: 4px;
  }

  .ctrl-btn {
    padding: 0.2rem 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.8);
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .ctrl-btn:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
