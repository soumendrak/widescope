// `WideError::X.into()` converts to JsValue on wasm but is identity natively,
// where `ApiError = WideError` — so clippy's useless_conversion only fires off-wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(clippy::useless_conversion))]

mod conventions;
mod errors;
mod layout;
mod models;
mod parsers;
mod share;
mod trace_builder;
mod utils;

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Error carried out of the bindings: a JS value in the browser, a plain
// `WideError` natively (the CLI). `JsValue` can't be constructed off-wasm —
// `JsValue::from_str` aborts — so native error paths must avoid it.
#[cfg(target_arch = "wasm32")]
type ApiError = JsValue;
#[cfg(not(target_arch = "wasm32"))]
type ApiError = errors::WideError;

use conventions::pricing::PricingTable;
use conventions::registry::{load_conventions, Convention};
use errors::WideError;
use layout::agent_flow::compute_agent_flow;
use layout::critical_path::compute_critical_path;
use layout::flamegraph::compute_flamegraph_layout;
use layout::graph::compute_service_graph as build_service_graph;
use layout::timeline::compute_timeline_layout;
use layout::waterfall::compute_waterfall_layout;
use models::layout::{
    EvalScoreDetail, EventDetail, LlmDetail, MessageDetail, RetrievedDocumentDetail,
    SpanDetailResponse, ToolCallDetail,
};
use models::llm::LlmSpanAttributes;
use models::span::{AttributeValue, Span, SpanEvent};
use models::trace::{ParseWarning, Trace};
use parsers::detect_format;
use parsers::jaeger::parse_jaeger_with_warnings;
use parsers::openinference::parse_openinference_with_warnings;
use parsers::otlp_json::parse_otlp_with_warnings;
use trace_builder::build_trace;
use utils::{format_duration, format_timestamp_display};

/// Serialize a response payload, mapping the (practically unreachable) failure
/// into the API error type.
///
/// Nineteen call sites used to inline this closure; one place to get it right
/// is also one place for a test to reach, which is why it exists.
fn to_json<T: Serialize>(value: &T) -> Result<String, ApiError> {
    serde_json::to_string(value).map_err(|e| {
        WideError::InvalidJson {
            message: e.to_string(),
            line: None,
            column: None,
        }
        .into()
    })
}

thread_local! {
    static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
    static TRACE_LIST: RefCell<Vec<(String, Trace)>> = const { RefCell::new(Vec::new()) };
    static ACTIVE_TRACE_INDEX: RefCell<usize> = const { RefCell::new(0) };
    static COMPARISON_TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
    static CONVENTIONS: RefCell<Vec<Convention>> = const { RefCell::new(Vec::new()) };
    static PRICING: RefCell<PricingTable> = RefCell::new(PricingTable::new());
}

#[cfg(feature = "console_error_panic_hook")]
fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(not(feature = "console_error_panic_hook"))]
fn set_panic_hook() {}

#[derive(Serialize)]
struct InitResult {
    conventions_loaded: usize,
    warnings: Vec<ParseWarning>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn init(conventions_json: &str) -> Result<String, ApiError> {
    set_panic_hook();

    let result = load_conventions(conventions_json);
    let loaded = result.conventions.len();

    CONVENTIONS.with(|c| {
        *c.borrow_mut() = result.conventions;
    });

    let init_result = InitResult {
        conventions_loaded: loaded,
        warnings: result.warnings,
    };

    to_json(&init_result)
}

#[derive(Serialize)]
struct InitPricingResult {
    models_loaded: usize,
    warnings: Vec<ParseWarning>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn init_pricing(pricing_json: &str) -> Result<String, ApiError> {
    let mut table = PricingTable::new();
    let mut warnings = Vec::new();
    let loaded = match table.load(pricing_json) {
        Ok(n) => n,
        Err(w) => {
            warnings.push(w);
            0
        }
    };

    PRICING.with(|p| {
        *p.borrow_mut() = table;
    });

    let result = InitPricingResult {
        models_loaded: loaded,
        warnings,
    };
    to_json(&result)
}

#[derive(Serialize)]
struct TraceSummary {
    trace_id: String,
    span_count: usize,
    service_count: usize,
    detected_format: String,
    has_errors: bool,
    error_count: usize,
    llm_span_count: usize,
    total_duration_ns: u64,
    total_duration_display: String,
    latency_p50_display: String,
    latency_p95_display: String,
    root_operation: Option<String>,
    root_service: Option<String>,
    warnings: Vec<ParseWarning>,
}

/// Parse raw trace input into a fully-resolved [`Trace`] (LLM attributes
/// resolved, costs priced). Shared by [`parse_trace`] and the cross-trace
/// analytics in [`compute_token_trends`].
fn parse_input_to_trace(raw_input: &str) -> Result<Trace, WideError> {
    let value: serde_json::Value =
        serde_json::from_str(raw_input).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let format = detect_format(&value)?;

    let (mut spans, parse_warnings) = match &format {
        models::trace::InputFormat::OtlpJson => {
            let result = parse_otlp_with_warnings(&value)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::JaegerJson => {
            let result = parse_jaeger_with_warnings(&value)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::OpenInferenceJson => {
            let result = parse_openinference_with_warnings(&value)?;
            (result.spans, result.warnings)
        }
        _ => {
            return Err(WideError::UnrecognizedFormat);
        }
    };

    let conventions = CONVENTIONS.with(|c| c.borrow().clone());

    for span in &mut spans {
        span.llm = conventions::resolver::resolve_llm_attributes(span, &conventions);
        span.safety = conventions::safety::detect_safety_signals(span);
    }

    PRICING.with(|p| {
        let table = p.borrow();
        if table.is_empty() {
            return;
        }
        for span in &mut spans {
            if let Some(llm) = span.llm.as_mut() {
                if llm.estimated_cost_usd.is_some() {
                    continue;
                }
                let Some(model) = llm.model_name.as_deref() else {
                    continue;
                };
                let input = llm.input_tokens.unwrap_or(0);
                let output = llm.output_tokens.unwrap_or(0);
                if input == 0 && output == 0 {
                    continue;
                }
                if let Some(cost) =
                    table.compute_cost(model, llm.model_provider.as_deref(), input, output)
                {
                    llm.estimated_cost_usd = Some(cost);
                }
            }
        }
    });

    build_trace(spans, format, parse_warnings)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn parse_trace(raw_input: &str) -> Result<String, ApiError> {
    let trace = parse_input_to_trace(raw_input).map_err(ApiError::from)?;

    let error_count = trace.spans.iter().filter(|s| s.status.is_error()).count();
    let llm_span_count = trace.spans.iter().filter(|s| s.llm.is_some()).count();

    let mut durations: Vec<u64> = trace.spans.iter().map(|s| s.duration_ns).collect();
    durations.sort_unstable();
    let p50 = percentile(&durations, 0.50);
    let p95 = percentile(&durations, 0.95);

    let root_op = trace
        .root_span_ids
        .first()
        .and_then(|id| trace.get_span(id).map(|s| s.operation_name.clone()));
    let root_svc = trace
        .root_span_ids
        .first()
        .and_then(|id| trace.get_span(id).map(|s| s.service_name.clone()));

    let summary = TraceSummary {
        trace_id: trace.trace_id.clone(),
        span_count: trace.span_count,
        service_count: trace.service_count,
        detected_format: trace.detected_format.as_str().to_string(),
        has_errors: trace.has_errors,
        error_count,
        llm_span_count,
        total_duration_ns: trace.total_duration_ns,
        total_duration_display: format_duration(trace.total_duration_ns),
        latency_p50_display: format_duration(p50),
        latency_p95_display: format_duration(p95),
        root_operation: root_op,
        root_service: root_svc,
        warnings: trace.warnings.clone(),
    };

    TRACE.with(|t| {
        *t.borrow_mut() = Some(trace);
    });

    to_json(&summary)
}

#[derive(Serialize)]
struct MatrixTrace {
    name: String,
    trace_id: String,
}

#[derive(Serialize)]
struct MetricRow {
    label: String,
    /// Some(true) = lower is better, Some(false) = higher is better, None = neutral.
    lower_is_better: Option<bool>,
    values: Vec<f64>,
    display: Vec<String>,
}

#[derive(Serialize)]
struct ComparisonMatrix {
    traces: Vec<MatrixTrace>,
    rows: Vec<MetricRow>,
}

#[derive(Deserialize)]
struct MatrixInput {
    name: String,
    json: String,
}

/// Build a side-by-side metrics matrix for N loaded traces.
/// Input: JSON array of `{name, json}`. Columns = traces, rows = metrics.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_comparison_matrix(raw_input: &str) -> Result<String, ApiError> {
    let inputs: Vec<MatrixInput> =
        serde_json::from_str(raw_input).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let mut traces = Vec::with_capacity(inputs.len());
    // Per-metric value columns, indexed by trace.
    let (mut duration, mut spans, mut errors, mut tokens, mut cost, mut p50, mut p95) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    let (mut d_disp, mut s_disp, mut e_disp, mut t_disp, mut c_disp, mut p50_disp, mut p95_disp) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);

    for input in &inputs {
        let trace = parse_input_to_trace(&input.json).map_err(ApiError::from)?;

        let error_count = trace.spans.iter().filter(|s| s.status.is_error()).count();
        let token_count: u64 = trace
            .spans
            .iter()
            .filter_map(|s| s.llm.as_ref())
            .map(|llm| {
                llm.total_tokens
                    .unwrap_or_else(|| llm.input_tokens.unwrap_or(0) + llm.output_tokens.unwrap_or(0))
            })
            .sum();
        let total_cost: f64 = trace
            .spans
            .iter()
            .filter_map(|s| s.llm.as_ref())
            .filter_map(|llm| llm.estimated_cost_usd)
            .sum();

        let mut durations: Vec<u64> = trace.spans.iter().map(|s| s.duration_ns).collect();
        durations.sort_unstable();
        let lp50 = percentile(&durations, 0.50);
        let lp95 = percentile(&durations, 0.95);

        traces.push(MatrixTrace {
            name: input.name.clone(),
            trace_id: trace.trace_id.clone(),
        });

        duration.push(trace.total_duration_ns as f64);
        d_disp.push(format_duration(trace.total_duration_ns));
        spans.push(trace.span_count as f64);
        s_disp.push(trace.span_count.to_string());
        errors.push(error_count as f64);
        e_disp.push(error_count.to_string());
        tokens.push(token_count as f64);
        t_disp.push(token_count.to_string());
        cost.push(total_cost);
        c_disp.push(format!("${total_cost:.4}"));
        p50.push(lp50 as f64);
        p50_disp.push(format_duration(lp50));
        p95.push(lp95 as f64);
        p95_disp.push(format_duration(lp95));
    }

    let rows = vec![
        MetricRow { label: "Total duration".into(), lower_is_better: Some(true), values: duration, display: d_disp },
        MetricRow { label: "Span count".into(), lower_is_better: None, values: spans, display: s_disp },
        MetricRow { label: "Error count".into(), lower_is_better: Some(true), values: errors, display: e_disp },
        MetricRow { label: "Token count".into(), lower_is_better: None, values: tokens, display: t_disp },
        MetricRow { label: "Cost (USD)".into(), lower_is_better: Some(true), values: cost, display: c_disp },
        MetricRow { label: "P50 latency".into(), lower_is_better: Some(true), values: p50, display: p50_disp },
        MetricRow { label: "P95 latency".into(), lower_is_better: Some(true), values: p95, display: p95_disp },
    ];

    let matrix = ComparisonMatrix { traces, rows };
    to_json(&matrix)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_flamegraph() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_flamegraph_layout(trace);
                to_json(&layout)
            }
        }
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_timeline() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_timeline_layout(trace);
                to_json(&layout)
            }
        }
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn get_span_detail(span_id: &str) -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let span = trace
                    .get_span(span_id)
                    .ok_or_else(|| WideError::SpanNotFound {
                        span_id: span_id.to_string(),
                    })?;

                let children_ids = trace.get_children(span_id).to_vec();

                let mut attributes: Vec<(String, String)> = span
                    .attributes
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_display_string()))
                    .collect();
                attributes.sort_by(|a, b| a.0.cmp(&b.0));

                let events: Vec<EventDetail> = span.events.iter().map(build_event_detail).collect();

                let llm = span.llm.as_ref().map(build_llm_detail);

                let detail = SpanDetailResponse {
                    span_id: span.span_id.clone(),
                    trace_id: span.trace_id.clone(),
                    parent_span_id: span.parent_span_id.clone(),
                    operation_name: span.operation_name.clone(),
                    service_name: span.service_name.clone(),
                    span_kind: span.span_kind.as_str().to_string(),
                    start_time_ns: span.start_time_ns,
                    start_time_display: format_timestamp_display(span.start_time_ns),
                    duration_ns: span.duration_ns,
                    duration_display: format_duration(span.duration_ns),
                    self_time_ns: span.self_time_ns,
                    self_time_display: format_duration(span.self_time_ns),
                    status: span.status.as_str().to_string(),
                    error_message: span.status.error_message().map(|s| s.to_string()),
                    attributes,
                    events,
                    llm,
                    safety: span.safety.clone(),
                    children_ids,
                };

                to_json(&detail)
            }
        }
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn search_spans(query: &str) -> Result<String, ApiError> {
    let normalized_query = query.trim().to_ascii_lowercase();

    if normalized_query.is_empty() {
        return Ok("[]".to_string());
    }

    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let mut matches: Vec<(String, u64)> = trace
                    .spans
                    .iter()
                    .filter(|span| span_matches_query(span, &normalized_query))
                    .map(|span| (span.span_id.clone(), span.start_time_ns))
                    .collect();

                matches.sort_by_key(|(_, start_time_ns)| *start_time_ns);

                let span_ids: Vec<String> =
                    matches.into_iter().map(|(span_id, _)| span_id).collect();

                to_json(&span_ids)
            }
        }
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_waterfall() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_waterfall_layout(trace);
                to_json(&layout)
            }
        }
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_agent_flow_layout() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let flow = compute_agent_flow(trace);
                to_json(&flow)
            }
        }
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn get_service_graph() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let graph = build_service_graph(trace);
                to_json(&graph)
            }
        }
    })
}

fn build_event_detail(e: &SpanEvent) -> EventDetail {
    let mut attrs: Vec<(String, String)> = e
        .attributes
        .iter()
        .map(|(k, v)| (k.clone(), v.as_display_string()))
        .collect();
    attrs.sort_by(|a, b| a.0.cmp(&b.0));

    EventDetail {
        name: e.name.clone(),
        timestamp_ns: e.timestamp_ns,
        timestamp_display: format_timestamp_display(e.timestamp_ns),
        attributes: attrs,
    }
}

fn build_llm_detail(llm: &LlmSpanAttributes) -> LlmDetail {
    LlmDetail {
        operation_type: llm.operation_type.as_str(),
        model_name: llm.model_name.clone(),
        model_provider: llm.model_provider.clone(),
        input_tokens: llm.input_tokens,
        output_tokens: llm.output_tokens,
        total_tokens: llm.total_tokens,
        estimated_cost_usd: llm.estimated_cost_usd,
        temperature: llm.temperature,
        input_messages: llm
            .input_messages
            .iter()
            .map(|m| MessageDetail {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        output_messages: llm
            .output_messages
            .iter()
            .map(|m| MessageDetail {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        tool_calls: llm
            .tool_calls
            .iter()
            .map(|tc| ToolCallDetail {
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
                result: tc.result.clone(),
            })
            .collect(),
        retrieved_documents: llm
            .retrieved_documents
            .iter()
            .map(|d| RetrievedDocumentDetail {
                id: d.id.clone(),
                score: d.score,
                content_snippet: d.content_snippet.clone(),
            })
            .collect(),
        eval_scores: llm
            .eval_scores
            .iter()
            .map(|e| EvalScoreDetail {
                name: e.name.clone(),
                value: e.value,
                threshold: e.threshold,
                passed: e.passed,
            })
            .collect(),
    }
}

fn span_matches_query(span: &Span, query: &str) -> bool {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return true;
    }

    // Try parsing as operators first
    if parse_search_operators(query).is_some() {
        return span_matches_operators(span, query);
    }

    // Fallback to substring search
    if span.operation_name.to_ascii_lowercase().contains(&q)
        || span.service_name.to_ascii_lowercase().contains(&q)
        || span.span_id.to_ascii_lowercase().contains(&q)
    {
        return true;
    }
    span.attributes.iter().any(|(key, value)| {
        key.to_ascii_lowercase().contains(&q) || attr_val_matches_query(value, &q)
    })
}

fn parse_search_operators(query: &str) -> Option<Vec<(String, String, String)>> {
    // Parse tokens like: duration>100ms status=error service~api kind=client llm
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut ops: Vec<(String, String, String)> = Vec::new();
    let mut has_op = false;

    for token in &tokens {
        if token == &"llm" {
            ops.push(("llm".into(), "=".into(), "true".into()));
            has_op = true;
        } else if let Some(gt_pos) = token.find('>') {
            if token.as_bytes().get(gt_pos + 1) == Some(&b'=') {
                let key = token[..gt_pos].to_ascii_lowercase();
                let val = token[gt_pos + 2..].to_ascii_lowercase();
                ops.push((key, ">=".into(), val));
            } else {
                let key = token[..gt_pos].to_ascii_lowercase();
                let val = token[gt_pos + 1..].to_ascii_lowercase();
                ops.push((key, ">".into(), val));
            }
            has_op = true;
        } else if let Some(lt_pos) = token.find('<') {
            if token.as_bytes().get(lt_pos + 1) == Some(&b'=') {
                let key = token[..lt_pos].to_ascii_lowercase();
                let val = token[lt_pos + 2..].to_ascii_lowercase();
                ops.push((key, "<=".into(), val));
            } else {
                let key = token[..lt_pos].to_ascii_lowercase();
                let val = token[lt_pos + 1..].to_ascii_lowercase();
                ops.push((key, "<".into(), val));
            }
            has_op = true;
        } else if let Some(eq_pos) = token.find('=') {
            let key = token[..eq_pos].to_ascii_lowercase();
            let val = token[eq_pos + 1..].to_ascii_lowercase();
            ops.push((key, "=".into(), val));
            has_op = true;
        } else if token.contains('~') {
            let parts: Vec<&str> = token.splitn(2, '~').collect();
            if parts.len() == 2 {
                ops.push((
                    parts[0].to_ascii_lowercase(),
                    "~".into(),
                    parts[1].to_ascii_lowercase(),
                ));
                has_op = true;
            }
        }
    }

    if has_op {
        Some(ops)
    } else {
        None
    }
}

fn parse_duration_query(val: &str) -> Option<u64> {
    let val = val.trim();
    if let Some(stripped) = val.strip_suffix("ms") {
        stripped
            .parse::<f64>()
            .ok()
            .map(|v| (v * 1_000_000.0) as u64)
    } else if let Some(stripped) = val.strip_suffix("µs") {
        stripped.parse::<f64>().ok().map(|v| (v * 1_000.0) as u64)
    } else if let Some(stripped) = val.strip_suffix("us") {
        stripped.parse::<f64>().ok().map(|v| (v * 1_000.0) as u64)
    } else if let Some(stripped) = val.strip_suffix('s') {
        stripped
            .parse::<f64>()
            .ok()
            .map(|v| (v * 1_000_000_000.0) as u64)
    } else {
        val.parse::<u64>().ok()
    }
}

fn span_matches_operators(span: &Span, query: &str) -> bool {
    let ops = match parse_search_operators(query) {
        Some(ops) => ops,
        None => return true,
    };

    for (key, op, val) in &ops {
        if key == "llm" {
            if span.llm.is_none() {
                return false;
            }
            continue;
        }
        match key.as_str() {
            "duration" | "dur" => {
                let target = match parse_duration_query(val) {
                    Some(t) => t,
                    None => return false,
                };
                match op.as_str() {
                    ">" if span.duration_ns <= target => {
                        return false;
                    }
                    ">=" if span.duration_ns < target => {
                        return false;
                    }
                    "<" if span.duration_ns >= target => {
                        return false;
                    }
                    "<=" if span.duration_ns > target => {
                        return false;
                    }
                    "=" if span.duration_ns != target => {
                        return false;
                    }
                    _ => {}
                }
            }
            "status" => {
                if span.status.as_str().to_ascii_lowercase() != *val {
                    return false;
                }
            }
            "kind" => {
                if span.span_kind.as_str().to_ascii_lowercase() != *val {
                    return false;
                }
            }
            "service" | "svc" => {
                if !span.service_name.to_ascii_lowercase().contains(val) {
                    return false;
                }
            }
            _ => {
                // Try matching as attribute key=value
                let attr_match = span.attributes.iter().any(|(attr_key, attr_val)| {
                    attr_key.to_ascii_lowercase().contains(key)
                        && attr_val_matches_query(attr_val, val)
                });
                if !attr_match {
                    return false;
                }
            }
        }
    }
    true
}

fn attr_val_matches_query(value: &AttributeValue, query: &str) -> bool {
    match value {
        AttributeValue::String(text) => text.to_ascii_lowercase().contains(query),
        AttributeValue::StringArray(values) => values
            .iter()
            .any(|text| text.to_ascii_lowercase().contains(query)),
        _ => false,
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

#[derive(Deserialize)]
struct FilterRequest {
    status: Option<String>,
    service: Option<String>,
    kind: Option<String>,
    llm_only: Option<bool>,
    safety_only: Option<bool>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn filter_spans(filter_json: &str) -> Result<String, ApiError> {
    let filter: FilterRequest =
        serde_json::from_str(filter_json).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let status_filter = filter.status.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_ascii_lowercase())
        }
    });
    let service_filter = filter.service.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_ascii_lowercase())
        }
    });
    let kind_filter = filter.kind.and_then(|k| {
        if k.is_empty() {
            None
        } else {
            Some(k.to_ascii_lowercase())
        }
    });
    let llm_only = filter.llm_only.unwrap_or(false);
    let safety_only = filter.safety_only.unwrap_or(false);

    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let mut matches: Vec<String> = trace
                    .spans
                    .iter()
                    .filter(|span| {
                        if let Some(ref st) = status_filter {
                            if span.status.as_str().to_ascii_lowercase() != *st {
                                return false;
                            }
                        }
                        if let Some(ref sv) = service_filter {
                            if !span.service_name.to_ascii_lowercase().contains(sv) {
                                return false;
                            }
                        }
                        if let Some(ref k) = kind_filter {
                            if span.span_kind.as_str().to_ascii_lowercase() != *k {
                                return false;
                            }
                        }
                        if llm_only && span.llm.is_none() {
                            return false;
                        }
                        if safety_only && span.safety.is_empty() {
                            return false;
                        }
                        true
                    })
                    .map(|span| span.span_id.clone())
                    .collect();

                matches.sort();
                to_json(&matches)
            }
        }
    })
}

#[derive(Serialize)]
struct ComparisonSummary {
    span_count: usize,
    service_count: usize,
    total_duration_ns: u64,
    total_duration_display: String,
    has_errors: bool,
    error_count: usize,
    llm_span_count: usize,
    trace_id: String,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn parse_comparison_trace(raw_input: &str) -> Result<String, ApiError> {
    let value: serde_json::Value =
        serde_json::from_str(raw_input).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let format = detect_format(&value).map_err(ApiError::from)?;

    let (mut spans, _parse_warnings) = match &format {
        models::trace::InputFormat::OtlpJson => {
            let result = parse_otlp_with_warnings(&value).map_err(ApiError::from)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::JaegerJson => {
            let result = parse_jaeger_with_warnings(&value).map_err(ApiError::from)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::OpenInferenceJson => {
            let result = parse_openinference_with_warnings(&value).map_err(ApiError::from)?;
            (result.spans, result.warnings)
        }
        _ => return Err(WideError::UnrecognizedFormat.into()),
    };

    let conventions = CONVENTIONS.with(|c| c.borrow().clone());
    for span in &mut spans {
        span.llm = conventions::resolver::resolve_llm_attributes(span, &conventions);
        span.safety = conventions::safety::detect_safety_signals(span);
    }

    let trace = build_trace(spans, format, vec![]).map_err(ApiError::from)?;

    let error_count = trace.spans.iter().filter(|s| s.status.is_error()).count();
    let llm_span_count = trace.spans.iter().filter(|s| s.llm.is_some()).count();

    let summary = ComparisonSummary {
        trace_id: trace.trace_id.clone(),
        span_count: trace.span_count,
        service_count: trace.service_count,
        total_duration_ns: trace.total_duration_ns,
        total_duration_display: format_duration(trace.total_duration_ns),
        has_errors: trace.has_errors,
        error_count,
        llm_span_count,
    };

    COMPARISON_TRACE.with(|t| {
        *t.borrow_mut() = Some(trace);
    });

    to_json(&summary)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn get_comparison_flamegraph() -> Result<String, ApiError> {
    COMPARISON_TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_flamegraph_layout(trace);
                to_json(&layout)
            }
        }
    })
}

/// Compress trace JSON into a self-contained share blob (see [`share`]).
///
/// Returns `[format tag] + deflate(json)` bytes; the UI base64url-encodes them
/// into the URL `#fragment`.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compress_share(json: &str) -> Vec<u8> {
    share::compress_share(json)
}

/// Decode a share blob produced by [`compress_share`] back into trace JSON.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn decompress_share(blob: &[u8]) -> Result<String, ApiError> {
    share::decompress_share(blob).map_err(ApiError::from)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn clear_comparison() {
    COMPARISON_TRACE.with(|t| {
        *t.borrow_mut() = None;
    });
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn get_critical_path() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let cp = compute_critical_path(trace);
                to_json(&cp)
            }
        }
    })
}

#[derive(Serialize)]
struct CostEntry {
    model: String,
    provider: String,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    estimated_cost_usd: f64,
    spans: Vec<String>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn get_cost_breakdown() -> Result<String, ApiError> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let mut models: std::collections::HashMap<String, CostEntry> =
                    std::collections::HashMap::new();

                for span in &trace.spans {
                    if let Some(ref llm) = span.llm {
                        let key = format!(
                            "{}::{}",
                            llm.model_provider.as_deref().unwrap_or("unknown"),
                            llm.model_name.as_deref().unwrap_or("unknown")
                        );
                        let entry = models.entry(key).or_insert_with(|| CostEntry {
                            model: llm.model_name.clone().unwrap_or_else(|| "unknown".into()),
                            provider: llm
                                .model_provider
                                .clone()
                                .unwrap_or_else(|| "unknown".into()),
                            input_tokens: 0,
                            output_tokens: 0,
                            total_tokens: 0,
                            estimated_cost_usd: 0.0,
                            spans: vec![],
                        });
                        entry.input_tokens += llm.input_tokens.unwrap_or(0);
                        entry.output_tokens += llm.output_tokens.unwrap_or(0);
                        entry.total_tokens += llm.total_tokens.unwrap_or(0);
                        entry.estimated_cost_usd += llm.estimated_cost_usd.unwrap_or(0.0);
                        entry.spans.push(span.span_id.clone());
                    }
                }

                let mut entries: Vec<&CostEntry> = models.values().collect();
                entries.sort_by(|a, b| {
                    b.estimated_cost_usd
                        .partial_cmp(&a.estimated_cost_usd)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let total_cost: f64 = entries.iter().map(|e| e.estimated_cost_usd).sum();

                let owned_entries: Vec<CostEntry> = entries
                    .into_iter()
                    .map(|e| CostEntry {
                        model: e.model.clone(),
                        provider: e.provider.clone(),
                        input_tokens: e.input_tokens,
                        output_tokens: e.output_tokens,
                        total_tokens: e.total_tokens,
                        estimated_cost_usd: e.estimated_cost_usd,
                        spans: e.spans.clone(),
                    })
                    .collect();

                #[derive(Serialize)]
                struct CostBreakdown {
                    entries: Vec<CostEntry>,
                    total_cost_usd: f64,
                    total_input_tokens: u64,
                    total_output_tokens: u64,
                }

                let breakdown = CostBreakdown {
                    total_cost_usd: total_cost,
                    total_input_tokens: owned_entries.iter().map(|e| e.input_tokens).sum(),
                    total_output_tokens: owned_entries.iter().map(|e| e.output_tokens).sum(),
                    entries: owned_entries,
                };

                to_json(&breakdown)
            }
        }
    })
}

#[derive(Serialize, Default, Clone)]
struct TokenGroup {
    name: String,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    cost_usd: f64,
    span_count: u64,
}

impl TokenGroup {
    fn add(&mut self, llm: &models::llm::LlmSpanAttributes) {
        self.input_tokens += llm.input_tokens.unwrap_or(0);
        self.output_tokens += llm.output_tokens.unwrap_or(0);
        self.total_tokens += llm.total_tokens.unwrap_or(0);
        self.cost_usd += llm.estimated_cost_usd.unwrap_or(0.0);
        self.span_count += 1;
    }
}

#[derive(Serialize)]
struct TraceTokens {
    name: String,
    trace_id: String,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    cost_usd: f64,
    llm_span_count: u64,
}

#[derive(Serialize)]
struct TokenTrends {
    per_model: Vec<TokenGroup>,
    per_service: Vec<TokenGroup>,
    per_trace: Vec<TraceTokens>,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
    trace_count: usize,
}

fn sorted_groups(map: std::collections::HashMap<String, TokenGroup>) -> Vec<TokenGroup> {
    let mut v: Vec<TokenGroup> = map.into_values().collect();
    v.sort_by_key(|g| std::cmp::Reverse(g.total_tokens));
    v
}

/// Aggregate LLM token usage across multiple loaded traces.
///
/// `traces_json` is a JSON array of `{ "name": string, "json": string }` —
/// the raw trace payloads the UI keeps in its trace list. Each is parsed and
/// priced with the currently-loaded conventions/pricing, then tokens and cost
/// are summed per model, per service, and per trace.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_token_trends(traces_json: &str) -> Result<String, ApiError> {
    #[derive(Deserialize)]
    struct Entry {
        name: String,
        json: String,
    }

    let entries: Vec<Entry> =
        serde_json::from_str(traces_json).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let mut per_model: std::collections::HashMap<String, TokenGroup> =
        std::collections::HashMap::new();
    let mut per_service: std::collections::HashMap<String, TokenGroup> =
        std::collections::HashMap::new();
    let mut per_trace: Vec<TraceTokens> = Vec::new();

    for entry in &entries {
        // Skip traces that fail to parse rather than failing the whole report.
        let Ok(trace) = parse_input_to_trace(&entry.json) else {
            continue;
        };

        let mut t = TraceTokens {
            name: entry.name.clone(),
            trace_id: trace.trace_id.clone(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_usd: 0.0,
            llm_span_count: 0,
        };

        for span in &trace.spans {
            let Some(ref llm) = span.llm else { continue };

            let model_key = format!(
                "{}::{}",
                llm.model_provider.as_deref().unwrap_or("unknown"),
                llm.model_name.as_deref().unwrap_or("unknown")
            );
            per_model
                .entry(model_key.clone())
                .or_insert_with(|| TokenGroup {
                    name: model_key,
                    ..Default::default()
                })
                .add(llm);

            let svc = span.service_name.clone();
            per_service
                .entry(svc.clone())
                .or_insert_with(|| TokenGroup {
                    name: svc,
                    ..Default::default()
                })
                .add(llm);

            t.input_tokens += llm.input_tokens.unwrap_or(0);
            t.output_tokens += llm.output_tokens.unwrap_or(0);
            t.total_tokens += llm.total_tokens.unwrap_or(0);
            t.cost_usd += llm.estimated_cost_usd.unwrap_or(0.0);
            t.llm_span_count += 1;
        }

        per_trace.push(t);
    }

    let per_model = sorted_groups(per_model);
    let per_service = sorted_groups(per_service);

    let trends = TokenTrends {
        total_input_tokens: per_trace.iter().map(|t| t.input_tokens).sum(),
        total_output_tokens: per_trace.iter().map(|t| t.output_tokens).sum(),
        total_tokens: per_trace.iter().map(|t| t.total_tokens).sum(),
        total_cost_usd: per_trace.iter().map(|t| t.cost_usd).sum(),
        trace_count: per_trace.len(),
        per_model,
        per_service,
        per_trace,
    };

    to_json(&trends)
}

#[derive(Serialize)]
struct DashboardRow {
    name: String,
    trace_id: String,
    detected_format: String,
    span_count: usize,
    service_count: usize,
    error_count: usize,
    llm_span_count: usize,
    total_duration_ns: u64,
    total_duration_display: String,
    latency_p95_display: String,
    cost_usd: f64,
    root_service: Option<String>,
    has_errors: bool,
}

#[derive(Serialize)]
struct ServiceFrequency {
    name: String,
    trace_count: usize,
}

#[derive(Serialize)]
struct Dashboard {
    rows: Vec<DashboardRow>,
    trace_count: usize,
    total_spans: usize,
    total_errors: usize,
    total_llm_spans: usize,
    total_cost_usd: f64,
    avg_duration_ns: u64,
    avg_duration_display: String,
    top_services: Vec<ServiceFrequency>,
}

/// Compute an at-a-glance summary across multiple loaded traces.
///
/// `traces_json` is a JSON array of `{ "name": string, "json": string }` — the
/// same raw payloads the UI keeps in its trace list. Each is parsed and priced
/// with the currently-loaded conventions/pricing into a per-trace row, plus
/// aggregate totals and how many traces each service appears in. Traces that
/// fail to parse are skipped rather than failing the whole dashboard.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_dashboard(traces_json: &str) -> Result<String, ApiError> {
    #[derive(Deserialize)]
    struct Entry {
        name: String,
        json: String,
    }

    let entries: Vec<Entry> =
        serde_json::from_str(traces_json).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let mut rows: Vec<DashboardRow> = Vec::new();
    let mut service_freq: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for entry in &entries {
        let Ok(trace) = parse_input_to_trace(&entry.json) else {
            continue;
        };

        let error_count = trace.spans.iter().filter(|s| s.status.is_error()).count();
        let llm_span_count = trace.spans.iter().filter(|s| s.llm.is_some()).count();
        let cost_usd: f64 = trace
            .spans
            .iter()
            .filter_map(|s| s.llm.as_ref().and_then(|l| l.estimated_cost_usd))
            .sum();

        let mut durations: Vec<u64> = trace.spans.iter().map(|s| s.duration_ns).collect();
        durations.sort_unstable();
        let p95 = percentile(&durations, 0.95);

        let services: std::collections::HashSet<&str> =
            trace.spans.iter().map(|s| s.service_name.as_str()).collect();
        for svc in &services {
            *service_freq.entry((*svc).to_string()).or_insert(0) += 1;
        }

        let root_service = trace
            .root_span_ids
            .first()
            .and_then(|id| trace.get_span(id).map(|s| s.service_name.clone()));

        rows.push(DashboardRow {
            name: entry.name.clone(),
            trace_id: trace.trace_id.clone(),
            detected_format: trace.detected_format.as_str().to_string(),
            span_count: trace.span_count,
            service_count: trace.service_count,
            error_count,
            llm_span_count,
            total_duration_ns: trace.total_duration_ns,
            total_duration_display: format_duration(trace.total_duration_ns),
            latency_p95_display: format_duration(p95),
            cost_usd,
            root_service,
            has_errors: trace.has_errors,
        });
    }

    let trace_count = rows.len();
    let total_duration_ns: u64 = rows.iter().map(|r| r.total_duration_ns).sum();
    let avg_duration_ns = if trace_count > 0 {
        total_duration_ns / trace_count as u64
    } else {
        0
    };

    let mut top_services: Vec<ServiceFrequency> = service_freq
        .into_iter()
        .map(|(name, trace_count)| ServiceFrequency { name, trace_count })
        .collect();
    top_services.sort_by(|a, b| {
        b.trace_count
            .cmp(&a.trace_count)
            .then_with(|| a.name.cmp(&b.name))
    });

    let dashboard = Dashboard {
        total_spans: rows.iter().map(|r| r.span_count).sum(),
        total_errors: rows.iter().map(|r| r.error_count).sum(),
        total_llm_spans: rows.iter().map(|r| r.llm_span_count).sum(),
        total_cost_usd: rows.iter().map(|r| r.cost_usd).sum(),
        avg_duration_ns,
        avg_duration_display: format_duration(avg_duration_ns),
        trace_count,
        rows,
        top_services,
    };

    to_json(&dashboard)
}

/// Attribute keys that carry a session/conversation identifier, in priority
/// order. Covers OTel GenAI semconv (`session.id`, `gen_ai.conversation.id`),
/// LangChain (`session_id`), and common `conversation`/`thread` variants.
const SESSION_KEYS: &[&str] = &[
    "session.id",
    "session_id",
    "gen_ai.conversation.id",
    "conversation.id",
    "conversation_id",
    "thread.id",
    "thread_id",
];

/// Detect a session id for a trace by scanning resource then span attributes
/// for the first known session key with a non-empty string value.
fn detect_session_id(trace: &Trace) -> Option<String> {
    let from = |attrs: &std::collections::HashMap<String, AttributeValue>| {
        SESSION_KEYS.iter().find_map(|k| {
            attrs
                .get(*k)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        })
    };
    trace
        .resources
        .values()
        .find_map(|r| from(&r.attributes))
        .or_else(|| trace.spans.iter().find_map(|s| from(&s.attributes)))
}

#[derive(Serialize)]
struct SessionGroup {
    /// `None` when no session attribute was found (standalone trace).
    session_id: Option<String>,
    /// Indices into the input `traces_json` array (= positions in the UI list).
    trace_indices: Vec<usize>,
    trace_names: Vec<String>,
    trace_count: usize,
    span_count: usize,
    llm_span_count: usize,
    error_count: usize,
    total_cost_usd: f64,
    total_duration_ns: u64,
    total_duration_display: String,
}

/// Group loaded traces by session id and aggregate per-session metrics.
///
/// `traces_json` is the same `[{ "name", "json" }]` payload [`compute_token_trends`]
/// takes. Traces with a shared session attribute are grouped; traces without
/// one each become a standalone group (`session_id: null`). Insertion order is
/// preserved so the UI list stays stable.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compute_session_groups(traces_json: &str) -> Result<String, ApiError> {
    #[derive(Deserialize)]
    struct Entry {
        name: String,
        json: String,
    }

    let entries: Vec<Entry> =
        serde_json::from_str(traces_json).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    // Preserve first-seen order of session keys.
    let mut order: Vec<String> = Vec::new();
    let mut by_key: std::collections::HashMap<String, SessionGroup> =
        std::collections::HashMap::new();
    let mut standalone = 0usize;

    for (idx, entry) in entries.iter().enumerate() {
        // Skip unparseable traces rather than failing the whole grouping.
        let Ok(trace) = parse_input_to_trace(&entry.json) else {
            continue;
        };

        let session_id = detect_session_id(&trace);
        // Standalone traces get a unique key so they never merge together.
        let key = match &session_id {
            Some(id) => format!("s:{id}"),
            None => {
                standalone += 1;
                format!("t:{idx}")
            }
        };

        if !by_key.contains_key(&key) {
            order.push(key.clone());
        }
        let group = by_key.entry(key).or_insert_with(|| SessionGroup {
            session_id: session_id.clone(),
            trace_indices: Vec::new(),
            trace_names: Vec::new(),
            trace_count: 0,
            span_count: 0,
            llm_span_count: 0,
            error_count: 0,
            total_cost_usd: 0.0,
            total_duration_ns: 0,
            total_duration_display: String::new(),
        });

        group.trace_indices.push(idx);
        group.trace_names.push(entry.name.clone());
        group.trace_count += 1;
        group.span_count += trace.span_count;
        group.llm_span_count += trace.spans.iter().filter(|s| s.llm.is_some()).count();
        group.error_count += trace.spans.iter().filter(|s| s.status.is_error()).count();
        group.total_cost_usd += trace
            .spans
            .iter()
            .filter_map(|s| s.llm.as_ref().and_then(|l| l.estimated_cost_usd))
            .sum::<f64>();
        group.total_duration_ns += trace.total_duration_ns;
    }

    let mut groups: Vec<SessionGroup> = order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .map(|mut g| {
            g.total_duration_display = format_duration(g.total_duration_ns);
            g
        })
        .collect();

    #[derive(Serialize)]
    struct SessionGroups {
        groups: Vec<SessionGroup>,
        /// Number of multi-trace sessions — when 0 the UI keeps the flat list.
        session_count: usize,
        standalone_count: usize,
    }

    let session_count = groups
        .iter()
        .filter(|g| g.session_id.is_some() && g.trace_count > 1)
        .count();

    // Sort multi-trace sessions first (by trace count desc), standalone after.
    groups.sort_by(|a, b| {
        let rank = |g: &SessionGroup| g.session_id.is_some() && g.trace_count > 1;
        rank(b)
            .cmp(&rank(a))
            .then(b.trace_count.cmp(&a.trace_count))
    });

    let out = SessionGroups {
        session_count,
        standalone_count: standalone,
        groups,
    };

    to_json(&out)
}

#[cfg(test)]
mod dashboard_tests {
    use super::*;

    const OTLP_ERR: &str = r#"{"resourceSpans":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"svc-a"}}]},"scopeSpans":[{"spans":[{"traceId":"0123456789abcdef0123456789abcdef","spanId":"0123456789abcdef","name":"root","startTimeUnixNano":"1000","endTimeUnixNano":"3000","status":{"code":2}}]}]}]}"#;

    #[test]
    fn aggregates_across_traces_and_skips_bad() {
        let input = serde_json::json!([
            {"name": "t1", "json": OTLP_ERR},
            {"name": "t2", "json": OTLP_ERR},
            {"name": "bad", "json": "not json"},
        ])
        .to_string();

        let out = compute_dashboard(&input).expect("dashboard");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert_eq!(v["trace_count"], 2);
        assert_eq!(v["rows"].as_array().unwrap().len(), 2);
        assert_eq!(v["total_spans"], 2);
        assert_eq!(v["total_errors"], 2);
        assert_eq!(v["rows"][0]["has_errors"], true);
        assert_eq!(v["top_services"][0]["name"], "svc-a");
        assert_eq!(v["top_services"][0]["trace_count"], 2);
    }
}

#[cfg(test)]
mod session_group_tests {
    use super::*;

    fn otlp_with_session(trace_id: &str, session: Option<&str>) -> String {
        let attrs = match session {
            Some(s) => format!(
                r#",{{"key":"session.id","value":{{"stringValue":"{s}"}}}}"#
            ),
            None => String::new(),
        };
        format!(
            r#"{{"resourceSpans":[{{"resource":{{"attributes":[{{"key":"service.name","value":{{"stringValue":"svc"}}}}]}},"scopeSpans":[{{"spans":[{{"traceId":"{trace_id}","spanId":"0123456789abcdef","name":"op","startTimeUnixNano":"1000","endTimeUnixNano":"2000","attributes":[{{"key":"x","value":{{"stringValue":"y"}}}}{attrs}]}}]}}]}}]}}"#
        )
    }

    #[test]
    fn groups_traces_by_session() {
        let a = otlp_with_session("0123456789abcdef0123456789abcde1", Some("sess-1"));
        let b = otlp_with_session("0123456789abcdef0123456789abcde2", Some("sess-1"));
        let c = otlp_with_session("0123456789abcdef0123456789abcde3", None);
        let input = serde_json::json!([
            {"name": "a", "json": a},
            {"name": "b", "json": b},
            {"name": "c", "json": c},
        ])
        .to_string();

        let out = compute_session_groups(&input).expect("groups");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert_eq!(v["session_count"], 1);
        assert_eq!(v["standalone_count"], 1);
        // First group is the multi-trace session.
        assert_eq!(v["groups"][0]["session_id"], "sess-1");
        assert_eq!(v["groups"][0]["trace_count"], 2);
        assert_eq!(v["groups"][0]["trace_indices"], serde_json::json!([0, 1]));
        assert_eq!(v["groups"][1]["session_id"], serde_json::Value::Null);
    }

    #[test]
    fn no_sessions_means_all_standalone() {
        let a = otlp_with_session("0123456789abcdef0123456789abcde1", None);
        let input = serde_json::json!([{"name": "a", "json": a}]).to_string();
        let out = compute_session_groups(&input).expect("groups");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["session_count"], 0);
        assert_eq!(v["standalone_count"], 1);
    }
}

#[cfg(test)]
mod token_trends_tests {
    use super::*;

    const OTLP: &str = r#"{"resourceSpans":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"svc-a"}}]},"scopeSpans":[{"spans":[{"traceId":"0123456789abcdef0123456789abcdef","spanId":"0123456789abcdef","name":"chat","startTimeUnixNano":"1000","endTimeUnixNano":"2000","attributes":[{"key":"gen_ai.system","value":{"stringValue":"openai"}},{"key":"gen_ai.request.model","value":{"stringValue":"gpt-4"}},{"key":"gen_ai.usage.input_tokens","value":{"intValue":"100"}},{"key":"gen_ai.usage.output_tokens","value":{"intValue":"50"}}]}]}]}]}"#;

    fn load_otel_conventions() {
        let one = include_str!("../../../conventions/opentelemetry.json");
        let json = format!("[{one}]");
        let result = load_conventions(&json);
        CONVENTIONS.with(|c| *c.borrow_mut() = result.conventions);
    }

    #[test]
    fn aggregates_tokens_across_traces() {
        load_otel_conventions();
        let input = serde_json::json!([
            {"name": "t1", "json": OTLP},
            {"name": "t2", "json": OTLP},
        ])
        .to_string();

        let out = compute_token_trends(&input).expect("trends");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert_eq!(v["trace_count"], 2);
        assert_eq!(v["total_input_tokens"], 200);
        assert_eq!(v["total_output_tokens"], 100);
        assert_eq!(v["per_model"].as_array().unwrap().len(), 1);
        assert_eq!(v["per_model"][0]["input_tokens"], 200);
        assert_eq!(v["per_service"][0]["name"], "svc-a");
        assert_eq!(v["per_trace"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn skips_unparseable_traces() {
        let input = r#"[{"name":"bad","json":"not json"}]"#;
        let out = compute_token_trends(input).expect("trends");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["trace_count"], 0);
    }
}

/// End-to-end tests for the binding surface.
///
/// Every exported function is reachable natively because `ApiError` is a plain
/// `WideError` off-wasm, so error paths are exercised here rather than left to
/// the browser. Each `#[test]` runs on its own thread, which is what keeps the
/// thread-local trace state from leaking between cases.
#[cfg(test)]
mod api_tests {
    use super::*;

    const OTLP: &str = include_str!("../../../test-fixtures/otlp/sample_llm_pipeline.json");
    const JAEGER: &str = include_str!("../../../test-fixtures/jaeger/sample_llm_pipeline.json");
    const OI: &str = include_str!("../../../test-fixtures/openinference/sample_llm_pipeline.json");
    const EDGE: &str = include_str!("../../../test-fixtures/domains/otlp-edge-cases.json");
    const K8S: &str = include_str!("../../../test-fixtures/domains/otlp-kubernetes-control-plane.json");

    const OTEL_CONV: &str = include_str!("../../../conventions/opentelemetry.json");
    const OI_CONV: &str = include_str!("../../../conventions/openinference.json");
    const LC_CONV: &str = include_str!("../../../conventions/langchain.json");
    const PRICING_JSON: &str = include_str!("../../../conventions/pricing.json");

    fn json(raw: &str) -> serde_json::Value {
        serde_json::from_str(raw).expect("response should be JSON")
    }

    /// Load conventions + pricing the way the UI does at boot.
    fn boot() {
        let merged = format!("[{OTEL_CONV},{OI_CONV},{LC_CONV}]");
        init(&merged).expect("conventions load");
        init_pricing(PRICING_JSON).expect("pricing load");
    }

    fn load(raw: &str) -> serde_json::Value {
        json(&parse_trace(raw).expect("fixture should parse"))
    }

    // ---------------------------------------------------------------- init

    #[test]
    fn init_reports_how_many_conventions_loaded() {
        let merged = format!("[{OTEL_CONV},{OI_CONV},{LC_CONV}]");
        let v = json(&init(&merged).unwrap());
        assert_eq!(v["conventions_loaded"], 3);
        assert_eq!(v["warnings"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn init_surfaces_a_warning_for_a_malformed_convention() {
        let v = json(&init(r#"[{"name":"broken"}]"#).unwrap());
        assert_eq!(v["conventions_loaded"], 0);
        assert_eq!(v["warnings"][0]["code"], "CONVENTION_ERROR");
    }

    #[test]
    fn init_warns_when_the_payload_is_not_an_array() {
        let v = json(&init("{}").unwrap());
        assert_eq!(v["conventions_loaded"], 0);
        assert_eq!(v["warnings"][0]["code"], "CONVENTION_ERROR");
    }

    #[test]
    fn init_pricing_loads_the_bundled_table() {
        let v = json(&init_pricing(PRICING_JSON).unwrap());
        assert!(v["models_loaded"].as_u64().unwrap() >= 20);
    }

    #[test]
    fn init_pricing_reports_a_warning_rather_than_failing_on_bad_input() {
        // A broken pricing table costs the cost column, not the whole session,
        // so it degrades to zero models plus a warning.
        let v = json(&init_pricing("not json").unwrap());
        assert_eq!(v["models_loaded"], 0);
        assert_eq!(v["warnings"].as_array().unwrap().len(), 1);
    }

    // --------------------------------------------------------- parse_trace

    #[test]
    fn parse_trace_reads_all_three_formats() {
        for (raw, format) in [
            (OTLP, "OtlpJson"),
            (JAEGER, "JaegerJson"),
            (OI, "OpenInferenceJson"),
        ] {
            let v = load(raw);
            assert_eq!(v["detected_format"], format, "format for {format}");
            assert_eq!(v["span_count"], 7);
        }
    }

    #[test]
    fn parse_trace_resolves_llm_spans_and_cost_once_conventions_are_loaded() {
        boot();
        let v = load(OTLP);
        assert_eq!(v["llm_span_count"], 4);
        let cost = json(&get_cost_breakdown().unwrap());
        assert!(cost["total_cost_usd"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn parse_trace_reports_data_quality_warnings() {
        let v = load(EDGE);
        let codes: Vec<&str> = v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w["code"].as_str().unwrap())
            .collect();
        assert!(codes.contains(&"DUPLICATE_SPAN_ID"), "{codes:?}");
        assert!(codes.contains(&"ORPHAN_PARENT"), "{codes:?}");
        assert!(codes.contains(&"TIMESTAMP_INVERTED"), "{codes:?}");
    }

    #[test]
    fn parse_trace_rejects_invalid_json_with_a_position() {
        let err = parse_trace("{ not json").unwrap_err();
        match err {
            WideError::InvalidJson { line, .. } => assert_eq!(line, Some(1)),
            other => panic!("expected InvalidJson, got {other:?}"),
        }
    }

    #[test]
    fn parse_trace_rejects_an_unrecognized_shape() {
        assert!(matches!(
            parse_trace(r#"{"hello":"world"}"#).unwrap_err(),
            WideError::UnrecognizedFormat
        ));
    }

    // ------------------------------------------------------------- layouts

    #[test]
    fn layouts_need_a_loaded_trace() {
        for result in [
            compute_flamegraph(),
            compute_timeline(),
            compute_waterfall(),
            get_service_graph(),
            get_critical_path(),
            compute_agent_flow_layout(),
        ] {
            assert!(matches!(result.unwrap_err(), WideError::NoTraceLoaded));
        }
    }

    #[test]
    fn flamegraph_covers_every_span() {
        load(OTLP);
        let v = json(&compute_flamegraph().unwrap());
        assert_eq!(v["nodes"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn timeline_groups_spans_into_service_lanes() {
        load(OTLP);
        let v = json(&compute_timeline().unwrap());
        assert_eq!(v["blocks"].as_array().unwrap().len(), 7);
        // Three services in the sample, each with at least one lane row.
        assert!(v["rows"].as_array().unwrap().len() >= 3);
    }

    #[test]
    fn waterfall_rows_are_depth_ordered() {
        load(OTLP);
        let v = json(&compute_waterfall().unwrap());
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[0]["depth"], 0);
    }

    #[test]
    fn service_graph_links_the_calling_services() {
        load(K8S);
        let v = json(&get_service_graph().unwrap());
        assert_eq!(v["nodes"].as_array().unwrap().len(), 6);
        assert!(!v["edges"].as_array().unwrap().is_empty());
    }

    #[test]
    fn critical_path_is_a_chain_through_the_root() {
        load(OTLP);
        let v = json(&get_critical_path().unwrap());
        assert!(!v["span_ids"].as_array().unwrap().is_empty());
    }

    #[test]
    fn agent_flow_layout_is_available_for_an_llm_trace() {
        boot();
        load(OTLP);
        let v = json(&compute_agent_flow_layout().unwrap());
        assert!(v.get("nodes").is_some());
    }

    // --------------------------------------------------------- span detail

    #[test]
    fn span_detail_needs_a_loaded_trace() {
        assert!(matches!(
            get_span_detail("whatever").unwrap_err(),
            WideError::NoTraceLoaded
        ));
    }

    #[test]
    fn span_detail_rejects_an_unknown_span_id() {
        load(OTLP);
        assert!(get_span_detail("ffffffffffffffff").is_err());
    }

    #[test]
    fn span_detail_carries_timing_and_llm_metadata() {
        boot();
        let summary = load(OTLP);
        let flame = json(&compute_flamegraph().unwrap());
        let root_id = summary["trace_id"].as_str().unwrap().to_string();
        assert!(!root_id.is_empty());

        let llm_span = flame["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["is_llm"] == true)
            .expect("sample trace has LLM spans");
        let detail = json(&get_span_detail(llm_span["span_id"].as_str().unwrap()).unwrap());
        assert!(detail["duration_display"].as_str().unwrap().len() > 1);
        assert!(detail["llm"]["model_name"].is_string());
    }

    // -------------------------------------------------------------- search

    #[test]
    fn search_needs_a_loaded_trace() {
        assert!(matches!(
            search_spans("chat").unwrap_err(),
            WideError::NoTraceLoaded
        ));
    }

    #[test]
    fn search_matches_span_names_and_reports_misses() {
        load(OTLP);
        let hits = json(&search_spans("chat").unwrap());
        assert!(!hits.as_array().unwrap().is_empty());
        let misses = json(&search_spans("nothing-matches-this").unwrap());
        assert!(misses.as_array().unwrap().is_empty());
    }

    // -------------------------------------------------------------- filter

    #[test]
    fn filter_needs_a_loaded_trace() {
        assert!(matches!(
            filter_spans("{}").unwrap_err(),
            WideError::NoTraceLoaded
        ));
    }

    #[test]
    fn filter_rejects_a_malformed_request() {
        load(OTLP);
        assert!(filter_spans("{").is_err());
    }

    #[test]
    fn filter_narrows_by_each_facet() {
        boot();
        load(OTLP);
        let all = json(&filter_spans("{}").unwrap());
        assert_eq!(all.as_array().unwrap().len(), 7);

        // Empty strings mean "no filter", not "match the empty value".
        let blank = json(&filter_spans(r#"{"status":"","service":"","kind":""}"#).unwrap());
        assert_eq!(blank.as_array().unwrap().len(), 7);

        let llm = json(&filter_spans(r#"{"llm_only":true}"#).unwrap());
        assert_eq!(llm.as_array().unwrap().len(), 4);

        let none = json(&filter_spans(r#"{"kind":"producer"}"#).unwrap());
        assert!(none.as_array().unwrap().is_empty());

        let safety = json(&filter_spans(r#"{"safety_only":true}"#).unwrap());
        assert!(safety.as_array().unwrap().is_empty());
    }

    // ---------------------------------------------------------- comparison

    #[test]
    fn comparison_flamegraph_needs_a_comparison_trace() {
        assert!(matches!(
            get_comparison_flamegraph().unwrap_err(),
            WideError::NoTraceLoaded
        ));
    }

    #[test]
    fn comparison_trace_round_trips_and_clears() {
        load(OTLP);
        let v = json(&parse_comparison_trace(JAEGER).unwrap());
        assert_eq!(v["span_count"], 7);
        assert!(get_comparison_flamegraph().is_ok());

        clear_comparison();
        assert!(get_comparison_flamegraph().is_err());
    }

    #[test]
    fn comparison_trace_rejects_junk() {
        assert!(parse_comparison_trace("{}").is_err());
    }

    // -------------------------------------------------------------- share

    #[test]
    fn share_blobs_round_trip() {
        let blob = compress_share(OTLP);
        assert!(blob.len() < OTLP.len());
        assert_eq!(decompress_share(&blob).unwrap(), OTLP);
    }

    #[test]
    fn share_rejects_a_corrupt_blob() {
        assert!(decompress_share(&[0, 1, 2, 3]).is_err());
        assert!(decompress_share(&[]).is_err());
    }

    // ------------------------------------------------------ cost breakdown

    #[test]
    fn cost_breakdown_needs_a_loaded_trace() {
        assert!(matches!(
            get_cost_breakdown().unwrap_err(),
            WideError::NoTraceLoaded
        ));
    }

    #[test]
    fn cost_breakdown_is_empty_without_llm_spans() {
        load(K8S);
        let v = json(&get_cost_breakdown().unwrap());
        assert_eq!(v["total_cost_usd"], 0.0);
    }

    // ------------------------------------------------- multi-trace metrics

    fn two_traces() -> String {
        serde_json::to_string(&serde_json::json!([
            { "name": "a", "json": OTLP },
            { "name": "b", "json": JAEGER },
        ]))
        .unwrap()
    }

    #[test]
    fn comparison_matrix_has_one_column_per_trace() {
        boot();
        let v = json(&compute_comparison_matrix(&two_traces()).unwrap());
        assert_eq!(v["traces"].as_array().unwrap().len(), 2);
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 7, "one row per metric");
        // Every metric carries a value and a display string per trace.
        for row in rows {
            assert_eq!(row["values"].as_array().unwrap().len(), 2);
            assert_eq!(row["display"].as_array().unwrap().len(), 2);
        }
    }

    #[test]
    fn comparison_matrix_rejects_malformed_input() {
        assert!(compute_comparison_matrix("[").is_err());
        assert!(compute_comparison_matrix(r#"[{"name":"a","json":"{}"}]"#).is_err());
    }

    #[test]
    fn token_trends_aggregate_across_traces() {
        boot();
        let v = json(&compute_token_trends(&two_traces()).unwrap());
        assert_eq!(v["trace_count"], 2);
    }

    #[test]
    fn token_trends_reject_malformed_input() {
        assert!(compute_token_trends("[").is_err());
    }

    #[test]
    fn dashboard_summarizes_every_trace() {
        boot();
        let v = json(&compute_dashboard(&two_traces()).unwrap());
        assert_eq!(v["rows"].as_array().unwrap().len(), 2);
        assert_eq!(v["trace_count"], 2);
        assert_eq!(v["total_spans"], 14);
    }

    #[test]
    fn dashboard_rejects_malformed_input() {
        assert!(compute_dashboard("[").is_err());
    }

    #[test]
    fn session_groups_fall_back_to_one_group_per_trace() {
        let v = json(&compute_session_groups(&two_traces()).unwrap());
        assert!(!v["groups"].as_array().unwrap().is_empty());
    }

    #[test]
    fn session_groups_reject_malformed_input() {
        assert!(compute_session_groups("[").is_err());
    }
}

#[cfg(test)]
mod to_json_tests {
    use super::*;

    #[test]
    fn to_json_serializes_a_payload() {
        assert_eq!(to_json(&vec![1, 2, 3]).unwrap(), "[1,2,3]");
    }

    #[test]
    fn to_json_reports_a_payload_serde_cannot_encode() {
        // JSON object keys must be strings, so a map keyed by a tuple is the
        // input that makes serialization fail — the arm every response shares.
        let mut impossible = std::collections::HashMap::new();
        impossible.insert((1, 2), "value");
        let err = to_json(&impossible).unwrap_err();
        assert!(matches!(err, WideError::InvalidJson { .. }));
    }
}

/// The in-trace search DSL: `duration>100ms status=error service~api kind=client llm`.
///
/// Each operator gets its own case because a wrong comparison here silently
/// hides spans rather than failing loudly.
#[cfg(test)]
mod search_dsl_tests {
    use super::*;
    use models::span::{SpanKind, SpanStatus};

    fn span(name: &str, service: &str, duration_ns: u64) -> Span {
        Span {
            trace_id: "t".into(),
            span_id: name.to_string(),
            parent_span_id: None,
            operation_name: name.to_string(),
            service_name: service.to_string(),
            span_kind: SpanKind::Client,
            start_time_ns: 0,
            end_time_ns: duration_ns,
            duration_ns,
            self_time_ns: duration_ns,
            status: SpanStatus::Ok,
            attributes: std::collections::HashMap::from([(
                "http.route".to_string(),
                AttributeValue::String("/api/chat".into()),
            )]),
            events: vec![],
            llm: None,
            safety: vec![],
        }
    }

    /// Minimal resolved LLM metadata — presence is all the `llm` token checks.
    fn llm_marker() -> LlmSpanAttributes {
        LlmSpanAttributes {
            operation_type: models::llm::LlmOperationType::ChatCompletion,
            model_name: None,
            model_provider: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
            input_messages: vec![],
            output_messages: vec![],
            tool_calls: vec![],
            temperature: None,
            top_p: None,
            max_tokens: None,
            embedding_dimensions: None,
            embedding_count: None,
            retrieved_documents: vec![],
            eval_scores: vec![],
        }
    }

    #[test]
    fn a_query_without_operators_matches_everything() {
        assert!(parse_search_operators("just some words").is_none());
        assert!(span_matches_operators(&span("a", "svc", 10), "just some words"));
    }

    #[test]
    fn duration_units_all_convert_to_nanoseconds() {
        assert_eq!(parse_duration_query("1ms"), Some(1_000_000));
        assert_eq!(parse_duration_query("1µs"), Some(1_000));
        assert_eq!(parse_duration_query("1us"), Some(1_000));
        assert_eq!(parse_duration_query("1.5s"), Some(1_500_000_000));
        assert_eq!(parse_duration_query("250"), Some(250));
        assert_eq!(parse_duration_query("wat"), None);
    }

    #[test]
    fn every_duration_comparison_is_honoured() {
        let s = span("a", "svc", 100_000_000); // 100ms
        assert!(span_matches_operators(&s, "duration>50ms"));
        assert!(!span_matches_operators(&s, "duration>100ms"));
        assert!(span_matches_operators(&s, "duration>=100ms"));
        assert!(!span_matches_operators(&s, "duration>=101ms"));
        assert!(span_matches_operators(&s, "dur<200ms"));
        assert!(!span_matches_operators(&s, "dur<100ms"));
        assert!(span_matches_operators(&s, "dur<=100ms"));
        assert!(!span_matches_operators(&s, "dur<=99ms"));
        assert!(span_matches_operators(&s, "duration=100ms"));
        assert!(!span_matches_operators(&s, "duration=99ms"));
    }

    #[test]
    fn an_unparseable_duration_matches_nothing() {
        assert!(!span_matches_operators(&span("a", "svc", 10), "duration>wat"));
    }

    #[test]
    fn status_kind_and_service_filter_independently() {
        let s = span("a", "api-gateway", 10);
        assert!(span_matches_operators(&s, "status=ok"));
        assert!(!span_matches_operators(&s, "status=error"));
        assert!(span_matches_operators(&s, "kind=client"));
        assert!(!span_matches_operators(&s, "kind=server"));
        assert!(span_matches_operators(&s, "service~api"));
        assert!(span_matches_operators(&s, "svc~gateway"));
        assert!(!span_matches_operators(&s, "service~payments"));
    }

    #[test]
    fn the_llm_token_requires_resolved_llm_metadata() {
        let mut s = span("a", "svc", 10);
        assert!(!span_matches_operators(&s, "llm"));
        s.llm = Some(llm_marker());
        assert!(span_matches_operators(&s, "llm"));
    }

    #[test]
    fn an_unknown_key_falls_back_to_attribute_matching() {
        let mut s = span("a", "svc", 10);
        assert!(span_matches_operators(&s, "route~chat"));
        assert!(!span_matches_operators(&s, "route~missing"));

        // Non-text attribute values never match a text query.
        s.attributes = std::collections::HashMap::from([(
            "http.status".to_string(),
            AttributeValue::Int(200),
        )]);
        assert!(!span_matches_operators(&s, "status~200"));

        s.attributes = std::collections::HashMap::from([(
            "tags".to_string(),
            AttributeValue::StringArray(vec!["alpha".into(), "beta".into()]),
        )]);
        assert!(span_matches_operators(&s, "tags~beta"));
        assert!(!span_matches_operators(&s, "tags~gamma"));
    }

    #[test]
    fn operators_combine_conjunctively() {
        let s = span("a", "api", 100_000_000);
        assert!(span_matches_operators(&s, "duration>50ms service~api status=ok"));
        assert!(!span_matches_operators(&s, "duration>50ms service~payments"));
    }

    #[test]
    fn a_lone_tilde_without_a_key_is_ignored() {
        // `~foo` parses to key "" which matches any attribute key.
        assert!(parse_search_operators("~foo").is_some());
    }

    #[test]
    fn percentiles_interpolate_over_the_sorted_slice() {
        assert_eq!(percentile(&[], 0.5), 0);
        assert_eq!(percentile(&[10], 0.5), 10);
        assert_eq!(percentile(&[10, 20, 30, 40], 0.5), 30);
        assert_eq!(percentile(&[10, 20, 30, 40], 0.95), 40);
    }
}
