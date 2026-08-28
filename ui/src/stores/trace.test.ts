import { describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { traceState } from './trace';
import { traceList } from './traceList';
import type { AgentFlow, FlameGraphLayout, ServiceGraph, TraceSummary } from '../lib/types';

vi.mock('../lib/input', () => ({
  handleRawInput: vi.fn(),
}));
import { handleRawInput } from '../lib/input';

const summary = { span_count: 7, trace_id: 't' } as unknown as TraceSummary;
const flame = { nodes: [], max_depth: 0 } as unknown as FlameGraphLayout;
const graph = { nodes: [], edges: [] } as unknown as ServiceGraph;

describe('trace state', () => {
  it('moves through loading, loaded and back to empty', () => {
    traceState.setLoading('Parsing', 20);
    expect(get(traceState).status).toBe('loading');
    expect(get(traceState).loadingPhase).toBe('Parsing');
    expect(get(traceState).loadingProgress).toBe(20);

    traceState.setLoaded(summary, flame, null, null, graph, { nodes: [], edges: [] } as unknown as AgentFlow, true);
    const loaded = get(traceState);
    expect(loaded.status).toBe('loaded');
    expect(loaded.summary?.span_count).toBe(7);
    expect(loaded.isSampleTrace).toBe(true);
    // A loaded trace clears any previous error.
    expect(loaded.error).toBeNull();

    traceState.reset();
    expect(get(traceState).status).toBe('empty');
    expect(get(traceState).summary).toBeNull();
  });

  it('records a parse error with its context', () => {
    traceState.setError({ code: 'INVALID_JSON', message: 'bad', context: { line: 3 } } as never);
    const state = get(traceState);
    expect(state.status).toBe('error');
    expect(state.error?.code).toBe('INVALID_JSON');
    expect(state.summary).toBeNull();
  });
});

describe('trace list', () => {
  it('adds traces and ignores exact duplicates', () => {
    traceList.clear();
    traceList.add('a', '{"one":1}');
    traceList.add('a-again', '{"one":1}');
    traceList.add('b', '{"two":2}');
    expect(get(traceList).map((e) => e.name)).toEqual(['a', 'b']);
    expect(traceList.count()).toBe(2);
  });

  it('adds several at once, still skipping duplicates', () => {
    traceList.clear();
    traceList.addMultiple([
      { name: 'a', json: '{"one":1}' },
      { name: 'b', json: '{"two":2}' },
      { name: 'dup', json: '{"one":1}' },
    ]);
    expect(get(traceList)).toHaveLength(2);
  });

  it('reloads the selected trace and ignores an out-of-range index', () => {
    traceList.clear();
    traceList.add('a', '{"one":1}');
    vi.mocked(handleRawInput).mockClear();

    traceList.switchTo(0);
    expect(handleRawInput).toHaveBeenCalledWith('{"one":1}', false, true);

    traceList.switchTo(5);
    traceList.switchTo(-1);
    expect(handleRawInput).toHaveBeenCalledTimes(1);
  });
});
