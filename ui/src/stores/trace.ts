import { writable } from 'svelte/store';
import type { AgentFlow, FlameGraphLayout, ServiceGraph, TimelineLayout, TraceSummary, WasmError, WaterfallLayout } from '../lib/types';

export interface TraceState {
  status: 'empty' | 'loading' | 'loaded' | 'error';
  loadingPhase: string | null;
  loadingProgress: number | null;
  summary: TraceSummary | null;
  flameLayout: FlameGraphLayout | null;
  timelineLayout: TimelineLayout | null;
  waterfallLayout: WaterfallLayout | null;
  serviceGraph: ServiceGraph | null;
  agentFlow: AgentFlow | null;
  error: WasmError | null;
  isSampleTrace: boolean;
}

const initial: TraceState = {
  status: 'empty',
  loadingPhase: null,
  loadingProgress: null,
  summary: null,
  flameLayout: null,
  timelineLayout: null,
  waterfallLayout: null,
  serviceGraph: null,
  agentFlow: null,
  error: null,
  isSampleTrace: false,
};

function createTraceStore() {
  const { subscribe, set } = writable<TraceState>(initial);

  return {
    subscribe,
    setLoading(phase = 'Parsing trace JSON', progress: number | null = null) {
      set({ ...initial, status: 'loading', loadingPhase: phase, loadingProgress: progress });
    },
    setLoaded(
      summary: TraceSummary,
      flameLayout: FlameGraphLayout,
      timelineLayout: TimelineLayout | null,
      waterfallLayout: WaterfallLayout | null,
      serviceGraph: ServiceGraph,
      agentFlow: AgentFlow,
      isSampleTrace: boolean
    ) {
      set({
        status: 'loaded',
        loadingPhase: null,
        loadingProgress: null,
        summary,
        flameLayout,
        timelineLayout,
        waterfallLayout,
        serviceGraph,
        agentFlow,
        error: null,
        isSampleTrace,
      });
    },
    setError(error: WasmError) {
      set({ ...initial, status: 'error', error });
    },
    reset() {
      set(initial);
    },
  };
}

export const traceState = createTraceStore();
