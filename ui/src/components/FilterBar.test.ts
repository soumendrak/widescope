import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import FilterBar from './FilterBar.svelte';
import { traceState } from '../stores/trace';
import { filterKind, filterLlmOnly, filterStatus, filteredSpanIds } from '../stores/selection';
import type { AgentFlow, FlameGraphLayout, ServiceGraph, TraceSummary } from '../lib/types';

vi.mock('../lib/wasm', () => ({
  filterSpans: vi.fn(() => []),
  getCostBreakdown: vi.fn(() => ({ total_cost_usd: 0.00384, entries: [] })),
}));
import { filterSpans } from '../lib/wasm';

const summary = {
  trace_id: 't',
  span_count: 7,
  service_count: 3,
  detected_format: 'OtlpJson',
  llm_span_count: 4,
  error_count: 0,
  has_errors: false,
  total_duration_display: '4.23s',
  latency_p50_display: '750.0ms',
  latency_p95_display: '4.23s',
  warnings: [],
} as unknown as TraceSummary;

function loadTrace(): void {
  traceState.setLoaded(
    summary,
    { nodes: [], max_depth: 0 } as unknown as FlameGraphLayout,
    null,
    null,
    { nodes: [], edges: [] } as unknown as ServiceGraph,
    { nodes: [], edges: [] } as unknown as AgentFlow,
    false,
  );
}

describe('FilterBar', () => {
  beforeEach(() => {
    filterStatus.set('');
    filterKind.set('');
    filterLlmOnly.set(false);
    filteredSpanIds.set([]);
    traceState.reset();
    vi.mocked(filterSpans).mockClear();
  });

  it('shows the headline stats for the loaded trace', () => {
    loadTrace();
    render(FilterBar, { props: { onOpenBudgets: () => {} } });
    expect(screen.getByText('OTLP JSON')).toBeInTheDocument();
    expect(screen.getByText(/7 of 7 spans/)).toBeInTheDocument();
    // total duration and p95 both read 4.23s on this trace
    expect(screen.getAllByText(/4\.23s/).length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /llm/i })).toBeInTheDocument();
  });

  it('tracks the readout against the active filter and flags the empty case', async () => {
    loadTrace();
    const { container } = render(FilterBar, { props: { onOpenBudgets: () => {} } });

    filterKind.set('producer');
    filteredSpanIds.set([]);
    await vi.waitFor(() => {
      expect(screen.getByText(/0 of 7 spans/)).toBeInTheDocument();
    });
    const readout = container.querySelector('[class*="filter-count"]');
    expect(readout?.className).toContain('empty');
  });

  it('asks the core to filter whenever a facet changes', async () => {
    loadTrace();
    render(FilterBar, { props: { onOpenBudgets: () => {} } });
    filterStatus.set('error');
    await vi.waitFor(() => {
      expect(filterSpans).toHaveBeenCalledWith(expect.objectContaining({ status: 'error' }));
    });
  });

  it('offers the LLM-only toggle and reflects its state', async () => {
    loadTrace();
    render(FilterBar, { props: { onOpenBudgets: () => {} } });
    const toggle = screen.getByRole('button', { name: /llm/i });
    toggle.click();
    await vi.waitFor(() => expect(get(filterLlmOnly)).toBe(true));
  });

  it('renders nothing while no trace is loaded', () => {
    const { container } = render(FilterBar, { props: { onOpenBudgets: () => {} } });
    expect(container.textContent?.trim()).toBe('');
  });
});
