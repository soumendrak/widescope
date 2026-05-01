use crate::errors::WideError;
use crate::models::span::{AttributeValue, Span, SpanEvent, SpanKind, SpanStatus};
use crate::models::trace::ParseWarning;
use serde_json::Value;
use std::collections::HashMap;

pub struct OiParseResult {
    pub spans: Vec<Span>,
    pub warnings: Vec<ParseWarning>,
}

pub fn parse_openinference_with_warnings(root: &Value) -> Result<OiParseResult, WideError> {
    let raw_spans = root
        .get("spans")
        .and_then(|v| v.as_array())
        .ok_or(WideError::UnrecognizedFormat)?;

    let mut spans: Vec<Span> = Vec::new();
    let mut warnings: Vec<ParseWarning> = Vec::new();
    let mut skip_count = 0usize;
    let mut skip_reasons: Vec<String> = Vec::new();

    for raw_span in raw_spans {
        match parse_single_span(raw_span) {
            Ok(span) => spans.push(span),
            Err(reason) => {
                skip_count += 1;
                if skip_reasons.len() < 5 {
                    skip_reasons.push(reason);
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
        warnings.push(ParseWarning::new("SPAN_MISSING_REQUIRED", msg).with_count(skip_count));
    }

    if spans.is_empty() {
        return Err(WideError::NoValidSpans {
            attempted: skip_count,
            failures: skip_reasons,
        });
    }

    Ok(OiParseResult { spans, warnings })
}

fn parse_single_span(raw: &Value) -> Result<Span, String> {
    let context = raw
        .get("context")
        .ok_or_else(|| "missing context".to_string())?;

    let trace_id = context
        .get("trace_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing context.trace_id".to_string())?
        .to_string();

    let span_id = context
        .get("span_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("span {}: missing context.span_id", trace_id))?
        .to_string();

    let operation_name = raw
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let parent_span_id = raw
        .get("parent_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let span_kind = raw
        .get("span_kind")
        .and_then(|v| v.as_str())
        .map(oi_span_kind_to_span_kind)
        .unwrap_or(SpanKind::Internal);

    let start_time_ns = raw
        .get("start_time_unix_nano")
        .and_then(parse_nano_ts)
        .ok_or_else(|| format!("span {}: missing start_time_unix_nano", span_id))?;

    let end_time_ns = raw
        .get("end_time_unix_nano")
        .and_then(parse_nano_ts)
        .ok_or_else(|| format!("span {}: missing end_time_unix_nano", span_id))?;

    let (start_time_ns, end_time_ns) = if start_time_ns <= end_time_ns {
        (start_time_ns, end_time_ns)
    } else {
        (end_time_ns, start_time_ns)
    };

    let duration_ns = end_time_ns - start_time_ns;

    let attributes = parse_oi_attributes(raw.get("attributes"));
    let events = parse_oi_events(raw.get("events"));

    let status = parse_oi_status(raw);

    let service_name = attributes
        .get("service.name")
        .and_then(AttributeValue::as_str)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown_service".to_string());

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

fn oi_span_kind_to_span_kind(kind: &str) -> SpanKind {
    match kind.to_ascii_uppercase().as_str() {
        "LLM" | "TOOL" | "RETRIEVER" => SpanKind::Client,
        _ => SpanKind::Internal,
    }
}

fn parse_nano_ts(v: &Value) -> Option<u64> {
    if let Some(s) = v.as_str() {
        s.parse::<u64>().ok()
    } else if let Some(n) = v.as_u64() {
        Some(n)
    } else if let Some(n) = v.as_i64() {
        u64::try_from(n).ok()
    } else {
        v.as_f64().map(|f| f as u64)
    }
}

fn parse_oi_status(raw: &Value) -> SpanStatus {
    let code = raw
        .get("status_code")
        .and_then(|v| v.as_str())
        .unwrap_or("UNSET")
        .to_ascii_uppercase();

    match code.as_str() {
        "OK" => SpanStatus::Ok,
        "ERROR" => {
            let msg = raw
                .get("status_message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            SpanStatus::Error { message: msg }
        }
        _ => SpanStatus::Unset,
    }
}

fn parse_oi_events(raw: Option<&Value>) -> Vec<SpanEvent> {
    let events = match raw.and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    events
        .iter()
        .map(|e| {
            let name = e
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp_ns = e
                .get("timestamp_unix_nano")
                .and_then(parse_nano_ts)
                .or_else(|| {
                    e.get("timestamp")
                        .and_then(|v| v.as_str())
                        .and_then(parse_iso_8601_ns)
                })
                .unwrap_or(0);

            let attributes = e
                .get("attributes")
                .map_or_else(HashMap::new, |a| parse_oi_attributes(Some(a)));

            SpanEvent {
                name,
                timestamp_ns,
                attributes,
            }
        })
        .collect()
}

fn parse_oi_attributes(raw: Option<&Value>) -> HashMap<String, AttributeValue> {
    let obj = match raw.and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return HashMap::new(),
    };

    let mut map = HashMap::new();
    for (key, value) in obj {
        if let Some(av) = json_value_to_attr_value(value) {
            map.insert(key.clone(), av);
        }
    }
    map
}

fn json_value_to_attr_value(value: &Value) -> Option<AttributeValue> {
    match value {
        Value::String(s) => Some(AttributeValue::String(s.clone())),
        Value::Number(n) => n
            .as_i64()
            .map(AttributeValue::Int)
            .or_else(|| n.as_f64().map(AttributeValue::Float)),
        Value::Bool(b) => Some(AttributeValue::Bool(*b)),
        Value::Array(arr) => {
            if arr.is_empty() {
                return Some(AttributeValue::StringArray(vec![]));
            }

            let all_int = arr.iter().all(|v| v.is_i64());
            if all_int {
                let ints: Vec<i64> = arr.iter().map(|v| v.as_i64().unwrap_or(0)).collect();
                return Some(AttributeValue::IntArray(ints));
            }

            let all_float = arr.iter().all(|v| v.is_f64() || v.is_i64());
            if all_float {
                let floats: Vec<f64> = arr.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect();
                return Some(AttributeValue::FloatArray(floats));
            }

            let all_bool = arr.iter().all(|v| v.is_boolean());
            if all_bool {
                let bools: Vec<bool> = arr.iter().map(|v| v.as_bool().unwrap_or(false)).collect();
                return Some(AttributeValue::BoolArray(bools));
            }

            let strings: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            Some(AttributeValue::StringArray(strings))
        }
        Value::Object(_) => Some(AttributeValue::String(value.to_string())),
        Value::Null => None,
    }
}

fn parse_iso_8601_ns(ts: &str) -> Option<u64> {
    // Try parsing as RFC 3339 / ISO 8601 with nanosecond precision
    // Format: "2024-04-07T17:13:20.850000Z"
    let (date_part, time_part) = ts.split_once('T')?;

    let time_part = time_part.strip_suffix('Z')?;

    // Parse date: YYYY-MM-DD
    let mut dp = date_part.split('-');
    let year: i64 = dp.next()?.parse().ok()?;
    let month: i64 = dp.next()?.parse().ok()?;
    let day: i64 = dp.next()?.parse().ok()?;

    // Parse time: HH:MM:SS.fffffffff (nanoseconds fraction optional)
    let (time, frac) = match time_part.split_once('.') {
        Some((t, f)) => (t, f),
        None => (time_part, "0"),
    };

    let mut tp = time.split(':');
    let hour: i64 = tp.next()?.parse().ok()?;
    let minute: i64 = tp.next()?.parse().ok()?;
    let second: i64 = tp.next()?.parse().ok()?;

    // Parse fractional seconds as nanoseconds
    let nanos_frac: u64 = if frac.len() >= 9 {
        frac[..9].parse().ok()?
    } else {
        let parsed: u64 = frac.parse().ok()?;
        parsed * 10u64.pow((9 - frac.len()) as u32)
    };

    // Calculate days since Unix epoch (simple but works for post-2000 dates)
    let days = days_since_epoch(year, month, day)?;
    let total_seconds = days as i64 * 86400 + hour * 3600 + minute * 60 + second;

    if total_seconds < 0 {
        return None;
    }

    Some(total_seconds as u64 * 1_000_000_000 + nanos_frac)
}

fn days_since_epoch(year: i64, month: i64, day: i64) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let (y, m) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };

    let era = if y >= 0 {
        (y / 400) as u64
    } else {
        ((y - 399) / 400) as u64
    };
    let yoe = (y - (era as i64) * 400) as u64;
    let doy = (153 * (m as u64 - 3) + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

    // Total days from March 1, year 0
    let total_days = era * 146097 + doe;

    // Days from March 1, year 0 to January 1, 1970 = 719468
    const UNIX_EPOCH_MARCH1: u64 = 719468;
    if total_days < UNIX_EPOCH_MARCH1 {
        return None;
    }

    Some((total_days - UNIX_EPOCH_MARCH1) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_basic_openinference_trace() {
        let input = json!({
            "spans": [
                {
                    "name": "agent.session",
                    "context": {
                        "trace_id": "abc123",
                        "span_id": "root-001"
                    },
                    "parent_id": null,
                    "span_kind": "AGENT",
                    "start_time_unix_nano": "1712510000000000000",
                    "end_time_unix_nano": "1712510004800000000",
                    "status_code": "OK",
                    "attributes": {
                        "openinference.span.kind": "AGENT",
                        "session.id": "sess-001"
                    },
                    "events": [
                        {
                            "name": "session.init",
                            "timestamp": "2024-04-07T17:13:20.500000Z",
                            "attributes": {
                                "step": 1
                            }
                        }
                    ]
                },
                {
                    "name": "llm.answer",
                    "context": {
                        "trace_id": "abc123",
                        "span_id": "llm-002"
                    },
                    "parent_id": "root-001",
                    "span_kind": "LLM",
                    "start_time_unix_nano": "1712510001000000000",
                    "end_time_unix_nano": "1712510003000000000",
                    "status_code": "OK",
                    "attributes": {
                        "openinference.span.kind": "LLM",
                        "llm.model_name": "gpt-4",
                        "llm.token_count.prompt": 100,
                        "llm.token_count.completion": 50
                    },
                    "events": []
                },
                {
                    "name": "tool.search",
                    "context": {
                        "trace_id": "abc123",
                        "span_id": "tool-003"
                    },
                    "parent_id": "llm-002",
                    "span_kind": "TOOL",
                    "start_time_unix_nano": "1712510002000000000",
                    "end_time_unix_nano": "1712510002500000000",
                    "status_code": "ERROR",
                    "status_message": "timeout",
                    "attributes": {
                        "openinference.span.kind": "TOOL",
                        "tool.name": "search"
                    },
                    "events": []
                }
            ]
        });

        let result =
            parse_openinference_with_warnings(&input).expect("should parse openinference trace");

        assert_eq!(result.spans.len(), 3);
        assert!(result.warnings.is_empty());

        let root = result
            .spans
            .iter()
            .find(|s| s.span_id == "root-001")
            .unwrap();
        assert_eq!(root.operation_name, "agent.session");
        assert!(matches!(root.span_kind, SpanKind::Internal));
        assert!(matches!(root.status, SpanStatus::Ok));
        assert!(root.parent_span_id.is_none());
        assert_eq!(root.events.len(), 1);
        assert_eq!(root.events[0].name, "session.init");

        let llm = result
            .spans
            .iter()
            .find(|s| s.span_id == "llm-002")
            .unwrap();
        assert_eq!(llm.parent_span_id.as_deref(), Some("root-001"));
        assert!(matches!(llm.span_kind, SpanKind::Client));

        let tool = result
            .spans
            .iter()
            .find(|s| s.span_id == "tool-003")
            .unwrap();
        assert!(matches!(tool.status, SpanStatus::Error { .. }));
        assert_eq!(tool.status.error_message(), Some("timeout"));
    }

    #[test]
    fn parses_iso_timestamps() {
        let ns = parse_iso_8601_ns("2024-04-07T17:13:20.500000Z");
        assert_eq!(ns, Some(1712510000500000000));

        let ns = parse_iso_8601_ns("2024-04-07T17:13:20Z");
        assert_eq!(ns, Some(1712510000000000000));
    }

    #[test]
    fn parses_sample_pipeline_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-fixtures/openinference/sample_llm_pipeline.json"
        );
        let raw = std::fs::read_to_string(path).expect("fixture file should exist");
        let value: serde_json::Value =
            serde_json::from_str(&raw).expect("fixture should be valid JSON");

        let result =
            parse_openinference_with_warnings(&value).expect("should parse sample pipeline");

        assert_eq!(result.spans.len(), 7);
        assert!(result.warnings.is_empty());

        let root = result
            .spans
            .iter()
            .find(|s| s.span_id == "oi-root-001")
            .unwrap();
        assert_eq!(root.operation_name, "agent.session");
        assert!(root.parent_span_id.is_none());

        let tool = result
            .spans
            .iter()
            .find(|s| s.span_id == "oi-tool-006")
            .unwrap();
        assert!(matches!(tool.status, SpanStatus::Error { .. }));
        assert_eq!(
            tool.status.error_message(),
            Some("Pricing cache shard timed out; fallback used")
        );
        assert!(!tool.events.is_empty());
        assert_eq!(tool.events[0].name, "tool.retry");
    }

    #[test]
    fn parses_mixed_attributes() {
        let input = json!({
            "spans": [
                {
                    "name": "test",
                    "context": {
                        "trace_id": "t1",
                        "span_id": "s1"
                    },
                    "parent_id": null,
                    "span_kind": "CHAIN",
                    "start_time_unix_nano": "1000000",
                    "end_time_unix_nano": "2000000",
                    "status_code": "OK",
                    "attributes": {
                        "str_val": "hello",
                        "int_val": 42,
                        "float_val": 3.14,
                        "bool_val": true,
                        "str_array": ["a", "b", "c"],
                        "int_array": [1, 2, 3],
                        "float_array": [1.1, 2.2],
                        "nested_obj": {"key": "value"},
                        "null_val": null
                    },
                    "events": []
                }
            ]
        });

        let result = parse_openinference_with_warnings(&input).unwrap();
        assert_eq!(result.spans.len(), 1);

        let span = &result.spans[0];
        let attrs = &span.attributes;

        assert!(matches!(attrs.get("str_val"), Some(AttributeValue::String(s)) if s == "hello"));
        assert!(matches!(
            attrs.get("int_val"),
            Some(AttributeValue::Int(42))
        ));
        assert!(matches!(
            attrs.get("bool_val"),
            Some(AttributeValue::Bool(true))
        ));
        assert!(
            matches!(attrs.get("str_array"), Some(AttributeValue::StringArray(arr)) if arr.len() == 3)
        );
        assert!(
            matches!(attrs.get("int_array"), Some(AttributeValue::IntArray(arr)) if arr.len() == 3)
        );
        // null_val should be skipped
        assert!(!attrs.contains_key("null_val"));
    }
}
