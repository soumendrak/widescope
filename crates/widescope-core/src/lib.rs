mod conventions;
mod errors;
mod layout;
mod models;
mod parsers;
mod trace_builder;
mod utils;

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use conventions::registry::{load_conventions, Convention};
use errors::WideError;
use layout::critical_path::compute_critical_path;
use layout::flamegraph::compute_flamegraph_layout;
use layout::graph::compute_service_graph as build_service_graph;
use layout::timeline::compute_timeline_layout;
use layout::waterfall::compute_waterfall_layout;
use models::layout::{EventDetail, LlmDetail, MessageDetail, SpanDetailResponse, ToolCallDetail};
use models::llm::LlmSpanAttributes;
use models::span::{AttributeValue, Span, SpanEvent};
use models::trace::{ParseWarning, Trace};
use parsers::detect_format;
use parsers::jaeger::parse_jaeger_with_warnings;
use parsers::openinference::parse_openinference_with_warnings;
use parsers::otlp_json::parse_otlp_with_warnings;
use trace_builder::build_trace;
use utils::{format_duration, format_timestamp_display};

thread_local! {
    static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
    static COMPARISON_TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
    static CONVENTIONS: RefCell<Vec<Convention>> = const { RefCell::new(Vec::new()) };
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

#[wasm_bindgen]
pub fn init(conventions_json: &str) -> Result<String, JsValue> {
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

    serde_json::to_string(&init_result).map_err(|e| {
        WideError::InvalidJson {
            message: e.to_string(),
            line: None,
            column: None,
        }
        .into()
    })
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

#[wasm_bindgen]
pub fn parse_trace(raw_input: &str) -> Result<String, JsValue> {
    let value: serde_json::Value =
        serde_json::from_str(raw_input).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let format = detect_format(&value).map_err(JsValue::from)?;

    let (mut spans, parse_warnings) = match &format {
        models::trace::InputFormat::OtlpJson => {
            let result = parse_otlp_with_warnings(&value).map_err(JsValue::from)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::JaegerJson => {
            let result = parse_jaeger_with_warnings(&value).map_err(JsValue::from)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::OpenInferenceJson => {
            let result = parse_openinference_with_warnings(&value).map_err(JsValue::from)?;
            (result.spans, result.warnings)
        }
        _ => {
            return Err(WideError::UnrecognizedFormat.into());
        }
    };

    let conventions = CONVENTIONS.with(|c| c.borrow().clone());

    for span in &mut spans {
        span.llm = conventions::resolver::resolve_llm_attributes(span, &conventions);
    }

    let trace = build_trace(spans, format, parse_warnings).map_err(JsValue::from)?;

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

    serde_json::to_string(&summary).map_err(|e| {
        WideError::InvalidJson {
            message: e.to_string(),
            line: None,
            column: None,
        }
        .into()
    })
}

#[wasm_bindgen]
pub fn compute_flamegraph() -> Result<String, JsValue> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_flamegraph_layout(trace);
                serde_json::to_string(&layout).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}

#[wasm_bindgen]
pub fn compute_timeline() -> Result<String, JsValue> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_timeline_layout(trace);
                serde_json::to_string(&layout).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}

#[wasm_bindgen]
pub fn get_span_detail(span_id: &str) -> Result<String, JsValue> {
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
                    children_ids,
                };

                serde_json::to_string(&detail).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}

#[wasm_bindgen]
pub fn search_spans(query: &str) -> Result<String, JsValue> {
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

                serde_json::to_string(&span_ids).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}

#[wasm_bindgen]
pub fn compute_waterfall() -> Result<String, JsValue> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_waterfall_layout(trace);
                serde_json::to_string(&layout).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}

#[wasm_bindgen]
pub fn get_service_graph() -> Result<String, JsValue> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let graph = build_service_graph(trace);
                serde_json::to_string(&graph).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
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
        } else if let Some(eq_pos) = token.find('=') {
            let key = token[..eq_pos].to_ascii_lowercase();
            let val = token[eq_pos + 1..].to_ascii_lowercase();
            ops.push((key, "=".into(), val));
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
        } else if token.contains('~') {
            let parts: Vec<&str> = token.splitn(2, '~').collect();
            if parts.len() == 2 {
                ops.push((parts[0].to_ascii_lowercase(), "~".into(), parts[1].to_ascii_lowercase()));
                has_op = true;
            }
        }
    }

    if has_op { Some(ops) } else { None }
}

fn parse_duration_query(val: &str) -> Option<u64> {
    let val = val.trim();
    if let Some(stripped) = val.strip_suffix("ms") {
        stripped.parse::<f64>().ok().map(|v| (v * 1_000_000.0) as u64)
    } else if let Some(stripped) = val.strip_suffix("µs") {
        stripped.parse::<f64>().ok().map(|v| (v * 1_000.0) as u64)
    } else if let Some(stripped) = val.strip_suffix("us") {
        stripped.parse::<f64>().ok().map(|v| (v * 1_000.0) as u64)
    } else if let Some(stripped) = val.strip_suffix('s') {
        stripped.parse::<f64>().ok().map(|v| (v * 1_000_000_000.0) as u64)
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
            if span.llm.is_none() { return false; }
            continue;
        }
        match key.as_str() {
            "duration" | "dur" => {
                let target = match parse_duration_query(val) {
                    Some(t) => t,
                    None => return false,
                };
                match op.as_str() {
                    ">" => if span.duration_ns <= target { return false; },
                    ">=" => if span.duration_ns < target { return false; },
                    "<" => if span.duration_ns >= target { return false; },
                    "<=" => if span.duration_ns > target { return false; },
                    "=" => if span.duration_ns != target { return false; },
                    _ => {},
                }
            }
            "status" => {
                if span.status.as_str().to_ascii_lowercase() != *val { return false; }
            }
            "kind" => {
                if span.span_kind.as_str().to_ascii_lowercase() != *val { return false; }
            }
            "service" | "svc" => {
                if !span.service_name.to_ascii_lowercase().contains(val) { return false; }
            }
            _ => {
                // Try matching as attribute key=value
                let attr_match = span.attributes.iter().any(|(attr_key, attr_val)| {
                    attr_key.to_ascii_lowercase().contains(key) &&
                    attr_val_matches_query(attr_val, val)
                });
                if !attr_match { return false; }
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
}

#[wasm_bindgen]
pub fn filter_spans(filter_json: &str) -> Result<String, JsValue> {
    let filter: FilterRequest = serde_json::from_str(filter_json).map_err(|e| {
        WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        }
    })?;

    let status_filter = filter.status.and_then(|s| {
        if s.is_empty() { None } else { Some(s.to_ascii_lowercase()) }
    });
    let service_filter = filter.service.and_then(|s| {
        if s.is_empty() { None } else { Some(s.to_ascii_lowercase()) }
    });
    let kind_filter = filter.kind.and_then(|k| {
        if k.is_empty() { None } else { Some(k.to_ascii_lowercase()) }
    });
    let llm_only = filter.llm_only.unwrap_or(false);

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
                        true
                    })
                    .map(|span| span.span_id.clone())
                    .collect();

                matches.sort();
                serde_json::to_string(&matches).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
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

#[wasm_bindgen]
pub fn parse_comparison_trace(raw_input: &str) -> Result<String, JsValue> {
    let value: serde_json::Value =
        serde_json::from_str(raw_input).map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: Some(e.line()),
            column: Some(e.column()),
        })?;

    let format = detect_format(&value).map_err(JsValue::from)?;

    let (mut spans, _parse_warnings) = match &format {
        models::trace::InputFormat::OtlpJson => {
            let result = parse_otlp_with_warnings(&value).map_err(JsValue::from)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::JaegerJson => {
            let result = parse_jaeger_with_warnings(&value).map_err(JsValue::from)?;
            (result.spans, result.warnings)
        }
        models::trace::InputFormat::OpenInferenceJson => {
            let result = parse_openinference_with_warnings(&value).map_err(JsValue::from)?;
            (result.spans, result.warnings)
        }
        _ => return Err(WideError::UnrecognizedFormat.into()),
    };

    let conventions = CONVENTIONS.with(|c| c.borrow().clone());
    for span in &mut spans {
        span.llm = conventions::resolver::resolve_llm_attributes(span, &conventions);
    }

    let trace = build_trace(spans, format, vec![]).map_err(JsValue::from)?;

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

    serde_json::to_string(&summary).map_err(|e| {
        WideError::InvalidJson {
            message: e.to_string(),
            line: None,
            column: None,
        }
        .into()
    })
}

#[wasm_bindgen]
pub fn get_comparison_flamegraph() -> Result<String, JsValue> {
    COMPARISON_TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let layout = compute_flamegraph_layout(trace);
                serde_json::to_string(&layout).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}

#[wasm_bindgen]
pub fn clear_comparison() {
    COMPARISON_TRACE.with(|t| {
        *t.borrow_mut() = None;
    });
}

#[wasm_bindgen]
pub fn get_critical_path() -> Result<String, JsValue> {
    TRACE.with(|t| {
        let borrow = t.borrow();
        match borrow.as_ref() {
            None => Err(WideError::NoTraceLoaded.into()),
            Some(trace) => {
                let cp = compute_critical_path(trace);
                serde_json::to_string(&cp).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
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

#[wasm_bindgen]
pub fn get_cost_breakdown() -> Result<String, JsValue> {
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
                            provider: llm.model_provider.clone().unwrap_or_else(|| "unknown".into()),
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

                let owned_entries: Vec<CostEntry> = entries.into_iter().map(|e| CostEntry {
                    model: e.model.clone(),
                    provider: e.provider.clone(),
                    input_tokens: e.input_tokens,
                    output_tokens: e.output_tokens,
                    total_tokens: e.total_tokens,
                    estimated_cost_usd: e.estimated_cost_usd,
                    spans: e.spans.clone(),
                }).collect();

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

                serde_json::to_string(&breakdown).map_err(|e| {
                    WideError::InvalidJson {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    }
                    .into()
                })
            }
        }
    })
}
