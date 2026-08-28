<script lang="ts">
  import { onDestroy } from 'svelte';

  /**
   * The raw-JSON drawer. Owns only its own presentation — text, collapse state
   * and drag-resize; parsing stays in the workspace above it. Auto-collapse on
   * a successful load is driven from there via `collapse()`.
   *
   * There is no Submit button: parsing is live (150ms debounce on every
   * keystroke), so a submit control could only re-run work already done. ⌘⏎
   * survives as `onSubmit` because it still does something useful — collapse
   * the drawer and move focus to the trace.
   */
  export let value = '';
  export let collapsed = false;
  export let message: string | null = null;
  export let onInput: () => void;
  export let onSubmit: () => void;
  export let onClear: () => void;
  export let onLoadSample: () => void;
  export let onPaste: () => void;
  export let onFormat: () => void;

  const DEFAULT_HEIGHT_PX = 280;
  const EMPTY_HEIGHT_PX = 160;
  const COLLAPSED_HEIGHT_PX = 88;
  const AUTO_EXPAND_DELTA_PX = 24;

  let inputEl: HTMLTextAreaElement;

  $: sizeLabel = value.trim()
    ? `${value.length.toLocaleString()} chars`
    : 'empty';
  let resizeObserver: ResizeObserver | null = null;
  let currentHeight = DEFAULT_HEIGHT_PX;
  let expandedHeight = DEFAULT_HEIGHT_PX;
  let isResizing = false;
  let resizeStartY = 0;
  let resizeStartHeight = 0;

  export function expand(): void {
    collapsed = false;
    currentHeight = Math.max(expandedHeight, DEFAULT_HEIGHT_PX);
  }

  export function collapse(): void {
    expandedHeight = Math.max(DEFAULT_HEIGHT_PX, currentHeight);
    collapsed = true;
    currentHeight = COLLAPSED_HEIGHT_PX;
  }

  /** Focus the textarea and put the caret at a 1-based line — used to land on a parse error. */
  export function focusLine(line: number): void {
    if (!inputEl) return;
    let charOffset = 0;
    const lines = value.split('\n');
    for (let i = 0; i < line - 1 && i < lines.length; i++) charOffset += lines[i].length + 1;
    inputEl.focus();
    inputEl.setSelectionRange(charOffset, charOffset);
    inputEl.scrollTop = Math.max(0, (line - 1) * 20 - 60);
  }

  function handleKeyDown(e: KeyboardEvent): void {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      onSubmit();
    }
  }

  function beginResize(event: PointerEvent): void {
    event.preventDefault();
    isResizing = true;
    resizeStartY = event.clientY;
    resizeStartHeight = currentHeight;
    document.body.style.cursor = 'ns-resize';
    document.body.style.userSelect = 'none';
  }

  function onWindowPointerMove(event: PointerEvent): void {
    if (!isResizing) return;
    const next = Math.max(COLLAPSED_HEIGHT_PX, resizeStartHeight + event.clientY - resizeStartY);
    currentHeight = next;
    if (next > COLLAPSED_HEIGHT_PX + AUTO_EXPAND_DELTA_PX) {
      collapsed = false;
      expandedHeight = Math.max(DEFAULT_HEIGHT_PX, next);
    } else if (!collapsed) {
      expandedHeight = Math.max(DEFAULT_HEIGHT_PX, next);
    }
  }

  function endResize(): void {
    if (!isResizing) return;
    isResizing = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }

  // The textarea only exists while the drawer is open, so (re)observe it each
  // time it appears rather than once on mount.
  $: if (inputEl) observeInput();

  function observeInput(): void {
    resizeObserver?.disconnect();
    resizeObserver = new ResizeObserver((entries) => {
      const next = Math.max(
        COLLAPSED_HEIGHT_PX,
        Math.round(entries[0]?.contentRect.height ?? inputEl.getBoundingClientRect().height)
      );
      currentHeight = next;
      if (collapsed) {
        if (next > COLLAPSED_HEIGHT_PX + AUTO_EXPAND_DELTA_PX) {
          collapsed = false;
          expandedHeight = Math.max(DEFAULT_HEIGHT_PX, next);
        }
        return;
      }
      expandedHeight = Math.max(DEFAULT_HEIGHT_PX, next);
    });
    resizeObserver.observe(inputEl);
  }

  onDestroy(() => resizeObserver?.disconnect());
</script>

<svelte:window on:pointermove={onWindowPointerMove} on:pointerup={endResize} on:pointercancel={endResize} />

<section
  class="editor-panel"
  class:editor-panel--collapsed={collapsed}
  class:editor-panel--empty={!value.trim()}
>
  <!--
    Collapsed, the drawer is a slim strip: the trace is the hero, JSON is source.
    Everything below the strip only exists when it is open.
  -->
  <div class="editor-strip">
    <button
      type="button"
      class="editor-disclosure"
      aria-expanded={!collapsed}
      aria-controls="editor-body"
      on:click={() => (collapsed ? expand() : collapse())}
    >
      <span class="editor-caret" class:editor-caret--open={!collapsed} aria-hidden="true">▸</span>
      <span class="editor-strip-title">source · trace.json</span>
    </button>
    {#if !collapsed && value.trim()}
      <kbd class="editor-strip-kbd" title="Collapse the editor and jump to the trace">⌘⏎</kbd>
    {/if}
    <span class="editor-strip-meta">{sizeLabel}</span>
    <div class="editor-strip-spacer"></div>
    {#if message}
      <span class="editor-message">{message}</span>
    {/if}
    <div class="editor-actions">
      <button type="button" class="editor-btn" on:click={onLoadSample}>Sample</button>
      <button type="button" class="editor-btn" on:click={onPaste}>Paste</button>
      {#if !collapsed}
        <button type="button" class="editor-btn" on:click={onClear} disabled={!value.trim()}>Clear</button>
        <button type="button" class="editor-btn" on:click={onFormat} disabled={!value.trim()}>Format</button>
      {/if}
    </div>
  </div>

  {#if !collapsed}
    <div class="editor-body" id="editor-body">
      <div class="editor-input-shell">
        <div
          class="editor-resize-handle"
          class:editor-resize-handle--active={isResizing}
          role="separator"
          aria-label="Resize trace JSON input"
          aria-orientation="horizontal"
          on:pointerdown={beginResize}
        ></div>
        <textarea
          class="editor-input"
          bind:this={inputEl}
          bind:value={value}
          on:input={onInput}
          on:keydown={handleKeyDown}
          placeholder="Paste a trace JSON payload here…"
          spellcheck="false"
          aria-label="Trace JSON input"
          style={`height: ${value.trim() ? currentHeight : EMPTY_HEIGHT_PX}px;`}
        ></textarea>
      </div>
      <div class="editor-footer">
        <span class="editor-hint">OTLP · Jaeger · OpenInference JSON — live parse is on as you type</span>
      </div>
    </div>
  {/if}
</section>

<style>
  .editor-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.1rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 14px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface, #0d1626) 96%, var(--color-accent)), var(--color-surface, #0d1626));
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.1) inset;
    transition: padding 0.28s var(--ease-spring), gap 0.28s var(--ease-spring);
  }
  .editor-panel--collapsed {
    gap: 0;
    padding: 0 0.4rem;
  }
  .editor-panel--empty {
    border-style: dashed;
    box-shadow: none;
    opacity: 0.96;
  }
  .editor-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }
  /*
   * One button style. Every control here is secondary — Sample, Paste, Clear,
   * Format — since Submit was removed as redundant with live parse, so the old
   * primary/ghost pair collapsed into this.
   */
  .editor-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.5rem 0.95rem;
    background: transparent;
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: 9px;
    font-family: var(--font-mono);
    font-size: 0.76rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring),
      color 0.15s var(--ease-spring), opacity 0.15s var(--ease-spring);
  }
  .editor-btn:hover:not(:disabled) {
    background: var(--color-panel-subtle);
    border-color: color-mix(in srgb, var(--color-sky) 38%, transparent);
  }
  .editor-btn:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
  .editor-input-shell {
    position: relative;
    padding-bottom: 12px;
  }
  .editor-input {
    width: 100%;
    min-height: 88px;
    resize: none;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 10px;
    padding: 1rem;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-text, #e9eff8);
    font: 400 0.84rem/1.65 var(--font-mono);
    outline: none;
    transition: min-height 0.28s var(--ease-spring), max-height 0.28s var(--ease-spring), padding 0.28s var(--ease-spring), border-color 0.18s var(--ease-spring), box-shadow 0.18s var(--ease-spring);
  }
  .editor-input::placeholder {
    color: var(--color-text-faint, #5b6b84);
  }
  .editor-input:focus {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent, #3b82f6) 18%, transparent);
  }
  .editor-resize-handle {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 12px;
    cursor: ns-resize;
  }
  .editor-resize-handle::before {
    content: '';
    position: absolute;
    left: 50%;
    bottom: 4px;
    transform: translateX(-50%);
    width: 72px;
    height: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-text-muted, #94a3b8) 45%, transparent);
    transition: background 0.15s var(--ease-spring), width 0.15s var(--ease-spring);
  }
  .editor-input-shell:hover .editor-resize-handle::before,
  .editor-resize-handle--active::before {
    width: 108px;
    background: color-mix(in srgb, var(--color-accent, #3b82f6) 70%, transparent);
  }
  .editor-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .editor-panel--collapsed .editor-footer {
    display: none;
  }
  .editor-hint {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    letter-spacing: 0.05em;
    color: var(--color-text-faint, #5b6b84);
  }
  .editor-message {
    font-family: var(--font-mono);
    font-size: 0.74rem;
    color: var(--color-danger, #f87171);
  }
  @media (max-width: 820px) {
    .editor-strip,
    .editor-footer {
      align-items: stretch;
    }

    .editor-actions,
    .editor-btn {
      width: 100%;
    }

    .editor-actions {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .editor-btn:last-child {
      grid-column: 1 / -1;
    }
  }

  @media (max-width: 520px) {
    .editor-actions {
      grid-template-columns: 1fr;
    }
  }

  /* --- collapsed strip: the drawer's resting state ---------------------- */
  .editor-strip {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    min-height: 40px;
  }

  .editor-disclosure {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0;
    border: 0;
    background: none;
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .editor-disclosure:hover { color: var(--color-sky); }

  .editor-caret {
    display: inline-block;
    transition: transform var(--dur-fast) var(--ease-out);
    color: var(--color-text-faint);
  }

  .editor-caret--open { transform: rotate(90deg); }

  .editor-strip-title { letter-spacing: 0.02em; }

  .editor-strip-meta {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-faint);
  }

  .editor-strip-spacer { flex: 1; }

  .editor-body {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--color-border-soft);
  }

  @media (prefers-reduced-motion: reduce) {
    .editor-caret { transition: none; }
  }

  .editor-strip-kbd {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-faint);
    border: 1px solid var(--color-border-soft);
    border-radius: var(--radius-xs);
    padding: 0.05rem 0.3rem;
  }
</style>
