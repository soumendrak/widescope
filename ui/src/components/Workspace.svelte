<script lang="ts">
  import { fly } from 'svelte/transition';
  import { viewSlideIn, viewSlideOut } from '../lib/animation';
  import { activeView } from '../stores/selection';
  import type { TraceState } from '../stores/trace';

  import FlameGraph from './FlameGraph.svelte';
  import Timeline from './Timeline.svelte';
  import WaterfallView from './WaterfallView.svelte';
  import ServiceGraph from './ServiceGraph.svelte';
  import DiffView from './DiffView.svelte';
  import ConversationView from './ConversationView.svelte';
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
    <WelcomeState {onLoadSample} {onOpenFile} {onPaste} />
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
    {:else if $activeView === 'diff'}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <DiffView />
      </div>
    {:else}
      <div class="view-wrapper" in:fly={viewSlideIn(slideDirection)} out:fly={viewSlideOut(slideDirection)}>
        <FlameGraph bind:this={flameGraphView} layout={state.flameLayout} />
      </div>
    {/if}
    {#if $activeView !== 'diff'}
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
