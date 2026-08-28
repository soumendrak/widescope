<script lang="ts">
  import Icon, { type IconName } from './ui/Icon.svelte';
  import { traceState } from '../stores/trace';
  import { traceList } from '../stores/traceList';
  import { openFilePicker } from '../lib/input';
  import { theme } from '../lib/theme';
  import { buildShareUrl, isShareSupported } from '../lib/permalink';
  import { searchSpans } from '../lib/wasm';
  import {
    activeView,
    focusedSpanId,
    searchQuery,
    searchResults,
    selectedSpanId,
    fullscreen,
  } from '../stores/selection';
  import { budgets, type BudgetViolation } from '../stores/budgets';
  import { viewTabs } from '../lib/views';

  export let onOpenFile: () => void = () => openFilePicker();
  export let violations: BudgetViolation[] = [];
  export let onOpenBudgets: () => void;

  $: summary = $traceState.summary;
  $: status = $traceState.status;
  $: themeIcon = ($theme === 'dark' ? 'sun' : 'moon') satisfies IconName as IconName;
  $: searchMessage = $searchQuery.trim()
    ? ($searchResults.length > 0
        ? `${$searchResults.length} match${$searchResults.length === 1 ? '' : 'es'}`
        : `No spans match '${$searchQuery.trim()}'`)
    : '';
  $: traceCount = $traceList.length;
  $: activeTraceIdx = $traceList.findIndex(e => $traceState.summary && (e.json.includes($traceState.summary.trace_id)));

  // Tab list, the 1-5 shortcuts and the slide direction all read from
  // lib/views.ts so they cannot drift apart.
  $: VIEW_TABS = viewTabs((summary?.llm_span_count ?? 0) > 0);

  function switchTrace(index: number): void {
    traceList.switchTo(index);
  }

  function applySearch(nextQuery: string): void {
    searchQuery.set(nextQuery);
    if (status !== 'loaded') { searchResults.set([]); return; }
    const q = nextQuery.trim();
    if (!q) { searchResults.set([]); focusedSpanId.set(null); return; }
    const matches = searchSpans(q);
    searchResults.set(matches);
    if (matches.length === 0) { selectedSpanId.set(null); focusedSpanId.set(null); return; }
    const current = $selectedSpanId ?? $focusedSpanId;
    if (!current || !matches.includes(current)) {
      selectedSpanId.set(matches[0]);
      focusedSpanId.set(matches[0]);
    }
  }

  function focusSearchResult(offset: number): void {
    if ($searchResults.length === 0) return;
    const current = $selectedSpanId ?? $focusedSpanId;
    const idx = current ? $searchResults.indexOf(current) : -1;
    const nextIdx = idx === -1
      ? (offset >= 0 ? 0 : $searchResults.length - 1)
      : (idx + offset + $searchResults.length) % $searchResults.length;
    selectedSpanId.set($searchResults[nextIdx]);
    focusedSpanId.set($searchResults[nextIdx]);
  }

  function onSearchKeyDown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') { event.preventDefault(); focusSearchResult(1); }
    else if (event.key === 'ArrowUp') { event.preventDefault(); focusSearchResult(-1); }
  }

  function onSearchInput(event: Event): void {
    applySearch((event.currentTarget as HTMLInputElement).value);
  }

  // --- Share / permalink ---

  type SharePopover = { text: string; kind: 'success' | 'warn' | 'error'; download: boolean };

  let sharePopover: SharePopover | null = null;
  let sharePopoverTimer: ReturnType<typeof setTimeout> | null = null;

  $: activeTraceJson = activeTraceIdx >= 0 ? ($traceList[activeTraceIdx]?.json ?? null) : null;

  function dismissSharePopover(): void {
    if (sharePopoverTimer) { clearTimeout(sharePopoverTimer); sharePopoverTimer = null; }
    sharePopover = null;
  }

  function showSharePopover(popover: SharePopover, autoDismissMs = 0): void {
    if (sharePopoverTimer) clearTimeout(sharePopoverTimer);
    sharePopover = popover;
    sharePopoverTimer = autoDismissMs > 0
      ? setTimeout(() => { sharePopover = null; sharePopoverTimer = null; }, autoDismissMs)
      : null;
  }

  async function shareTrace(): Promise<void> {
    if (sharePopover) { dismissSharePopover(); return; }
    const json = activeTraceJson;
    if (!json) {
      showSharePopover({ text: 'No trace loaded to share.', kind: 'error', download: false }, 3500);
      return;
    }
    if (!isShareSupported()) {
      showSharePopover({
        text: 'This browser cannot build share links. Download the trace and share the file instead.',
        kind: 'warn',
        download: true,
      });
      return;
    }
    try {
      const result = await buildShareUrl({ json, view: $activeView, spanId: $selectedSpanId });
      if (result.tooLarge) {
        const kb = Math.round(result.dataChars / 1024);
        showSharePopover({
          text: `Trace is too large for a self-contained link (~${kb} KB). Download the trace and share the file, or host it and open it with ?trace=<url>.`,
          kind: 'warn',
          download: true,
        });
        return;
      }
      await navigator.clipboard.writeText(result.url);
      showSharePopover({ text: 'Share link copied to clipboard.', kind: 'success', download: false }, 2800);
    } catch {
      showSharePopover({
        text: 'Could not create or copy a share link. Download the trace instead.',
        kind: 'error',
        download: true,
      }, 5000);
    }
  }

  function downloadTrace(): void {
    const json = activeTraceJson;
    if (!json) return;
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `widescope-trace-${summary?.trace_id ?? 'export'}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    dismissSharePopover();
  }
</script>

<div class="top-bar">
    <div class="top-left">
      <a class="brand" href="/" title="WideScope home">
        <img class="logo" src="/widescope-logo.svg" alt="" width="26" height="26" />
        <span class="name">WideScope</span>
      </a>
      <button type="button" class="btn-open" on:click={onOpenFile}>Open file <kbd>⌘O</kbd></button>

      {#if traceCount > 1}
        <select class="trace-select" value={activeTraceIdx} on:change={(e) => switchTrace(parseInt(e.currentTarget.value))} aria-label="Switch trace">
          {#each $traceList as entry, i}
            <option value={i}>{entry.name}</option>
          {/each}
        </select>
        <span class="trace-count">{traceCount} traces</span>
      {/if}
    </div>

    <div class="top-center">
      {#if status === 'loading'}
        <span class="status-loading">parsing…</span>
      {/if}
    </div>

    <div class="top-right">
      {#if status === 'loaded'}
        <div class="search-shell">
          <span class="search-icon" aria-hidden="true">⌕</span>
          <input
            type="search"
            class="search-input"
            value={$searchQuery}
            placeholder="search spans · duration>100ms"
            aria-label="Search spans"
            on:input={onSearchInput}
            on:keydown={onSearchKeyDown}
          />
          <kbd class="search-kbd" aria-hidden="true">⌘K</kbd>
          <button
            type="button"
            class="search-nav"
            aria-label="Previous search result"
            disabled={$searchResults.length === 0}
            on:click={() => focusSearchResult(-1)}
          >↑</button>
          <button
            type="button"
            class="search-nav"
            aria-label="Next search result"
            disabled={$searchResults.length === 0}
            on:click={() => focusSearchResult(1)}
          >↓</button>
          {#if searchMessage}
            <span class="search-status" class:search-status--empty={$searchQuery.trim() && $searchResults.length === 0}>
              {searchMessage}
            </span>
          {/if}
        </div>

        <div class="view-tabs" role="tablist" aria-label="View mode">
          {#each VIEW_TABS as tab, i}
            <button
              type="button"
              class="view-tab"
              class:view-tab--active={$activeView === tab.id}
              role="tab"
              aria-selected={$activeView === tab.id}
              aria-controls="view-panel"
              title="{tab.label} ({i + 1})"
              on:click={() => activeView.set(tab.id)}
            ><span class="vt-ic">{tab.glyph}</span><span class="vt-label">{tab.label}</span></button>
          {/each}
        </div>
      {/if}

      {#if status === 'loaded'}
        <div class="share-wrap">
          <button
            type="button"
            class="share-btn"
            class:share-btn--active={sharePopover !== null}
            aria-label="Share this trace"
            title="Copy a self-contained share link"
            on:click={shareTrace}
          ><Icon name="link" size={13} /> Share</button>
          {#if sharePopover}
            <div class="share-popover share-popover--{sharePopover.kind}" role="status">
              <span class="share-popover-text">{sharePopover.text}</span>
              <div class="share-popover-actions">
                {#if sharePopover.download}
                  <button type="button" class="share-popover-btn" on:click={downloadTrace}>Download trace</button>
                {/if}
                <button type="button" class="share-popover-close" aria-label="Dismiss" on:click={dismissSharePopover}><Icon name="close" size={12} /></button>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <button
        type="button"
        class="budgets-btn"
        class:budgets-btn--violated={violations.length > 0}
        aria-label="Performance budgets"
        title={violations.length > 0 ? `${violations.length} budget violation${violations.length === 1 ? '' : 's'}` : 'Performance budgets'}
        on:click={onOpenBudgets}
      >
        <Icon name="target" size={15} />
        {#if $budgets.length > 0}
          <span class="budgets-count" class:budgets-count--violated={violations.length > 0}>
            {violations.length > 0 ? violations.length : $budgets.length}
          </span>
        {/if}
      </button>
      <button type="button" class="theme-btn" aria-label="Toggle theme" on:click={() => theme.toggle()}><Icon name={themeIcon} size={15} /></button>
      <button
        type="button"
        class="fullscreen-btn"
        aria-label={$fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
        title={$fullscreen ? 'Exit fullscreen (Esc)' : 'Fullscreen (Shift+F)'}
        on:click={() => fullscreen.update(v => !v)}
      ><Icon name={$fullscreen ? 'collapse' : 'expand'} size={15} /></button>
    </div>
  </div>

<style>
  .top-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 0.9rem;
    height: 50px;
  }
  .top-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-shrink: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.02rem;
    letter-spacing: -0.01em;
    color: inherit;
    text-decoration: none;
    transition: opacity 0.15s ease;
  }
  .brand:hover { opacity: 0.85; }
  .logo {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    box-shadow: 0 4px 14px -4px rgba(29, 78, 216, 0.55);
  }
  .top-center {
    flex: 1;
    min-width: 0;
  }
  .top-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }
  .btn-open {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.38rem 0.8rem;
    background: var(--grad-cta);
    color: #fff;
    border: none;
    border-radius: 8px;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    white-space: nowrap;
    box-shadow: var(--shadow-cta);
    transition: transform 0.18s var(--ease-out), box-shadow 0.18s var(--ease-spring);
  }
  .btn-open kbd {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    border: 1px solid rgba(255, 255, 255, 0.35);
    border-radius: 4px;
    padding: 0 0.26rem;
    background: rgba(255, 255, 255, 0.12);
  }
  .btn-open:hover {
    transform: translateY(-1px);
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.28) inset, 0 12px 30px -8px rgba(37, 99, 235, 0.65);
  }
  .trace-select {
    padding: 0.26rem 0.45rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    outline: none;
    cursor: pointer;
    max-width: 180px;
  }
  .trace-select:focus { border-color: var(--color-sky, #7dd3fc); }
  .trace-count {
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.66rem;
    letter-spacing: 0.08em;
    white-space: nowrap;
  }
  .status-loading {
    color: var(--color-sky, #7dd3fc);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    animation: loading-pulse 1.6s ease-in-out infinite;
  }
  .search-shell {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    position: relative;
  }
  .search-icon {
    position: absolute;
    left: 0.6rem;
    color: var(--color-text-faint, #5b6b84);
    font-size: 0.9rem;
    pointer-events: none;
  }
  .search-kbd {
    position: absolute;
    right: 4.2rem;
    font-family: var(--font-mono);
    font-size: 0.58rem;
    color: var(--color-text-faint, #5b6b84);
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 0.28rem;
    pointer-events: none;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
  }
  .search-input {
    width: 230px;
    padding: 0.42rem 2.6rem 0.42rem 1.8rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 9px;
    background: var(--color-canvas-bg, #070c16);
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.74rem;
    outline: none;
    transition: border-color 0.18s var(--ease-spring), box-shadow 0.18s var(--ease-spring), width 0.25s var(--ease-out);
  }
  .search-input::placeholder { color: var(--color-text-faint, #5b6b84); }
  .search-input:focus {
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent, #3b82f6) 16%, transparent);
  }
  .search-nav {
    padding: 0.3rem 0.5rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    cursor: pointer;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring);
  }
  .search-nav:hover:not(:disabled) {
    background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
  }
  .search-nav:disabled { cursor: not-allowed; opacity: 0.4; }
  .search-status {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.68rem;
  }
  .view-tabs {
    display: flex;
    gap: 2px;
    background: var(--color-canvas-bg, #070c16);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 9px;
    padding: 3px;
  }
  .view-tab {
    display: inline-flex;
    align-items: center;
    gap: 0.36rem;
    height: 26px;
    padding: 0 0.65rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--color-toolbar-muted, #8b9cb5);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: background 0.15s var(--ease-spring), color 0.15s var(--ease-spring);
  }
  .vt-ic { font-size: 0.72rem; opacity: 0.8; }
  .view-tab:hover {
    color: var(--color-toolbar-text, #e9eff8);
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
  }
  .theme-btn {
    padding: 0.32rem 0.55rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-text, #e9eff8);
    font-size: 0.8rem;
    cursor: pointer;
    line-height: 1;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-bounce);
  }
  .theme-btn:hover {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    transform: scale(1.06);
  }
  .budgets-btn {
    position: relative;
    padding: 0.32rem 0.55rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-text, #e9eff8);
    font-size: 0.8rem;
    cursor: pointer;
    line-height: 1;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-bounce);
  }
  .budgets-btn:hover {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    transform: scale(1.06);
  }
  .budgets-count {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 7px;
    background: var(--color-accent, #3b82f6);
    color: white;
    font-size: 0.6rem;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
  }
  .fullscreen-btn {
    padding: 0.32rem 0.55rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-muted, #8b9cb5);
    font-size: 0.8rem;
    cursor: pointer;
    line-height: 1;
    transition: color 0.15s var(--ease-spring), background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring);
  }
  .fullscreen-btn:hover {
    color: var(--color-toolbar-text, #e9eff8);
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
  }
  .share-wrap {
    position: relative;
    display: flex;
  }
  .share-btn {
    padding: 0.32rem 0.6rem;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 7px;
    background: transparent;
    color: var(--color-toolbar-text, #e9eff8);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    cursor: pointer;
    line-height: 1;
    white-space: nowrap;
    transition: background 0.15s var(--ease-spring), border-color 0.15s var(--ease-spring), transform 0.15s var(--ease-bounce);
  }
  .share-btn:hover {
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
    border-color: color-mix(in srgb, var(--color-sky, #7dd3fc) 38%, transparent);
    transform: scale(1.04);
  }
  .share-popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 30;
    width: 270px;
    padding: 0.7rem 0.8rem;
    border-radius: 10px;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    background: color-mix(in srgb, var(--color-surface, #0d1626) 96%, transparent);
    backdrop-filter: blur(10px);
    box-shadow: var(--shadow-panel);
    font-size: 0.78rem;
    line-height: 1.5;
  }
    .vt-label { display: none; }
    .view-tab { padding: 0 0.55rem; }
    .vt-ic { font-size: 0.8rem; opacity: 1; }
    .search-input { width: 170px; }
    .search-kbd { display: none; }
    .top-bar { flex-wrap: wrap; height: auto; padding: 0.4rem 0.5rem; gap: 0.4rem 0.5rem; }
    .top-right {
      flex-wrap: wrap;
      flex-basis: 100%;
      justify-content: flex-end;
      row-gap: 0.45rem;
    }
    .search-shell { flex: 1 1 100%; }
    .search-input { flex: 1; width: auto; min-width: 0; }

  .search-status--empty { color: var(--color-gold, #fcd34d); }
  .view-tab--active {
    background: var(--grad-cta);
    color: #fff;
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.25) inset, 0 4px 12px -4px rgba(37, 99, 235, 0.6);
  }
  .view-tab--active .vt-ic { opacity: 1; }
  .budgets-btn--violated { border-color: rgba(248, 113, 113, 0.55); }
  .budgets-count--violated { background: #f87171; }
  .share-btn--active { background: var(--color-panel-highlight, rgba(125, 211, 252, 0.05)); }
  .share-popover--success { border-left: 3px solid #22c55e; }
  .share-popover--warn { border-left: 3px solid #f59e0b; }
  .share-popover--error { border-left: 3px solid #ef4444; }
  .share-popover-text {
    display: block;
    color: var(--color-toolbar-text, #f1f5f9);
  }
  .share-popover-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }
  .share-popover-btn {
    padding: 0.22rem 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 5px;
    background: transparent;
    color: var(--color-toolbar-text, #f1f5f9);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .share-popover-btn:hover { background: rgba(255, 255, 255, 0.1); }
  .share-popover-close {
    padding: 0.1rem 0.35rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--color-toolbar-muted, #94a3b8);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .share-popover-close:hover { color: var(--color-toolbar-text, #f1f5f9); background: rgba(255, 255, 255, 0.1); }

  @media (max-width: 1180px) {
    .vt-label { display: none; }
    .view-tab { padding: 0 0.55rem; }
    .vt-ic { font-size: 0.8rem; opacity: 1; }
    .search-input { width: 170px; }
    .search-kbd { display: none; }
  }

  @media (max-width: 820px) {
    .top-bar { flex-wrap: wrap; height: auto; padding: 0.4rem 0.5rem; gap: 0.4rem 0.5rem; }

    /* Let the right-hand cluster wrap onto its own rows instead of
       overflowing the viewport: search takes a full row, the tabs and
       action buttons flow onto the next. */
    .top-right {
      flex-wrap: wrap;
      flex-basis: 100%;
      justify-content: flex-end;
      row-gap: 0.45rem;
    }
    .search-shell { flex: 1 1 100%; }
    .search-input { flex: 1; width: auto; min-width: 0; }
  }
</style>
