import init, * as widescopeCore from '../../../crates/widescope-core/pkg/widescope_core';
import { BUNDLED_CONVENTIONS, BUNDLED_PRICING } from './conventions-bundle';
import type {
  AgentFlow,
  ComparisonMatrix,
  ComparisonSummary,
  CostBreakdown,
  CriticalPath,
  FlameGraphLayout,
  InitResult,
  ParseWarning,
  ServiceGraph,
  SessionGroups,
  SpanDetail,
  TimelineLayout,
  TokenTrends,
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

  const initPricing = (widescopeCore as { init_pricing?: (json: string) => string }).init_pricing;
  if (initPricing) {
    try {
      const pricingResult = JSON.parse(initPricing(BUNDLED_PRICING)) as { warnings?: ParseWarning[] };
      if (pricingResult.warnings && pricingResult.warnings.length > 0) {
        initWarnings = initWarnings.concat(pricingResult.warnings);
      }
    } catch (e) {
      // Pricing is optional — a load failure should not break trace parsing.
      console.warn('Failed to initialise pricing table:', e);
    }
  }
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
  safety_only?: boolean;
}

export function filterSpans(filters: SpanFilters): string[] {
  const filterFn = (widescopeCore as { filter_spans?: (value: string) => string }).filter_spans;
  if (!filterFn) {
    return [];
  }
  return JSON.parse(filterFn(JSON.stringify(filters))) as string[];
}

export function getAgentFlow(): AgentFlow {
  const fn = (widescopeCore as { compute_agent_flow_layout?: () => string }).compute_agent_flow_layout;
  if (!fn) return { nodes: [], edges: [] };
  return JSON.parse(fn()) as AgentFlow;
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

export function computeComparisonMatrix(entries: { name: string; json: string }[]): ComparisonMatrix {
  const fn = (widescopeCore as { compute_comparison_matrix?: (v: string) => string }).compute_comparison_matrix;
  if (!fn) throw new Error('compute_comparison_matrix not available');
  return JSON.parse(fn(JSON.stringify(entries))) as ComparisonMatrix;
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

export function getCriticalPath(): CriticalPath | null {
  const fn = (widescopeCore as { get_critical_path?: () => string }).get_critical_path;
  if (!fn) return null;
  return JSON.parse(fn()) as CriticalPath;
}

export function getCostBreakdown(): CostBreakdown | null {
  const fn = (widescopeCore as { get_cost_breakdown?: () => string }).get_cost_breakdown;
  if (!fn) return null;
  return JSON.parse(fn()) as CostBreakdown;
}

/** Aggregate token usage across the raw trace payloads the UI holds in its list. */
export function computeTokenTrends(
  traces: { name: string; json: string }[],
): TokenTrends | null {
  const fn = (widescopeCore as { compute_token_trends?: (s: string) => string })
    .compute_token_trends;
  if (!fn) return null;
  return JSON.parse(fn(JSON.stringify(traces))) as TokenTrends;
}

/** Group the UI's raw trace payloads by session id with aggregated metrics. */
export function computeSessionGroups(
  traces: { name: string; json: string }[],
): SessionGroups | null {
  const fn = (widescopeCore as { compute_session_groups?: (s: string) => string })
    .compute_session_groups;
  if (!fn) return null;
  return JSON.parse(fn(JSON.stringify(traces))) as SessionGroups;
}

/** Whether the WASM share-link compressor is available (i.e. WASM is loaded). */
export function isShareCompressionReady(): boolean {
  return (
    typeof (widescopeCore as { compress_share?: unknown }).compress_share === 'function'
  );
}

/** Compress trace JSON into a tagged, self-contained share blob. */
export function compressShare(json: string): Uint8Array {
  const fn = (widescopeCore as { compress_share?: (v: string) => Uint8Array })
    .compress_share;
  if (!fn) throw new Error('compress_share not available');
  return fn(json);
}

/** Decode a share blob produced by {@link compressShare} back into trace JSON. */
export function decompressShare(blob: Uint8Array): string {
  const fn = (widescopeCore as { decompress_share?: (v: Uint8Array) => string })
    .decompress_share;
  if (!fn) throw new Error('decompress_share not available');
  return fn(blob);
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
