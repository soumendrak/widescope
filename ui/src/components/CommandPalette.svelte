<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import { searchSpans, getSpanDetail } from '../lib/wasm';
  import { activeView, selectedSpanId, focusedSpanId, searchQuery, searchResults, fullscreen } from '../stores/selection';
  import { traceState } from '../stores/trace';
  import { theme } from '../lib/theme';
  import { showCriticalPath } from '../stores/criticalPath';
  import { viewTabs } from '../lib/views';
  import Icon, { type IconName } from './ui/Icon.svelte';

  /**
   * One surface for everything: jump to a span, switch lens, load the sample,
   * flip the theme, export. Replaces the old ⌘K, which only moved focus into
   * the search box.
   */

  export let open = false;
  export let onLoadSample: () => void;
  export let onOpenFile: () => void;
  export let onPaste: () => void;
  export let onExport: () => void;

  const dispatch = createEventDispatcher<{ close: void }>();

  interface Command {
    id: string;
    label: string;
    hint?: string;
    group: 'Spans' | 'Views' | 'Trace' | 'Display';
    icon?: IconName;
    run: () => void;
  }

  let query = '';
  let cursor = 0;
  let inputEl: HTMLInputElement;
  let listEl: HTMLElement;
  let previouslyFocused: HTMLElement | null = null;

  $: summary = $traceState.summary;
  $: hasTrace = $traceState.status === 'loaded';
  $: hasLlm = (summary?.llm_span_count ?? 0) > 0;

  // Static commands are always offered; span results are query-driven.
  $: staticCommands = buildStatic(hasTrace, hasLlm, $theme, $showCriticalPath);

  function buildStatic(loaded: boolean, llm: boolean, current: string, critical: boolean): Command[] {
    const cmds: Command[] = [];

    if (loaded) {
      for (const tab of viewTabs(llm)) {
        cmds.push({
          id: `view:${tab.id}`,
          label: `Go to ${tab.label}`,
          hint: tab.glyph,
          group: 'Views',
          run: () => activeView.set(tab.id),
        });
      }
    }

    cmds.push(
      { id: 'trace:sample', label: 'Load sample trace', group: 'Trace', icon: 'bolt', run: onLoadSample },
      { id: 'trace:open', label: 'Open trace file…', hint: '⌘O', group: 'Trace', run: onOpenFile },
      { id: 'trace:paste', label: 'Paste trace JSON', hint: '⌘V', group: 'Trace', run: onPaste },
    );
    if (loaded) {
      cmds.push({ id: 'trace:export', label: 'Download trace JSON', group: 'Trace', icon: 'link', run: onExport });
    }

    cmds.push(
      {
        id: 'display:theme',
        label: current === 'dark' ? 'Switch to light theme' : 'Switch to dark theme',
        group: 'Display',
        icon: current === 'dark' ? 'sun' : 'moon',
        run: () => theme.toggle(),
      },
      {
        id: 'display:fullscreen',
        label: 'Toggle fullscreen',
        hint: '⇧F',
        group: 'Display',
        icon: 'expand',
        run: () => fullscreen.update((v) => !v),
      },
    );
    if (loaded) {
      cmds.push({
        id: 'display:critical',
        label: critical ? 'Hide critical path' : 'Show critical path',
        group: 'Display',
        icon: 'target',
        run: () => showCriticalPath.update((v) => !v),
      });
    }
    return cmds;
  }

  /** Span hits come from the same WASM search the toolbar uses, so operators work here too. */
  function spanCommands(q: string): Command[] {
    if (!hasTrace || q.trim().length < 2) return [];
    let ids: string[] = [];
    try {
      ids = searchSpans(q.trim()).slice(0, 8);
    } catch {
      return [];
    }
    return ids.flatMap((id) => {
      try {
        const d = getSpanDetail(id);
        return [{
          id: `span:${id}`,
          label: d.operation_name,
          hint: `${d.service_name} · ${d.duration_display}`,
          group: 'Spans' as const,
          run: () => {
            selectedSpanId.set(id);
            focusedSpanId.set(id);
            searchQuery.set(q.trim());
            searchResults.set(ids);
          },
        }];
      } catch {
        return [];
      }
    });
  }

  function matches(c: Command, q: string): boolean {
    if (!q) return true;
    return (c.label + ' ' + (c.hint ?? '')).toLowerCase().includes(q.toLowerCase());
  }

  $: results = [
    ...spanCommands(query),
    ...staticCommands.filter((c) => matches(c, query)),
  ];
  $: if (cursor >= results.length) cursor = Math.max(0, results.length - 1);

  // Group headers are rendered by comparing each row to the one before it.
  $: rows = results.map((c, i) => ({
    cmd: c,
    startsGroup: i === 0 || results[i - 1].group !== c.group,
  }));

  $: if (open) void enter();

  async function enter(): Promise<void> {
    previouslyFocused = document.activeElement as HTMLElement | null;
    query = '';
    cursor = 0;
    await tick();
    inputEl?.focus();
  }

  function close(): void {
    dispatch('close');
    previouslyFocused?.focus?.();
    previouslyFocused = null;
  }

  function runAt(i: number): void {
    const cmd = results[i];
    if (!cmd) return;
    close();
    cmd.run();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') { e.preventDefault(); close(); return; }
    if (e.key === 'ArrowDown') { e.preventDefault(); cursor = (cursor + 1) % Math.max(1, results.length); scrollToCursor(); return; }
    if (e.key === 'ArrowUp') { e.preventDefault(); cursor = (cursor - 1 + results.length) % Math.max(1, results.length); scrollToCursor(); return; }
    if (e.key === 'Enter') { e.preventDefault(); runAt(cursor); return; }
    // Tab must not escape the palette.
    if (e.key === 'Tab') { e.preventDefault(); }
  }

  async function scrollToCursor(): Promise<void> {
    await tick();
    listEl?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }
</script>

{#if open}
  <div class="backdrop">
    <button type="button" class="backdrop-dismiss" tabindex="-1" aria-hidden="true" on:click={close}></button>
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <div class="field">
        <span class="field-icon" aria-hidden="true">⌕</span>
        <input
          bind:this={inputEl}
          bind:value={query}
          on:keydown={onKeydown}
          class="field-input"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-list"
          aria-autocomplete="list"
          aria-activedescendant={results[cursor] ? `pal-${cursor}` : undefined}
          placeholder="Search spans, switch views, run a command…"
          spellcheck="false"
        />
        <kbd class="field-kbd">esc</kbd>
      </div>

      <div class="list" id="palette-list" role="listbox" aria-label="Commands" bind:this={listEl}>
        {#each rows as { cmd, startsGroup }, i (cmd.id)}
          {#if startsGroup}
            <div class="group" role="presentation">{cmd.group}</div>
          {/if}
          <div
            id="pal-{i}"
            class="row"
            class:row--active={i === cursor}
            role="option"
            aria-selected={i === cursor}
            tabindex="-1"
            on:click={() => runAt(i)}
            on:mousemove={() => (cursor = i)}
            on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); runAt(i); } }}
          >
            {#if cmd.icon}
              <span class="row-icon"><Icon name={cmd.icon} size={14} /></span>
            {:else}
              <span class="row-icon row-icon--glyph">{cmd.hint && cmd.group === 'Views' ? cmd.hint : ''}</span>
            {/if}
            <span class="row-label">{cmd.label}</span>
            {#if cmd.hint && cmd.group !== 'Views'}
              <span class="row-hint">{cmd.hint}</span>
            {/if}
          </div>
        {:else}
          <div class="empty">No commands match “{query}”.</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 12vh var(--space-4) var(--space-4);
    background: color-mix(in srgb, var(--color-bg) 70%, transparent);
    backdrop-filter: blur(4px);
  }

  .backdrop-dismiss {
    position: absolute;
    inset: 0;
    border: 0;
    padding: 0;
    background: transparent;
    cursor: default;
  }

  .palette {
    position: relative;
    width: 100%;
    max-width: 560px;
    max-height: 60dvh;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    overflow: hidden;
  }

  .field {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border-bottom: 1px solid var(--color-border);
  }

  .field-icon { color: var(--color-text-faint); font-size: var(--text-md); }

  .field-input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--color-text);
    font-family: inherit;
    font-size: var(--text-md);
    outline: none;
  }

  .field-input::placeholder { color: var(--color-text-faint); }

  .field-kbd {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-faint);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs);
    padding: 0.1rem 0.3rem;
  }

  .list { overflow-y: auto; padding: var(--space-1); }

  .group {
    padding: var(--space-2) var(--space-2) var(--space-1);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-text-faint);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: var(--text-sm);
    color: var(--color-text);
  }

  .row--active { background: var(--color-badge-bg); }

  .row-icon {
    width: 16px;
    display: inline-flex;
    justify-content: center;
    color: var(--color-text-muted);
    flex: none;
  }

  .row-icon--glyph { font-size: var(--text-xs); }

  .row-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-hint {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  .empty {
    padding: var(--space-5);
    text-align: center;
    font-size: var(--text-sm);
    color: var(--color-text-muted);
  }
</style>
