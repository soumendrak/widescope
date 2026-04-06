use std::collections::HashMap;
use serde_json::Value;
use crate::errors::WideError;
use crate::models::span::{AttributeValue, Span, SpanEvent, SpanKind, SpanStatus};
use crate::models::trace::ParseWarning;

pub struct OtlpParseResult {
    pub spans: Vec<Span>,
    pub warnings: Vec<ParseWarning>,
}

#[allow(dead_code)]
pub fn parse_otlp(root: &Value) -> Result<Vec<Span>, WideError> {
    let result = parse_otlp_with_warnings(root)?;
    Ok(result.spans)
}

pub fn parse_otlp_with_warnings(root: &Value) -> Result<OtlpParseResult, WideError> {
    let resource_spans = root
        .get("resourceSpans")
        .and_then(|v| v.as_array())
        .ok_or(WideError::UnrecognizedFormat)?;

    let mut spans: Vec<Span> = Vec::new();
    let mut warnings: Vec<ParseWarning> = Vec::new();
    let mut skip_count = 0usize;
    let mut skip_reasons: Vec<String> = Vec::new();

    for resource_span in resource_spans {
        let service_name = extract_service_name(resource_span);

        let empty_scope: Vec<serde_json::Value> = Vec::new();
        let scope_spans_arr = resource_span
            .get("scopeSpans")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_scope);

        for scope_span in scope_spans_arr {
            let empty_spans: Vec<serde_json::Value> = Vec::new();
            let raw_spans = scope_span
                .get("spans")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty_spans);

            for raw_span in raw_spans {
                match parse_single_span(raw_span, &service_name) {
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
    }

    if skip_count > 0 {
        let msg = format!(
            "{} span(s) skipped due to missing required fields: {}",
            skip_count,
            skip_reasons.join("; ")
        );
        warnings.push(
            ParseWarning::new("SPAN_MISSING_REQUIRED", msg).with_count(skip_count),
        );
    }

    if spans.is_empty() {
        return Err(WideError::NoValidSpans {
            attempted: skip_count,
            failures: skip_reasons,
        });
    }

    Ok(OtlpParseResult { spans, warnings })
}

fn extract_service_name(resource_span: &Value) -> String {
    resource_span
        .get("resource")
        .and_then(|r| r.get("attributes"))
        .and_then(|a| a.as_array())
        .and_then(|attrs| {
            attrs.iter().find(|attr| {
                attr.get("key")
                    .and_then(|k| k.as_str())
                    .map(|k| k == "service.name")
                    .unwrap_or(false)
            })
        })
        .and_then(|attr| {
            attr.get("value")
                .and_then(|v| v.get("stringValue"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("unknown_service")
        .to_string()
}

fn parse_single_span(raw: &Value, service_name: &str) -> Result<Span, String> {
    let span_id = raw
        .get("spanId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing spanId".to_string())?
        .to_string();

    let trace_id = raw
        .get("traceId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("span {}: missing traceId", span_id))?
        .to_string();

    let operation_name = raw
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let parent_span_id = raw
        .get("parentSpanId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let kind_int = raw
        .get("kind")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let span_kind = SpanKind::from_otlp_int(kind_int);

    let start_time_ns: u64 = raw
        .get("startTimeUnixNano")
        .and_then(parse_nano_ts)
        .ok_or_else(|| format!("span {}: missing startTimeUnixNano", span_id))?;

    let end_time_ns: u64 = raw
        .get("endTimeUnixNano")
        .and_then(parse_nano_ts)
        .ok_or_else(|| format!("span {}: missing endTimeUnixNano", span_id))?;

    let (start_time_ns, end_time_ns) = if start_time_ns <= end_time_ns {
        (start_time_ns, end_time_ns)
    } else {
        (end_time_ns, start_time_ns)
    };

    let duration_ns = end_time_ns - start_time_ns;

    let status = parse_status(raw);

    let attributes = parse_attributes(
        raw.get("attributes").and_then(|v| v.as_array()).unwrap_or(&vec![]),
    );

    let events = parse_events(
        raw.get("events").and_then(|v| v.as_array()).unwrap_or(&vec![]),
    );

    let svc = if service_name.is_empty() {
        "unknown_service".to_string()
    } else {
        service_name.to_string()
    };

    Ok(Span {
        trace_id,
        span_id,
        parent_span_id,
        operation_name,
        service_name: svc,
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

fn parse_nano_ts(v: &Value) -> Option<u64> {
    if let Some(s) = v.as_str() {
        s.parse::<u64>().ok()
    } else if let Some(n) = v.as_u64() {
        Some(n)
    } else {
        v.as_f64().map(|f| f as u64)
    }
}

fn parse_status(raw: &Value) -> SpanStatus {
    let code = raw
        .get("status")
        .and_then(|s| s.get("code"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0);

    match code {
        1 => SpanStatus::Ok,
        2 => {
            let msg = raw
                .get("status")
                .and_then(|s| s.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            SpanStatus::Error { message: msg }
        }
        _ => SpanStatus::Unset,
    }
}

pub fn parse_attributes(attrs: &[Value]) -> HashMap<String, AttributeValue> {
    let mut map = HashMap::new();
    for attr in attrs {
        let key = match attr.get("key").and_then(|k| k.as_str()) {
            Some(k) => k.to_string(),
            None => continue,
        };
        let value = match attr.get("value") {
            Some(v) => parse_any_value(v),
            None => continue,
        };
        if let Some(av) = value {
            map.insert(key, av);
        }
    }
    map
}

pub fn parse_any_value(v: &Value) -> Option<AttributeValue> {
    if let Some(s) = v.get("stringValue").and_then(|x| x.as_str()) {
        return Some(AttributeValue::String(s.to_string()));
    }
    if let Some(i_str) = v.get("intValue") {
        if let Some(s) = i_str.as_str() {
            if let Ok(n) = s.parse::<i64>() {
                return Some(AttributeValue::Int(n));
            }
        } else if let Some(n) = i_str.as_i64() {
            return Some(AttributeValue::Int(n));
        }
    }
    if let Some(f) = v.get("doubleValue").and_then(|x| x.as_f64()) {
        return Some(AttributeValue::Float(f));
    }
    if let Some(b) = v.get("boolValue").and_then(|x| x.as_bool()) {
        return Some(AttributeValue::Bool(b));
    }
    if let Some(arr_obj) = v.get("arrayValue") {
        if let Some(values) = arr_obj.get("values").and_then(|x| x.as_array()) {
            return Some(parse_array_value(values));
        }
    }
    if let Some(bytes_b64) = v.get("bytesValue").and_then(|x| x.as_str()) {
        return Some(AttributeValue::String(format!("base64:{}", bytes_b64)));
    }
    if let Some(kv_list) = v.get("kvlistValue") {
        return Some(AttributeValue::String(kv_list.to_string()));
    }
    None
}

fn parse_array_value(values: &[Value]) -> AttributeValue {
    if values.is_empty() {
        return AttributeValue::StringArray(vec![]);
    }

    let parsed: Vec<Option<AttributeValue>> =
        values.iter().map(parse_any_value).collect();

    let all_int = parsed
        .iter()
        .all(|v| matches!(v, Some(AttributeValue::Int(_))));
    if all_int {
        let ints: Vec<i64> = parsed
            .into_iter()
            .map(|v| match v {
                Some(AttributeValue::Int(i)) => i,
                _ => 0,
            })
            .collect();
        return AttributeValue::IntArray(ints);
    }

    let all_float = parsed
        .iter()
        .all(|v| matches!(v, Some(AttributeValue::Float(_)) | Some(AttributeValue::Int(_))));
    if all_float {
        let floats: Vec<f64> = parsed
            .into_iter()
            .map(|v| match v {
                Some(AttributeValue::Float(f)) => f,
                Some(AttributeValue::Int(i)) => i as f64,
                _ => 0.0,
            })
            .collect();
        return AttributeValue::FloatArray(floats);
    }

    let all_bool = parsed
        .iter()
        .all(|v| matches!(v, Some(AttributeValue::Bool(_))));
    if all_bool {
        let bools: Vec<bool> = parsed
            .into_iter()
            .map(|v| match v {
                Some(AttributeValue::Bool(b)) => b,
                _ => false,
            })
            .collect();
        return AttributeValue::BoolArray(bools);
    }

    let strings: Vec<String> = values
        .iter()
        .map(|v| parse_any_value(v).map(|av| av.as_display_string()).unwrap_or_else(|| v.to_string()))
        .collect();
    AttributeValue::StringArray(strings)
}

fn parse_events(events: &[Value]) -> Vec<SpanEvent> {
    events
        .iter()
        .filter_map(|e| {
            let name = e.get("name").and_then(|n| n.as_str())?.to_string();
            let timestamp_ns = e
                .get("timeUnixNano")
                .and_then(parse_nano_ts)
                .unwrap_or(0);
            let attributes = parse_attributes(
                e.get("attributes").and_then(|a| a.as_array()).unwrap_or(&vec![]),
            );
            Some(SpanEvent { name, timestamp_ns, attributes })
        })
        .collect()
}
