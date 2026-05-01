import { writable } from 'svelte/store';
import type { ComparisonSummary, FlameGraphLayout } from '../lib/types';
import { parseComparisonTrace, getComparisonFlamegraph, clearComparison as clearWasmComparison } from '../lib/wasm';

export interface ComparisonState {
  status: 'empty' | 'loading' | 'loaded' | 'error';
  summary: ComparisonSummary | null;
  flameLayout: FlameGraphLayout | null;
  error: string | null;
}

const initial: ComparisonState = {
  status: 'empty',
  summary: null,
  flameLayout: null,
  error: null,
};

function createComparisonStore() {
  const { subscribe, set } = writable<ComparisonState>(initial);

  return {
    subscribe,
    setLoading() {
      set({ ...initial, status: 'loading' });
    },
    setLoaded(summary: ComparisonSummary, flameLayout: FlameGraphLayout) {
      set({ status: 'loaded', summary, flameLayout, error: null });
    },
    setError(error: string) {
      set({ ...initial, status: 'error', error });
    },
    loadComparison(raw: string) {
      try {
        const summary = parseComparisonTrace(raw);
        const flameLayout = getComparisonFlamegraph();
        this.setLoaded(summary, flameLayout);
      } catch (e) {
        this.setError(String(e));
      }
    },
    clear() {
      clearWasmComparison();
      set(initial);
    },
    reset() {
      set(initial);
    },
  };
}

export const comparisonState = createComparisonStore();
