<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { FlameGraphLayout, FlameNode } from '../lib/types';
  import { selectedSpanId, hoveredSpanId, focusedSpanId, searchResults, sliceStartNs, sliceEndNs } from '../stores/selection';
  import { getCriticalPath } from '../lib/wasm';
  import type { CriticalPath } from '../lib/types';

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
  let heatmapMode = false;
  let showCriticalPath = false;
  let criticalPath: CriticalPath | null = null;
  let criticalPathSet = new Set<string>();
  let sliceEnabled = false;
  let draggingSlice = ''; // 'start' | 'end' | ''

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

  type CanvasPalette = {
    axisBorder: string;
    textMuted: string;
    textStrong: string;
    codeText: string;
    accent: string;
    searchStroke: string;
    dangerOverlay: string;
  };

  export function focusView(): void {
    if (!container) return;
    container.scrollIntoView({ block: 'nearest' });
    container.focus();
  }

  function themeColor(name: string, fallback: string): string {
    if (!container) return fallback;
    const value = getComputedStyle(container).getPropertyValue(name).trim();
    return value || fallback;
  }

  function getCanvasPalette(): CanvasPalette {
    return {
      axisBorder: themeColor('--color-border', '#334155'),
      textMuted: themeColor('--color-text-muted', '#94a3b8'),
      textStrong: themeColor('--color-text', '#e2e8f0'),
      codeText: themeColor('--color-code-text', '#ffffff'),
      accent: themeColor('--color-accent', '#3b82f6'),
      searchStroke: '#fbbf24',
      dangerOverlay: 'rgba(239,68,68,0.45)',
    };
  }

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
    const activeSearchResults = $searchResults;
    const hasSearch = activeSearchResults.length > 0;
    const searchMatchSet = new Set(activeSearchResults);
    const palette = getCanvasPalette();

    const sStart = $sliceStartNs;
    const sEnd = $sliceEndNs;
    const hasSlice = sStart !== null && sEnd !== null && sStart < sEnd;
    let sliceXStart = 0;
    let sliceXEnd = canvasW;

    if (hasSlice && layout.trace_duration_ns > 0) {
      sliceXStart = toPixelX(sStart! / layout.trace_duration_ns);
      sliceXEnd = toPixelX(sEnd! / layout.trace_duration_ns);

      // Dim region outside slice
      ctx.save();
      ctx.fillStyle = 'rgba(0,0,0,0.35)';
      ctx.fillRect(0, 0, sliceXStart, canvasH);
      ctx.fillRect(sliceXEnd, 0, canvasW - sliceXEnd, canvasH);
      ctx.restore();

      // Slice zone highlight
      ctx.save();
      ctx.fillStyle = 'rgba(59,130,246,0.05)';
      ctx.fillRect(sliceXStart, 0, sliceXEnd - sliceXStart, canvasH);
      ctx.strokeStyle = 'rgba(59,130,246,0.5)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.strokeRect(sliceXStart, 0, sliceXEnd - sliceXStart, canvasH);
      ctx.setLineDash([]);
      ctx.restore();
    }

    for (const node of layout.nodes) {
      drawNode(node, sel, hov, foc, hasSearch, searchMatchSet, palette, hasSlice, sStart, sEnd, sliceXStart, sliceXEnd);
    }
    drawTimeAxis(palette);

    if (hasSlice) {
      drawSliceLabels(sliceXStart, sliceXEnd, sStart!, sEnd!);
    }
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
    foc: string | null,
    hasSearch: boolean,
    searchMatchSet: Set<string>,
    palette: CanvasPalette,
    hasSlice: boolean = false,
    sStart: number | null = null,
    sEnd: number | null = null,
    sliceXStart: number = 0,
    sliceXEnd: number = 0
  ) {
    const px = toPixelX(node.x);
    const pw = toPixelW(node.width);
    if (px + pw < 0 || px > canvasW) return;

    if (hasSlice && layout.trace_duration_ns > 0) {
      const nodeTs = node.x * layout.trace_duration_ns;
      const nodeTe = (node.x + node.width) * layout.trace_duration_ns;
      if (nodeTe < sStart! || nodeTs > sEnd!) {
        ctx!.save();
        ctx!.globalAlpha = 0.2;
        drawNodeBody(node, px, pw, sel, hov, foc, hasSearch, searchMatchSet, palette);
        ctx!.restore();
        return;
      }
    }

    drawNodeBody(node, px, pw, sel, hov, foc, hasSearch, searchMatchSet, palette);
  }

  function drawNodeBody(
    node: FlameNode,
    px: number,
    pw: number,
    sel: string | null,
    hov: string | null,
    foc: string | null,
    hasSearch: boolean,
    searchMatchSet: Set<string>,
    palette: CanvasPalette
  ) { // cull

    const py = node.depth * ROW_HEIGHT;
    const minW = 1;
    const rw = Math.max(pw, minW);
    const isSearchMatch = !hasSearch || searchMatchSet.has(node.span_id);

    const baseColor = heatmapMode
      ? heatmapColor(node)
      : (colorMap.get(node.color_key) ?? '#64748b');
    ctx!.save();
    if (hasSearch && !isSearchMatch) {
      ctx!.globalAlpha = 0.22;
    }
    ctx!.fillStyle = baseColor;
    ctx!.fillRect(px, py, rw, ROW_HEIGHT - 1);

    if (node.is_error) {
      ctx!.fillStyle = palette.dangerOverlay;
      ctx!.fillRect(px, py, rw, ROW_HEIGHT - 1);
    }

    if (hasSearch && isSearchMatch) {
      ctx!.strokeStyle = palette.searchStroke;
      ctx!.lineWidth = 1.5;
      ctx!.strokeRect(px + 0.75, py + 0.75, rw - 1.5, ROW_HEIGHT - 2.5);
    }

    if (node.span_id === sel) {
      ctx!.strokeStyle = palette.codeText;
      ctx!.lineWidth = 2;
      ctx!.strokeRect(px + 1, py + 1, rw - 2, ROW_HEIGHT - 3);
    } else if (node.span_id === hov) {
      ctx!.strokeStyle = palette.textStrong;
      ctx!.lineWidth = 1;
      ctx!.strokeRect(px + 0.5, py + 0.5, rw - 1, ROW_HEIGHT - 2);
    }

    if (node.span_id === foc && node.span_id !== sel) {
      ctx!.strokeStyle = palette.textStrong;
      ctx!.lineWidth = 2;
      ctx!.setLineDash([3, 3]);
      ctx!.strokeRect(px + 1, py + 1, rw - 2, ROW_HEIGHT - 3);
      ctx!.setLineDash([]);
    }

    if (showCriticalPath && criticalPathSet.has(node.span_id)) {
      ctx!.strokeStyle = '#fbbf24';
      ctx!.lineWidth = 2;
      ctx!.strokeRect(px + 1, py + 1, rw - 2, ROW_HEIGHT - 3);
    }

    if (node.is_llm && pw > 14) {
      ctx!.font = '10px sans-serif';
      ctx!.fillStyle = palette.codeText;
      ctx!.fillText('⚡', px + 2, py + ROW_HEIGHT - 7);
    }

    if (pw > MIN_LABEL_PX) {
      ctx!.font = '11px system-ui, sans-serif';
      ctx!.fillStyle = palette.codeText;
      ctx!.save();
      ctx!.rect(px + 2, py, rw - 4, ROW_HEIGHT - 1);
      ctx!.clip();
      ctx!.fillText(node.label, px + (node.is_llm ? 14 : 4), py + ROW_HEIGHT - 7);
      ctx!.restore();
    }
    ctx!.restore();
  }

  function drawTimeAxis(palette: CanvasPalette) {
    const y = (layout.max_depth + 1) * ROW_HEIGHT;
    if (!ctx) return;
    ctx.fillStyle = palette.axisBorder;
    ctx.fillRect(0, y, canvasW, 1);

    const totalNs = layout.trace_duration_ns;
    if (!totalNs) return;

    const tickCount = Math.max(4, Math.floor(canvasW / 120));
    ctx.font = '10px monospace';
    ctx.fillStyle = palette.textMuted;

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

  function drawSliceLabels(sx: number, ex: number, startNs: number, endNs: number) {
    if (!ctx) return;
    const y = (layout.max_depth + 1) * ROW_HEIGHT + 4;
    ctx.font = '9px monospace';
    ctx.fillStyle = '#93c5fd';
    ctx.fillText(formatNs(startNs), sx + 4, y + 10);
    ctx.fillText(formatNs(endNs), ex - 60, y + 10);
  }

  function clearSlice() {
    sliceStartNs.set(null);
    sliceEndNs.set(null);
  }

  function sliceToSpan() {
    const sel = $selectedSpanId;
    if (!sel || !nodeMap) return;
    const node = nodeMap.get(sel);
    if (!node || !layout || layout.trace_duration_ns <= 0) return;
    sliceStartNs.set(Math.round(node.x * layout.trace_duration_ns));
    sliceEndNs.set(Math.round((node.x + node.width) * layout.trace_duration_ns));
  }

  function sliceToAll(): void {
    if (!layout) return;
    sliceStartNs.set(0);
    sliceEndNs.set(layout.trace_duration_ns);
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

  function exportPng(): void {
    if (!canvas) return;
    canvas.toBlob((blob) => {
      if (!blob) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'flamegraph.png';
      a.click();
      URL.revokeObjectURL(url);
    }, 'image/png');
  }

  function toggleHeatmap(): void {
    heatmapMode = !heatmapMode;
    scheduleRender();
  }

  function toggleCriticalPath(): void {
    showCriticalPath = !showCriticalPath;
    if (showCriticalPath) {
      criticalPath = getCriticalPath();
      criticalPathSet = new Set(criticalPath?.span_ids ?? []);
    } else {
      criticalPathSet = new Set();
    }
    scheduleRender();
  }

  function heatmapColor(node: FlameNode): string {
    if (layout.trace_duration_ns <= 0) return '#64748b';
    const ratio = Math.min(node.duration_ns / layout.trace_duration_ns, 1);
    const r = Math.round(200 + ratio * 55);
    const g = Math.round(200 - ratio * 160);
    const b = Math.round(60 - ratio * 40);
    return `rgb(${r},${g},${b})`;
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
  const unsubSearch = searchResults.subscribe(() => scheduleRender());

  onDestroy(() => {
    unsubSel();
    unsubHov();
    unsubFoc();
    unsubSearch();
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
  ></canvas>
  <div class="controls">
    <button class="ctrl-btn" title="Reset zoom (0)" on:click={resetZoom}>Reset</button>
    <button class="ctrl-btn" title="Fit selection (F)" on:click={() => {
      const sel = $selectedSpanId;
      const n = sel ? nodeMap.get(sel) : null;
      if (n) zoomToSpan(n); else resetZoom();
    }}>Fit</button>
    <button class="ctrl-btn" class:ctrl-btn--active={heatmapMode} title="Toggle latency heatmap" on:click={toggleHeatmap}>Heatmap</button>
    <button class="ctrl-btn" class:ctrl-btn--active={showCriticalPath} title="Show critical path" on:click={toggleCriticalPath}>Critical</button>
    <button class="ctrl-btn" title="Slice to selected span" on:click={sliceToSpan}>Slice</button>
    {#if $sliceStartNs !== null && $sliceEndNs !== null}
      <button class="ctrl-btn" title="Clear time slice" on:click={clearSlice}>✕</button>
    {/if}
    <button class="ctrl-btn" title="Download PNG" on:click={exportPng}>PNG</button>
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
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--color-border, #334155);
    color: var(--color-text, #e2e8f0);
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .ctrl-btn:hover {
    border-color: var(--color-accent, #3b82f6);
    background: var(--color-panel-highlight, rgba(255, 255, 255, 0.04));
  }

  .ctrl-btn--active {
    border-color: var(--color-accent, #3b82f6);
    background: rgba(59, 130, 246, 0.15);
    color: #fff;
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
