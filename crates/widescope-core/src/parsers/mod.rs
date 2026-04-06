pub mod otlp_json;
pub mod jaeger;

use serde_json::Value;
use crate::errors::WideError;
use crate::models::span::Span;
use crate::models::trace::InputFormat;

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
            if first.get("context").and_then(|c| c.get("trace_id")).is_some() {
                return Ok(InputFormat::OpenInferenceJson);
            }
        }
    }

    Err(WideError::UnrecognizedFormat)
}

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
            return Err(WideError::UnrecognizedFormat);
        }
        InputFormat::Unknown => return Err(WideError::UnrecognizedFormat),
    };

    Ok((spans, format))
}
