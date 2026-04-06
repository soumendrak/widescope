use std::collections::HashMap;
use serde_json::Value;
use crate::errors::WideError;
use crate::models::span::{AttributeValue, Span, SpanEvent, SpanKind, SpanStatus};
use crate::models::trace::ParseWarning;

pub struct JaegerParseResult {
    pub spans: Vec<Span>,
    pub warnings: Vec<ParseWarning>,
}

pub fn parse_jaeger(root: &Value) -> Result<Vec<Span>, WideError> {
    let result = parse_jaeger_with_warnings(root)?;
    Ok(result.spans)
}

pub fn parse_jaeger_with_warnings(root: &Value) -> Result<JaegerParseResult, WideError> {
    let traces = root
        .get("data")
        .and_then(|value| value.as_array())
        .ok_or(WideError::UnrecognizedFormat)?;

    let mut spans = Vec::new();
    let mut warnings = Vec::new();
    let mut skip_count = 0usize;
    let mut skip_reasons = Vec::new();

    for trace in traces {
        let process_services = parse_process_services(trace);
        let empty_spans: Vec<Value> = Vec::new();
        let raw_spans = trace
            .get("spans")
            .and_then(|value| value.as_array())
            .unwrap_or(&empty_spans);

        for raw_span in raw_spans {
            match parse_single_span(raw_span, &process_services) {
                Ok(span) => spans.push(span),
                Err(reason) => {
                    skip_count += 1;
                    if skip_reasons.len() < 5 {
                        skip_reasons.push(reason);
                    }
                }
            }
        }
    }

    if skip_count > 0 {
        let message = format!(
            "{} span(s) skipped due to missing required fields: {}",
            skip_count,
            skip_reasons.join("; ")
        );
        warnings.push(
            ParseWarning::new("SPAN_MISSING_REQUIRED", message).with_count(skip_count),
        );
    }

    if spans.is_empty() {
        return Err(WideError::NoValidSpans {
            attempted: skip_count,
            failures: skip_reasons,
        });
    }

    Ok(JaegerParseResult { spans, warnings })
}

fn parse_process_services(trace: &Value) -> HashMap<String, String> {
    let mut process_services = HashMap::new();

    let Some(processes) = trace.get("processes").and_then(|value| value.as_object()) else {
        return process_services;
    };

    for (process_id, process) in processes {
        let service_name = process
            .get("serviceName")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown_service")
            .to_string();
        process_services.insert(process_id.clone(), service_name);
    }

    process_services
}

fn parse_single_span(
    raw: &Value,
    process_services: &HashMap<String, String>,
) -> Result<Span, String> {
    let span_id = raw
        .get("spanID")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing spanID".to_string())?
        .to_string();

    let trace_id = raw
        .get("traceID")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("span {}: missing traceID", span_id))?
        .to_string();

    let start_time_us = raw
        .get("startTime")
        .and_then(parse_micro_u64)
        .ok_or_else(|| format!("span {}: missing startTime", span_id))?;

    let duration_us = raw
        .get("duration")
        .and_then(parse_micro_u64)
        .ok_or_else(|| format!("span {}: missing duration", span_id))?;

    let start_time_ns = start_time_us.saturating_mul(1_000);
    let duration_ns = duration_us.saturating_mul(1_000);
    let end_time_ns = start_time_ns.saturating_add(duration_ns);

    let operation_name = raw
        .get("operationName")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    let parent_span_id = parse_parent_span_id(raw);
    let attributes = parse_tags(
        raw.get("tags")
            .and_then(|value| value.as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    let events = parse_logs(
        raw.get("logs")
            .and_then(|value| value.as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    let process_id = raw.get("processID").and_then(|value| value.as_str()).unwrap_or("");
    let service_name = process_services
        .get(process_id)
        .cloned()
        .unwrap_or_else(|| "unknown_service".to_string());
    let span_kind = span_kind_from_attributes(&attributes);
    let status = status_from_attributes(&attributes);

    Ok(Span {
        trace_id,
        span_id,
        parent_span_id,
        operation_name,
        service_name,
        span_kind,
        start_time_ns,
        end_time_ns,
        duration_ns,
        self_time_ns: 0,
        status,
        attributes,
        events,
        llm: None,
    })
}

fn parse_parent_span_id(raw: &Value) -> Option<String> {
    let references = raw.get("references").and_then(|value| value.as_array())?;

    if let Some(parent) = references.iter().find_map(|reference| {
        let ref_type = reference.get("refType").and_then(|value| value.as_str())?;
        if ref_type == "CHILD_OF" {
            reference
                .get("spanID")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        } else {
            None
        }
    }) {
        return Some(parent);
    }

    references
        .iter()
        .find_map(|reference| {
            reference
                .get("spanID")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        })
}

fn parse_micro_u64(value: &Value) -> Option<u64> {
    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    if let Some(number) = value.as_i64() {
        return u64::try_from(number).ok();
    }
    if let Some(number) = value.as_f64() {
        if number.is_sign_negative() {
            return None;
        }
        return Some(number as u64);
    }
    if let Some(number) = value.as_str() {
        return number.parse::<u64>().ok();
    }
    None
}

fn parse_logs(logs: &[Value]) -> Vec<SpanEvent> {
    logs.iter()
        .filter_map(|log| {
            let timestamp_ns = log
                .get("timestamp")
                .and_then(parse_micro_u64)
                .map(|value| value.saturating_mul(1_000))
                .unwrap_or(0);
            let attributes = parse_tags(
                log.get("fields")
                    .and_then(|value| value.as_array())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            );
            let name = attributes
                .get("event")
                .and_then(AttributeValue::as_str)
                .or_else(|| attributes.get("message").and_then(AttributeValue::as_str))
                .unwrap_or("log")
                .to_string();
            Some(SpanEvent {
                name,
                timestamp_ns,
                attributes,
            })
        })
        .collect()
}

fn parse_tags(tags: &[Value]) -> HashMap<String, AttributeValue> {
    let mut attributes = HashMap::new();

    for tag in tags {
        let Some(key) = tag.get("key").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(value) = parse_tag_value(tag) else {
            continue;
        };
        attributes.insert(key.to_string(), value);
    }

    attributes
}

fn parse_tag_value(tag: &Value) -> Option<AttributeValue> {
    let tag_type = tag.get("type").and_then(|value| value.as_str()).unwrap_or("");
    let raw_value = tag.get("value")?;

    match tag_type {
        "string" => raw_value
            .as_str()
            .map(|value| AttributeValue::String(value.to_string()))
            .or_else(|| Some(AttributeValue::String(raw_value.to_string()))),
        "bool" | "boolean" => raw_value
            .as_bool()
            .map(AttributeValue::Bool)
            .or_else(|| raw_value.as_str().and_then(|value| value.parse::<bool>().ok()).map(AttributeValue::Bool)),
        "int64" | "int32" | "int" | "long" => raw_value
            .as_i64()
            .map(AttributeValue::Int)
            .or_else(|| raw_value.as_u64().and_then(|value| i64::try_from(value).ok()).map(AttributeValue::Int))
            .or_else(|| raw_value.as_str().and_then(|value| value.parse::<i64>().ok()).map(AttributeValue::Int)),
        "float64" | "float32" | "float" | "double" => raw_value
            .as_f64()
            .map(AttributeValue::Float)
            .or_else(|| raw_value.as_str().and_then(|value| value.parse::<f64>().ok()).map(AttributeValue::Float)),
        "binary" => raw_value
            .as_str()
            .map(|value| AttributeValue::String(format!("base64:{}", value)))
            .or_else(|| Some(AttributeValue::String(raw_value.to_string()))),
        _ => {
            if let Some(value) = raw_value.as_str() {
                Some(AttributeValue::String(value.to_string()))
            } else if let Some(value) = raw_value.as_bool() {
                Some(AttributeValue::Bool(value))
            } else if let Some(value) = raw_value.as_i64() {
                Some(AttributeValue::Int(value))
            } else if let Some(value) = raw_value.as_u64() {
                i64::try_from(value).ok().map(AttributeValue::Int)
            } else {
                raw_value.as_f64().map(AttributeValue::Float)
            }
        }
    }
}

fn span_kind_from_attributes(attributes: &HashMap<String, AttributeValue>) -> SpanKind {
    match attributes
        .get("span.kind")
        .and_then(AttributeValue::as_str)
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("server") => SpanKind::Server,
        Some("client") => SpanKind::Client,
        Some("producer") => SpanKind::Producer,
        Some("consumer") => SpanKind::Consumer,
        _ => SpanKind::Internal,
    }
}

fn status_from_attributes(attributes: &HashMap<String, AttributeValue>) -> SpanStatus {
    let error_message = attributes
        .get("error.message")
        .and_then(AttributeValue::as_str)
        .or_else(|| attributes.get("otel.status_description").and_then(AttributeValue::as_str))
        .unwrap_or("")
        .to_string();

    if attributes
        .get("error")
        .and_then(|value| match value {
            AttributeValue::Bool(flag) => Some(*flag),
            AttributeValue::String(flag) => flag.parse::<bool>().ok(),
            _ => None,
        })
        .unwrap_or(false)
    {
        return SpanStatus::Error {
            message: error_message,
        };
    }

    match attributes
        .get("otel.status_code")
        .and_then(AttributeValue::as_str)
        .or_else(|| attributes.get("status.code").and_then(AttributeValue::as_str))
        .map(|value| value.to_ascii_uppercase())
        .as_deref()
    {
        Some("ERROR") => SpanStatus::Error {
            message: error_message,
        },
        Some("OK") => SpanStatus::Ok,
        _ => SpanStatus::Unset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_basic_jaeger_trace() {
        let input = json!({
            "data": [
                {
                    "traceID": "trace-1",
                    "processes": {
                        "p1": { "serviceName": "gateway" },
                        "p2": { "serviceName": "worker" }
                    },
                    "spans": [
                        {
                            "traceID": "trace-1",
                            "spanID": "root",
                            "operationName": "GET /chat",
                            "references": [],
                            "startTime": 1000,
                            "duration": 25,
                            "processID": "p1",
                            "tags": [
                                { "key": "span.kind", "type": "string", "value": "server" }
                            ],
                            "logs": []
                        },
                        {
                            "traceID": "trace-1",
                            "spanID": "child",
                            "operationName": "call-model",
                            "references": [
                                { "refType": "CHILD_OF", "traceID": "trace-1", "spanID": "root" }
                            ],
                            "startTime": 1005,
                            "duration": 10,
                            "processID": "p2",
                            "tags": [
                                { "key": "span.kind", "type": "string", "value": "client" },
                                { "key": "error", "type": "bool", "value": true },
                                { "key": "error.message", "type": "string", "value": "boom" }
                            ],
                            "logs": [
                                {
                                    "timestamp": 1010,
                                    "fields": [
                                        { "key": "event", "type": "string", "value": "retry" }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let result = parse_jaeger_with_warnings(&input).expect("jaeger trace should parse");

        assert_eq!(result.spans.len(), 2);
        assert!(result.warnings.is_empty());

        let root = result.spans.iter().find(|span| span.span_id == "root").unwrap();
        assert_eq!(root.service_name, "gateway");
        assert!(matches!(root.span_kind, SpanKind::Server));
        assert_eq!(root.duration_ns, 25_000);

        let child = result.spans.iter().find(|span| span.span_id == "child").unwrap();
        assert_eq!(child.parent_span_id.as_deref(), Some("root"));
        assert_eq!(child.service_name, "worker");
        assert!(matches!(child.span_kind, SpanKind::Client));
        assert!(matches!(child.status, SpanStatus::Error { .. }));
        assert_eq!(child.events.len(), 1);
        assert_eq!(child.events[0].name, "retry");
        assert_eq!(child.events[0].timestamp_ns, 1_010_000);
    }
}
