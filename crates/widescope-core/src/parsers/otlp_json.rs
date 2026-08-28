use crate::errors::WideError;
use crate::models::span::{AttributeValue, Span, SpanEvent, SpanKind, SpanStatus};
use crate::models::trace::ParseWarning;
use serde_json::Value;
use std::collections::HashMap;

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
        warnings.push(ParseWarning::new("SPAN_MISSING_REQUIRED", msg).with_count(skip_count));
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

    let kind_int = raw.get("kind").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let span_kind = SpanKind::from_otlp_int(kind_int);

    let start_time_ns: u64 = raw
        .get("startTimeUnixNano")
        .and_then(parse_nano_ts)
        .ok_or_else(|| format!("span {}: missing startTimeUnixNano", span_id))?;

    let end_time_ns: u64 = raw
        .get("endTimeUnixNano")
        .and_then(parse_nano_ts)
        .ok_or_else(|| format!("span {}: missing endTimeUnixNano", span_id))?;

    // Inverted clocks are left as parsed: trace_builder normalises them and is
    // the only place that can warn about it. Swapping here made that warning
    // unreachable, so a trace with broken timestamps looked clean.
    let duration_ns = end_time_ns.saturating_sub(start_time_ns);

    let status = parse_status(raw);

    let attributes = parse_attributes(
        raw.get("attributes")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]),
    );

    let events = parse_events(
        raw.get("events")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]),
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
        safety: Vec::new(),
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
    let raw_code = raw.get("status").and_then(|s| s.get("code"));
    // Proto3 JSON allows an enum to travel as its number or its name, and real
    // exporters use both. Reading only the number silently downgraded every
    // errored span to Unset.
    let code = match raw_code {
        Some(Value::String(name)) => match name.as_str() {
            "STATUS_CODE_OK" => 1,
            "STATUS_CODE_ERROR" => 2,
            _ => 0,
        },
        other => other.and_then(|c| c.as_u64()).unwrap_or(0),
    };

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

    let parsed: Vec<Option<AttributeValue>> = values.iter().map(parse_any_value).collect();

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

    let all_float = parsed.iter().all(|v| {
        matches!(
            v,
            Some(AttributeValue::Float(_)) | Some(AttributeValue::Int(_))
        )
    });
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
        .map(|v| {
            parse_any_value(v)
                .map(|av| av.as_display_string())
                .unwrap_or_else(|| v.to_string())
        })
        .collect();
    AttributeValue::StringArray(strings)
}

fn parse_events(events: &[Value]) -> Vec<SpanEvent> {
    events
        .iter()
        .filter_map(|e| {
            let name = e.get("name").and_then(|n| n.as_str())?.to_string();
            let timestamp_ns = e.get("timeUnixNano").and_then(parse_nano_ts).unwrap_or(0);
            let attributes = parse_attributes(
                e.get("attributes")
                    .and_then(|a| a.as_array())
                    .unwrap_or(&vec![]),
            );
            Some(SpanEvent {
                name,
                timestamp_ns,
                attributes,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_pipeline_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-fixtures/otlp/sample_llm_pipeline.json"
        );
        let raw = std::fs::read_to_string(path).expect("fixture file should exist");
        let value: Value = serde_json::from_str(&raw).expect("fixture should be valid JSON");

        let result = parse_otlp_with_warnings(&value).expect("should parse sample pipeline");

        assert_eq!(result.spans.len(), 7);

        let services: std::collections::HashSet<_> = result
            .spans
            .iter()
            .map(|s| s.service_name.as_str())
            .collect();
        assert_eq!(services.len(), 3, "gateway, rag-retriever, llm-service");

        // The `chat` LLM span carries gen_ai token usage (raw attrs; resolution
        // into Span.llm happens later in the pipeline).
        let chat = result
            .spans
            .iter()
            .find(|s| s.operation_name == "chat")
            .expect("chat span present");
        assert!(matches!(
            chat.attributes.get("gen_ai.usage.input_tokens"),
            Some(AttributeValue::Int(_))
        ));

        // Exactly one root (POST /api/chat has no in-trace parent).
        let roots = result
            .spans
            .iter()
            .filter(|s| s.parent_span_id.is_none())
            .count();
        assert_eq!(roots, 1);
    }

    #[test]
    fn rejects_non_otlp_shape() {
        let value = serde_json::json!({"not": "otlp"});
        assert!(parse_otlp(&value).unwrap_or_default().is_empty());
    }
}

/// Attribute, timestamp and status decoding, plus the skip/reject paths.
///
/// OTLP's `anyValue` union is where malformed exports hurt most: a wrong arm
/// here silently drops a field rather than failing, so every arm gets a case.
#[cfg(test)]
mod value_tests {
    use super::*;
    use serde_json::json;

    fn attrs(list: serde_json::Value) -> HashMap<String, AttributeValue> {
        parse_attributes(list.as_array().unwrap())
    }

    #[test]
    fn every_any_value_arm_decodes() {
        assert_eq!(
            parse_any_value(&json!({"stringValue": "hi"})),
            Some(AttributeValue::String("hi".into()))
        );
        assert_eq!(
            parse_any_value(&json!({"intValue": 7})),
            Some(AttributeValue::Int(7))
        );
        // Proto3 JSON encodes 64-bit ints as strings.
        assert_eq!(
            parse_any_value(&json!({"intValue": "9007199254740993"})),
            Some(AttributeValue::Int(9007199254740993))
        );
        assert_eq!(
            parse_any_value(&json!({"doubleValue": 1.5})),
            Some(AttributeValue::Float(1.5))
        );
        assert_eq!(
            parse_any_value(&json!({"boolValue": true})),
            Some(AttributeValue::Bool(true))
        );
        assert_eq!(
            parse_any_value(&json!({"bytesValue": "AAEC"})),
            Some(AttributeValue::String("base64:AAEC".into()))
        );
        assert!(matches!(
            parse_any_value(&json!({"kvlistValue": {"values": []}})),
            Some(AttributeValue::String(_))
        ));
        assert_eq!(parse_any_value(&json!({})), None);
        assert_eq!(parse_any_value(&json!({"intValue": "not-a-number"})), None);
    }

    #[test]
    fn arrays_collapse_to_the_narrowest_matching_type() {
        let int_arr =
            parse_any_value(&json!({"arrayValue": {"values": [{"intValue": 1}, {"intValue": 2}]}}));
        assert_eq!(int_arr, Some(AttributeValue::IntArray(vec![1, 2])));

        // A mixed int/double array widens to floats rather than losing the decimals.
        let float_arr = parse_any_value(
            &json!({"arrayValue": {"values": [{"intValue": 1}, {"doubleValue": 2.5}]}}),
        );
        assert_eq!(float_arr, Some(AttributeValue::FloatArray(vec![1.0, 2.5])));

        let str_arr = parse_any_value(
            &json!({"arrayValue": {"values": [{"stringValue": "a"}, {"boolValue": false}]}}),
        );
        assert_eq!(
            str_arr,
            Some(AttributeValue::StringArray(vec![
                "a".into(),
                "false".into()
            ]))
        );

        assert_eq!(
            parse_any_value(&json!({"arrayValue": {"values": []}})),
            Some(AttributeValue::StringArray(vec![]))
        );
        // An arrayValue with no `values` key is not an array at all.
        assert_eq!(parse_any_value(&json!({"arrayValue": {}})), None);
    }

    #[test]
    fn attributes_skip_entries_without_a_usable_key_or_value() {
        let map = attrs(json!([
            {"key": "a", "value": {"stringValue": "1"}},
            {"key": "b"},
            {"value": {"stringValue": "orphan"}},
            {"key": "c", "value": {}},
        ]));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("a"), Some(&AttributeValue::String("1".into())));
    }

    #[test]
    fn timestamps_accept_both_json_encodings() {
        assert_eq!(
            parse_nano_ts(&json!("1700000000000000000")),
            Some(1_700_000_000_000_000_000)
        );
        assert_eq!(
            parse_nano_ts(&json!(1_700_000_000_000_000_000u64)),
            Some(1_700_000_000_000_000_000)
        );
        assert_eq!(parse_nano_ts(&json!("nope")), None);
        assert_eq!(parse_nano_ts(&json!(null)), None);
    }

    #[test]
    fn status_maps_every_code() {
        assert_eq!(
            parse_status(&json!({"status": {"code": 0}})).as_str(),
            "Unset"
        );
        assert_eq!(parse_status(&json!({"status": {"code": 1}})).as_str(), "Ok");
        assert_eq!(
            parse_status(&json!({"status": {"code": 2}})).as_str(),
            "Error"
        );
        assert_eq!(
            parse_status(&json!({"status": {"code": 99}})).as_str(),
            "Unset"
        );
        assert_eq!(parse_status(&json!({})).as_str(), "Unset");
        // Proto3 JSON may spell the code out.
        assert_eq!(
            parse_status(&json!({"status": {"code": "STATUS_CODE_ERROR"}})).as_str(),
            "Error"
        );
        // An error status carries its message through to the inspector.
        assert_eq!(
            parse_status(&json!({"status": {"code": 2, "message": "boom"}})).error_message(),
            Some("boom")
        );
    }

    #[test]
    fn events_carry_their_timestamp_and_attributes() {
        let events = parse_events(
            json!([
                {"name": "exception", "timeUnixNano": "5", "attributes": [
                    {"key": "exception.type", "value": {"stringValue": "IOError"}}
                ]},
                {"timeUnixNano": "6"},
            ])
            .as_array()
            .unwrap(),
        );
        // A nameless event carries nothing worth showing, so it is dropped.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "exception");
        assert_eq!(events[0].timestamp_ns, 5);
        assert_eq!(events[0].attributes.len(), 1);
    }

    #[test]
    fn service_name_falls_back_when_the_resource_is_absent() {
        assert_eq!(extract_service_name(&json!({})), "unknown_service");
        assert_eq!(
            extract_service_name(&json!({"resource": {"attributes": [
                {"key": "service.name", "value": {"stringValue": "checkout"}}
            ]}})),
            "checkout"
        );
    }

    fn span_json() -> serde_json::Value {
        json!({
            "traceId": "0123456789abcdef0123456789abcdef",
            "spanId": "0123456789abcdef",
            "name": "op",
            "startTimeUnixNano": "10",
            "endTimeUnixNano": "20",
        })
    }

    #[test]
    fn a_span_missing_any_required_field_is_reported_not_dropped_silently() {
        for missing in ["traceId", "spanId", "startTimeUnixNano", "endTimeUnixNano"] {
            let mut raw = span_json();
            raw.as_object_mut().unwrap().remove(missing);
            let err = parse_single_span(&raw, "svc").unwrap_err();
            assert!(err.contains(missing), "{missing} should be named in {err}");
        }
    }

    #[test]
    fn span_kinds_map_across_the_whole_enum() {
        for (code, expected) in [
            (1, SpanKind::Internal),
            (2, SpanKind::Server),
            (3, SpanKind::Client),
            (4, SpanKind::Producer),
            (5, SpanKind::Consumer),
            (0, SpanKind::Internal),
        ] {
            let mut raw = span_json();
            raw["kind"] = json!(code);
            let span = parse_single_span(&raw, "svc").unwrap();
            assert_eq!(span.span_kind.as_str(), expected.as_str(), "kind {code}");
        }
    }

    #[test]
    fn a_parent_span_id_is_optional_and_blank_means_root() {
        let mut raw = span_json();
        raw["parentSpanId"] = json!("");
        assert_eq!(parse_single_span(&raw, "svc").unwrap().parent_span_id, None);

        raw["parentSpanId"] = json!("fedcba9876543210");
        assert_eq!(
            parse_single_span(&raw, "svc")
                .unwrap()
                .parent_span_id
                .as_deref(),
            Some("fedcba9876543210")
        );
    }

    #[test]
    fn skipped_spans_produce_one_counted_warning() {
        let doc = json!({"resourceSpans": [{"scopeSpans": [{"spans": [
            span_json(),
            {"spanId": "1", "name": "no-trace-id"},
            {"traceId": "2", "name": "no-span-id"},
        ]}]}]});
        let result = parse_otlp_with_warnings(&doc).unwrap();
        assert_eq!(result.spans.len(), 1);
        let warning = &result.warnings[0];
        assert_eq!(warning.code, "SPAN_MISSING_REQUIRED");
        assert_eq!(warning.count, 2);
    }

    #[test]
    fn a_document_with_no_usable_span_is_an_error() {
        let doc = json!({"resourceSpans": [{"scopeSpans": [{"spans": [{"name": "junk"}]}]}]});
        let err = parse_otlp_with_warnings(&doc)
            .err()
            .expect("no usable span");
        assert!(matches!(err, WideError::NoValidSpans { .. }));
    }

    #[test]
    fn missing_scope_spans_and_empty_documents_are_tolerated() {
        assert!(parse_otlp_with_warnings(&json!({"resourceSpans": []})).is_err());
        assert!(parse_otlp_with_warnings(&json!({"resourceSpans": [{}]})).is_err());
        assert!(
            parse_otlp_with_warnings(&json!({"resourceSpans": [{"scopeSpans": [{}]}]})).is_err()
        );
    }

    #[test]
    fn parse_otlp_returns_spans_without_the_warning_envelope() {
        let doc = json!({"resourceSpans": [{"scopeSpans": [{"spans": [span_json()]}]}]});
        assert_eq!(parse_otlp(&doc).unwrap().len(), 1);
    }
}
