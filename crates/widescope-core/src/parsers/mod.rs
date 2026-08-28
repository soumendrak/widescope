pub mod jaeger;
pub mod openinference;
pub mod otlp_json;

use crate::errors::WideError;
use crate::models::span::Span;
use crate::models::trace::InputFormat;
use serde_json::Value;

pub fn detect_format(value: &Value) -> Result<InputFormat, WideError> {
    if value.get("resourceSpans").is_some() {
        return Ok(InputFormat::OtlpJson);
    }

    if let Some(data) = value.get("data") {
        if let Some(arr) = data.as_array() {
            if arr.first().and_then(|v| v.get("traceID")).is_some() {
                return Ok(InputFormat::JaegerJson);
            }
        }
    }

    if let Some(arr) = value.get("spans").and_then(|v| v.as_array()) {
        if let Some(first) = arr.first() {
            if first
                .get("context")
                .and_then(|c| c.get("trace_id"))
                .is_some()
            {
                return Ok(InputFormat::OpenInferenceJson);
            }
        }
    }

    Err(WideError::UnrecognizedFormat)
}

#[allow(dead_code)]
pub fn parse(raw_input: &str) -> Result<(Vec<Span>, InputFormat), WideError> {
    let value: Value = serde_json::from_str(raw_input).map_err(|e| WideError::InvalidJson {
        message: e.to_string(),
        line: Some(e.line()),
        column: Some(e.column()),
    })?;

    let format = detect_format(&value)?;

    let spans = match &format {
        InputFormat::OtlpJson => otlp_json::parse_otlp(&value)?,
        InputFormat::JaegerJson => jaeger::parse_jaeger(&value)?,
        InputFormat::OpenInferenceJson => {
            let result = openinference::parse_openinference_with_warnings(&value)?;
            result.spans
        }
        InputFormat::Unknown => return Err(WideError::UnrecognizedFormat),
    };

    Ok((spans, format))
}

/// Format sniffing and the raw-string entry point.
///
/// Detection runs before any parser sees the payload, so a wrong answer here
/// shows up as "unrecognized format" on a perfectly good trace.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const OTLP: &str = include_str!("../../../../test-fixtures/otlp/sample_llm_pipeline.json");
    const JAEGER: &str = include_str!("../../../../test-fixtures/jaeger/sample_llm_pipeline.json");
    const OI: &str =
        include_str!("../../../../test-fixtures/openinference/sample_llm_pipeline.json");

    fn detect(raw: &str) -> InputFormat {
        detect_format(&serde_json::from_str(raw).unwrap()).unwrap()
    }

    #[test]
    fn each_bundled_fixture_is_detected_as_its_own_format() {
        assert!(matches!(detect(OTLP), InputFormat::OtlpJson));
        assert!(matches!(detect(JAEGER), InputFormat::JaegerJson));
        assert!(matches!(detect(OI), InputFormat::OpenInferenceJson));
    }

    #[test]
    fn detection_keys_on_shape_not_just_the_top_level_name() {
        // `data` alone is not Jaeger — the first entry must carry a traceID.
        assert!(detect_format(&json!({"data": []})).is_err());
        assert!(detect_format(&json!({"data": [{"nope": 1}]})).is_err());
        assert!(detect_format(&json!({"data": {"traceID": "x"}})).is_err());

        // `spans` alone is not OpenInference — the first span needs a context.
        assert!(detect_format(&json!({"spans": []})).is_err());
        assert!(detect_format(&json!({"spans": [{"name": "x"}]})).is_err());
        assert!(detect_format(&json!({"spans": [{"context": {}}]})).is_err());
    }

    #[test]
    fn anything_else_is_rejected() {
        for value in [json!({}), json!([]), json!({"hello": "world"}), json!(null)] {
            assert!(matches!(
                detect_format(&value),
                Err(WideError::UnrecognizedFormat)
            ));
        }
    }

    #[test]
    fn parse_reads_a_raw_string_in_every_format() {
        for (raw, expected) in [
            (OTLP, InputFormat::OtlpJson),
            (JAEGER, InputFormat::JaegerJson),
            (OI, InputFormat::OpenInferenceJson),
        ] {
            let (spans, format) = parse(raw).unwrap();
            assert_eq!(spans.len(), 7);
            assert_eq!(format.as_str(), expected.as_str());
        }
    }

    #[test]
    fn parse_reports_where_invalid_json_broke() {
        match parse("{\"resourceSpans\": [").unwrap_err() {
            WideError::InvalidJson { line, .. } => assert_eq!(line, Some(1)),
            other => panic!("expected InvalidJson, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_an_unrecognized_payload() {
        assert!(matches!(
            parse("{\"hello\":1}").unwrap_err(),
            WideError::UnrecognizedFormat
        ));
    }
}
