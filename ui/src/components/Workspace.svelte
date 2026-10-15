<script lang="ts">
  import { fly } from 'svelte/transition';
  import { viewSlideIn, viewSlideOut } from '../lib/animation';
  import { activeView } from '../stores/selection';
  import type { TraceState } from '../stores/trace';
  import type { ViewName } from '../lib/types';

  /** Lenses that describe a set of traces, not a span — no inspector rail. */
  const AGGREGATE_VIEWS = new Set<ViewName>(['diff', 'analytics', 'matrix', 'dashboard']);

  import FlameGraph from './FlameGraph.svelte';
  import Timeline from './Timeline.svelte';
  import WaterfallView from './WaterfallView.svelte';
  import ServiceGraph from './ServiceGraph.svelte';
  import DiffView from './DiffView.svelte';
  import ConversationView from './ConversationView.svelte';
  import AgentFlow from './AgentFlow.svelte';
  import AgentTimeline from './AgentTimeline.svelte';
  import ComparisonTable from './ComparisonTable.svelte';
  import TokenTrends from './TokenTrends.svelte';
  import DashboardView from './DashboardView.svelte';
  import SpanDetail from './SpanDetail.svelte';
  import EmptyState from './ui/EmptyState.svelte';
  import WelcomeState from './WelcomeState.svelte';
  import ViewSkeleton from './ui/ViewSkeleton.svelte';

  /** The visualization stage: one lens at a time, plus the persistent inspector rail. */
  export let state: TraceState;
  /** +1 slides the incoming view in from the right, -1 from the left. */
  export let slideDirection: 1 | -1 = 1;
  /** True before any trace has been entered — the stage hosts the load actions. */
  export let empty = false;
  export let onLoadSample: () => void;
  export let onOpenFile: () => void;
  export let onPaste: () => void;
  /** Load raw JSON text — the first-run "recent traces" chips use this. */
  export let onLoadText: (text: string) => void = () => {};

  /** Agent flow ships two readings of the same data; the toggle lives in-view. */
  let agentSubview: 'flow' | 'timeline' = 'flow';

  let flameGraphView: { focusView: () => void } | null = null;
  let timelineView: { focusView: () => void } | null = null;
  let waterfallView: { focusView: () => void } | null = null;

  /** Focus whichever lens is showing, so keyboard nav lands somewhere useful. */
  export function focusActiveLens(): void {
    if ($activeView === 'waterfall') waterfallView?.focusView();
    else if ($activeView === 'timeline') timelineView?.focusView();
    else flameGraphView?.focusView();
  }
</script>

<div class="workspace" id="view-panel" role="tabpanel">
  {#if empty}
    <WelcomeState {onLoadSample} {onOpenFile} {onPaste} {onLoadText} />
  {:else if state.status === 'loaded' && state.flameLayout}
    {#if $activeView === 'conversation'}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <ConversationView />
      </div>
    {:else if $activeView === 'timeline' && state.timelineLayout}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <Timeline bind:this={timelineView} layout={state.timelineLayout} />
      </div>
    {:else if $activeView === 'waterfall' && state.waterfallLayout}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <WaterfallView bind:this={waterfallView} layout={state.waterfallLayout} />
      </div>
    {:else if $activeView === 'graph' && state.serviceGraph}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <ServiceGraph graph={state.serviceGraph} />
      </div>
    {:else if $activeView === 'agent' && state.agentFlow}
      <div class="view-wrapper agent-view" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <div class="agent-subview-toggle" role="tablist" aria-label="Agent view mode">
          <button type="button" role="tab" class:active={agentSubview === 'flow'} aria-selected={agentSubview === 'flow'} on:click={() => (agentSubview = 'flow')}>Flow</button>
          <button type="button" role="tab" class:active={agentSubview === 'timeline'} aria-selected={agentSubview === 'timeline'} on:click={() => (agentSubview = 'timeline')}>Timeline</button>
        </div>
        {#if agentSubview === 'timeline'}
          <AgentTimeline flow={state.agentFlow} />
        {:else}
          <AgentFlow flow={state.agentFlow} />
        {/if}
      </div>
    {:else if $activeView === 'diff'}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <DiffView />
      </div>
    {:else if $activeView === 'analytics'}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <TokenTrends />
      </div>
    {:else if $activeView === 'matrix'}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <ComparisonTable />
      </div>
    {:else if $activeView === 'dashboard'}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <DashboardView />
      </div>
    {:else}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <FlameGraph bind:this={flameGraphView} layout={state.flameLayout} />
      </div>
    {/if}
    {#if !AGGREGATE_VIEWS.has($activeView)}
      <SpanDetail />
    {/if}
  {:else if state.status === 'error'}
    <EmptyState
      icon="warning"
      tone="danger"
      title="Could not parse trace"
      description="Update the JSON in the editor — the views refresh as soon as the payload becomes valid."
    >
      {#if state.error?.code === 'INVALID_JSON' && state.error?.context}
        <div class="error-context">
          {#if state.error.context.line !== undefined && state.error.context.line !== null}
            Line {state.error.context.line}{state.error.context.column !== undefined && state.error.context.column !== null ? `, column ${state.error.context.column}` : ''}
          {/if}
        </div>
      {/if}
    </EmptyState>
  {:else if state.status === 'loading'}
    <ViewSkeleton
      label={state.loadingPhase ?? 'Parsing trace JSON'}
      progress={state.loadingProgress ?? 20}
    />
  {:else}
    <EmptyState
      icon="warning"
      tone="muted"
      title="Nothing to show yet"
      description="The payload parsed but produced no spans to visualize."
    />
  {/if}
</div>

<style>
  .workspace {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
    position: relative;
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 16px;
    background: var(--color-canvas-bg, #070c16);
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.08) inset;
  }
  .view-wrapper {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .view-wrapper.agent-view {
    flex-direction: column;
  }
  .agent-subview-toggle {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    margin: var(--space-2) var(--space-2) 0;
    border: 1px solid var(--color-border-soft);
    border-radius: 7px;
    background: color-mix(in srgb, var(--color-canvas-bg) 60%, transparent);
    align-self: flex-start;
  }
  .agent-subview-toggle button {
    background: none;
    border: 0;
    border-radius: 5px;
    padding: 0.2rem 0.8rem;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-text-muted);
  }
  .agent-subview-toggle button.active {
    background: var(--color-badge-bg);
    color: var(--color-sky);
  }
  .error-context {
    margin-top: 0.5rem;
    padding: 0.35rem 0.65rem;
    border: 1px solid var(--color-error-border, #fca5a5);
    border-radius: 6px;
    background: var(--color-error-bg, #fee2e2);
    color: var(--color-error-text, #991b1b);
    font-family: monospace;
    font-size: 0.8125rem;
  }
  :global(.app--fullscreen) .workspace {
    border-radius: 0;
    border: none;
  }

  /*
   * Below 820px there is no room for a 320px rail beside the stage, so the
   * master-detail split becomes a vertical stack: view on top, inspector as a
   * sheet under it. Read-only mobile parity, per the plan's guardrail.
   */
  @media (max-width: 820px) {
    .workspace { flex-direction: column; }
  }
</style>
