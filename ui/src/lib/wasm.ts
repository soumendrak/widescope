import init, * as widescopeCore from '../../../crates/widescope-core/pkg/widescope_core';
import { BUNDLED_CONVENTIONS } from './conventions-bundle';
import type {
  ComparisonSummary,
  FlameGraphLayout,
  InitResult,
  ParseWarning,
  ServiceGraph,
  SpanDetail,
  TimelineLayout,
  TraceSummary,
  WasmError,
  WaterfallLayout,
} from './types';

const {
  init: wasmInit,
  parse_trace,
  compute_flamegraph,
  compute_timeline,
  compute_waterfall,
  get_span_detail,
} = widescopeCore;

let initWarnings: ParseWarning[] = [];

export async function loadWasm(): Promise<void> {
  await init();
  const merged = '[' + BUNDLED_CONVENTIONS.join(',') + ']';
  const result: InitResult = JSON.parse(wasmInit(merged));
  initWarnings = result.warnings;
}

export function getInitWarnings(): ParseWarning[] {
  return initWarnings;
}

export function parseTrace(raw: string): TraceSummary {
  return JSON.parse(parse_trace(raw)) as TraceSummary;
}

export function getFlameGraphLayout(): FlameGraphLayout {
  return JSON.parse(compute_flamegraph()) as FlameGraphLayout;
}

export function getTimelineLayout(): TimelineLayout {
  return JSON.parse(compute_timeline()) as TimelineLayout;
}

export function getWaterfallLayout(): WaterfallLayout {
  return JSON.parse(compute_waterfall()) as WaterfallLayout;
}

export function getSpanDetail(spanId: string): SpanDetail {
  return JSON.parse(get_span_detail(spanId)) as SpanDetail;
}

export function searchSpans(query: string): string[] {
  const search = (widescopeCore as { search_spans?: (value: string) => string }).search_spans;
  if (!search) {
    return [];
  }
  return JSON.parse(search(query)) as string[];
}

export interface SpanFilters {
  status?: string;
  service?: string;
  kind?: string;
  llm_only?: boolean;
}

export function filterSpans(filters: SpanFilters): string[] {
  const filterFn = (widescopeCore as { filter_spans?: (value: string) => string }).filter_spans;
  if (!filterFn) {
    return [];
  }
  return JSON.parse(filterFn(JSON.stringify(filters))) as string[];
}

export function getServiceGraph(): ServiceGraph {
  const fn = (widescopeCore as { get_service_graph?: () => string }).get_service_graph;
  if (!fn) return { nodes: [], edges: [] };
  return JSON.parse(fn()) as ServiceGraph;
}

export function parseComparisonTrace(raw: string): ComparisonSummary {
  const fn = (widescopeCore as { parse_comparison_trace?: (v: string) => string }).parse_comparison_trace;
  if (!fn) throw new Error('parse_comparison_trace not available');
  return JSON.parse(fn(raw)) as ComparisonSummary;
}

export function getComparisonFlamegraph(): FlameGraphLayout {
  const fn = (widescopeCore as { get_comparison_flamegraph?: () => string }).get_comparison_flamegraph;
  if (!fn) throw new Error('get_comparison_flamegraph not available');
  return JSON.parse(fn()) as FlameGraphLayout;
}

export function clearComparison(): void {
  const fn = (widescopeCore as { clear_comparison?: () => void }).clear_comparison;
  fn?.();
}

export function safeParseWasmError(err: unknown): WasmError {
  let raw: string | undefined;
  if (typeof err === 'string') raw = err;
  else if (err instanceof Error) raw = err.message;

  if (raw) {
    try {
      return JSON.parse(raw) as WasmError;
    } catch {
      // fall through
    }
  }

  return {
    error_type: 'Unknown',
    code: 'INTERNAL_ERROR',
    message: String(err),
    context: null,
  };
}
