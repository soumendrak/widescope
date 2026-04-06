export interface ParseWarning {
  code: string;
  message: string;
  count: number;
  context: Record<string, string | number> | null;
}

export interface WasmError {
  error_type: string;
  code: string;
  message: string;
  context: Record<string, unknown> | null;
}

export interface InitResult {
  conventions_loaded: number;
  warnings: ParseWarning[];
}

export interface TraceSummary {
  trace_id: string;
  span_count: number;
  service_count: number;
  detected_format: 'OtlpJson' | 'JaegerJson' | 'OpenInferenceJson';
  has_errors: boolean;
  total_duration_ns: number;
  total_duration_display: string;
  root_operation: string | null;
  root_service: string | null;
  warnings: ParseWarning[];
}

export interface FlameGraphLayout {
  nodes: FlameNode[];
  max_depth: number;
  trace_duration_ns: number;
  trace_duration_display: string;
}

export interface FlameNode {
  span_id: string;
  label: string;
  x: number;
  width: number;
  depth: number;
  color_key: string;
  is_error: boolean;
  is_llm: boolean;
  duration_ns: number;
  self_time_ns: number;
  duration_display: string;
  self_time_display: string;
}

export interface TimelineLayout {
  blocks: TimelineBlock[];
  rows: TimelineRow[];
  trace_duration_ns: number;
  trace_duration_display: string;
}

export interface TimelineBlock {
  span_id: string;
  label: string;
  service_name: string;
  x_start: number;
  x_end: number;
  row_index: number;
  is_error: boolean;
  is_llm: boolean;
  duration_ns: number;
  duration_display: string;
}

export interface TimelineRow {
  service_name: string;
  row_index: number;
  lane_index: number;
}

export interface SpanDetail {
  span_id: string;
  trace_id: string;
  parent_span_id: string | null;
  operation_name: string;
  service_name: string;
  span_kind: 'Internal' | 'Server' | 'Client' | 'Producer' | 'Consumer';
  start_time_ns: string;
  start_time_display: string;
  duration_ns: number;
  duration_display: string;
  self_time_ns: number;
  self_time_display: string;
  status: 'Unset' | 'Ok' | 'Error';
  error_message: string | null;
  attributes: [string, string][];
  events: EventDetail[];
  llm: LlmDetail | null;
  children_ids: string[];
}

export interface EventDetail {
  name: string;
  timestamp_ns: string;
  timestamp_display: string;
  attributes: [string, string][];
}

export interface WaterfallRow {
  span_id: string;
  operation_name: string;
  service_name: string;
  span_kind: 'Internal' | 'Server' | 'Client' | 'Producer' | 'Consumer';
  depth: number;
  x_start: number;
  x_end: number;
  color_key: string;
  is_error: boolean;
  is_llm: boolean;
  has_children: boolean;
  duration_ns: number;
  duration_display: string;
  self_time_ns: number;
  self_time_display: string;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  estimated_cost_usd: number | null;
}

export interface WaterfallLayout {
  rows: WaterfallRow[];
  trace_duration_ns: number;
  trace_duration_display: string;
  max_depth: number;
}

export interface LlmDetail {
  operation_type: string;
  model_name: string | null;
  model_provider: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  estimated_cost_usd: number | null;
  temperature: number | null;
  input_messages: { role: string; content: string | null }[];
  output_messages: { role: string; content: string | null }[];
  tool_calls: { name: string; arguments: string | null; result: string | null }[];
}
