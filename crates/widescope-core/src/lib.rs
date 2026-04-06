mod conventions;
mod errors;
mod layout;
mod models;
mod parsers;
mod trace_builder;
mod utils;

use std::cell::RefCell;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use conventions::registry::{load_conventions, Convention};
use errors::WideError;
use layout::flamegraph::compute_flamegraph_layout;
use layout::timeline::compute_timeline_layout;
use layout::waterfall::compute_waterfall_layout;
use models::layout::{
    EventDetail, LlmDetail, MessageDetail, SpanDetailResponse, ToolCallDetail,
};
use models::llm::LlmSpanAttributes;
use models::span::SpanEvent;
use models::trace::{ParseWarning, Trace};
use parsers::otlp_json::parse_otlp_with_warnings;
use parsers::detect_format;
use trace_builder::build_trace;
use utils::{format_duration, format_timestamp_display};

thread_local! {
    static TRACE: RefCell<Option<Trace>> = RefCell::new(None);
    static CONVENTIONS: RefCell<Vec<Convention>> = RefCell::new(Vec::new());
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

    serde_json::to_string(&init_result)
        .map_err(|e| WideError::InvalidJson {
            message: e.to_string(),
            line: None,
            column: None,
        }.into())
}

#[derive(Serialize)]
struct TraceSummary {
    trace_id: String,
    span_count: usize,
    service_count: usize,
    detected_format: String,
    has_errors: bool,
    total_duration_ns: u64,
    total_duration_display: String,
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

    let parse_result = match &format {
        models::trace::InputFormat::OtlpJson => {
            parse_otlp_with_warnings(&value).map_err(JsValue::from)?
        }
        _ => {
            return Err(WideError::UnrecognizedFormat.into());
        }
    };

    let conventions = CONVENTIONS.with(|c| c.borrow().clone());

    let mut spans = parse_result.spans;

    for span in &mut spans {
        span.llm = conventions::resolver::resolve_llm_attributes(span, &conventions);
    }

    let trace = build_trace(spans, format, parse_result.warnings).map_err(JsValue::from)?;

    let root_op = trace.root_span_ids.first().and_then(|id| {
        trace.get_span(id).map(|s| s.operation_name.clone())
    });
    let root_svc = trace.root_span_ids.first().and_then(|id| {
        trace.get_span(id).map(|s| s.service_name.clone())
    });

    let summary = TraceSummary {
        trace_id: trace.trace_id.clone(),
        span_count: trace.span_count,
        service_count: trace.service_count,
        detected_format: trace.detected_format.as_str().to_string(),
        has_errors: trace.has_errors,
        total_duration_ns: trace.total_duration_ns,
        total_duration_display: format_duration(trace.total_duration_ns),
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
                let span = trace.get_span(span_id).ok_or_else(|| {
                    WideError::SpanNotFound {
                        span_id: span_id.to_string(),
                    }
                })?;

                let children_ids = trace.get_children(span_id).to_vec();

                let mut attributes: Vec<(String, String)> = span
                    .attributes
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_display_string()))
                    .collect();
                attributes.sort_by(|a, b| a.0.cmp(&b.0));

                let events: Vec<EventDetail> = span
                    .events
                    .iter()
                    .map(|e| build_event_detail(e))
                    .collect();

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
