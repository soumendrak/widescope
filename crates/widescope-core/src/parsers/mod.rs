pub mod jaeger;
pub mod openinference;
pub mod otlp_json;

use crate::errors::WideError;
use crate::models::trace::InputFormat;
use serde_json::Value;

pub fn read_input(raw_input: &str) -> Result<Value, WideError> {
    let raw_input = raw_input.strip_prefix('\u{feff}').unwrap_or(raw_input);
    let whole_error = match serde_json::from_str(raw_input) {
        Ok(value) => return Ok(value),
        Err(error) => error,
    };
    let lines: Vec<(usize, &str)> = raw_input
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty())
        .collect();
    let one_record_per_line = lines.iter().all(|(_, line)| line.starts_with('{'));
    if lines.len() < 2 || !one_record_per_line {
        return Err(invalid_json(&whole_error, None));
    }
    let mut resource_spans = Vec::new();
    for (number, line) in lines {
        let mut value: Value =
            serde_json::from_str(line).map_err(|error| invalid_json(&error, Some(number)))?;
        let Some(Value::Array(batch)) = value
            .as_object_mut()
            .and_then(|object| object.remove("resourceSpans"))
        else {
            return Err(WideError::InvalidJson {
                message: format!("line {number}: not an OTLP resourceSpans document"),
                line: Some(number),
                column: Some(1),
            });
        };
        resource_spans.extend(batch);
    }
    let mut merged = serde_json::Map::new();
    merged.insert("resourceSpans".to_owned(), Value::Array(resource_spans));
    Ok(Value::Object(merged))
}

fn invalid_json(error: &serde_json::Error, ndjson_line: Option<usize>) -> WideError {
    match ndjson_line {
        None => WideError::InvalidJson {
            message: error.to_string(),
            line: Some(error.line()),
            column: Some(error.column()),
        },
        Some(number) => {
            let text = error.to_string();
            let reason = text
                .rsplit_once(" at line ")
                .map_or(text.as_str(), |(reason, _)| reason);
            WideError::InvalidJson {
                message: format!("line {number}: {reason}"),
                line: Some(number),
                column: Some(error.column()),
            }
        }
    }
}

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
}
