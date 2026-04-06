import { writable } from 'svelte/store';
import type { FlameGraphLayout, TimelineLayout, TraceSummary, WasmError, WaterfallLayout } from '../lib/types';

export interface TraceState {
  status: 'empty' | 'loading' | 'loaded' | 'error';
  summary: TraceSummary | null;
  flameLayout: FlameGraphLayout | null;
  timelineLayout: TimelineLayout | null;
  waterfallLayout: WaterfallLayout | null;
  error: WasmError | null;
  isSampleTrace: boolean;
}

const initial: TraceState = {
  status: 'empty',
  summary: null,
  flameLayout: null,
  timelineLayout: null,
  waterfallLayout: null,
  error: null,
  isSampleTrace: false,
};

function createTraceStore() {
  const { subscribe, set, update } = writable<TraceState>(initial);

  return {
    subscribe,
    setLoading() {
      set({ ...initial, status: 'loading' });
    },
    setLoaded(
      summary: TraceSummary,
      flameLayout: FlameGraphLayout,
      timelineLayout: TimelineLayout | null,
      waterfallLayout: WaterfallLayout | null,
      isSampleTrace: boolean
    ) {
      set({ status: 'loaded', summary, flameLayout, timelineLayout, waterfallLayout, error: null, isSampleTrace });
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
