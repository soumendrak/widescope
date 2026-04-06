use thiserror::Error;
use wasm_bindgen::JsValue;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Error)]
pub enum WideError {
    #[error("Invalid JSON: {message}")]
    InvalidJson {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    #[error("Unrecognized trace format")]
    UnrecognizedFormat,

    #[error("No valid spans found in input")]
    NoValidSpans {
        attempted: usize,
        failures: Vec<String>,
    },

    #[error("No trace loaded")]
    NoTraceLoaded,

    #[error("Span not found: {span_id}")]
    SpanNotFound { span_id: String },

    #[error("Convention error: {message}")]
    ConventionError { message: String },
}

#[derive(Serialize)]
struct JsError {
    error_type: &'static str,
    code: &'static str,
    message: String,
    context: serde_json::Value,
}

impl From<WideError> for JsValue {
    fn from(err: WideError) -> JsValue {
        let (code, context) = match &err {
            WideError::InvalidJson { line, column, .. } => (
                "INVALID_JSON",
                json!({
                    "line": line,
                    "column": column,
                }),
            ),
            WideError::UnrecognizedFormat => ("UNRECOGNIZED_FORMAT", json!(null)),
            WideError::NoValidSpans { attempted, failures } => (
                "NO_VALID_SPANS",
                json!({
                    "attempted": attempted,
                    "failures": failures,
                }),
            ),
            WideError::NoTraceLoaded => ("NO_TRACE_LOADED", json!(null)),
            WideError::SpanNotFound { span_id } => (
                "SPAN_NOT_FOUND",
                json!({ "span_id": span_id }),
            ),
            WideError::ConventionError { .. } => ("CONVENTION_ERROR", json!(null)),
        };

        let js_err = JsError {
            error_type: "WideError",
            code,
            message: err.to_string(),
            context,
        };

        JsValue::from_str(&serde_json::to_string(&js_err).unwrap_or_else(|_| {
            r#"{"error_type":"WideError","code":"INTERNAL_ERROR","message":"serialization failed","context":null}"#.to_string()
        }))
    }
}
