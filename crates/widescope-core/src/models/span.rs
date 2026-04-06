use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::models::llm::LlmSpanAttributes;

pub type Timestamp = u64;
pub type Duration = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub span_kind: SpanKind,
    pub start_time_ns: Timestamp,
    pub end_time_ns: Timestamp,
    pub duration_ns: Duration,
    pub self_time_ns: Duration,
    pub status: SpanStatus,
    pub attributes: HashMap<String, AttributeValue>,
    pub events: Vec<SpanEvent>,
    pub llm: Option<LlmSpanAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

impl SpanKind {
    pub fn from_otlp_int(v: u8) -> Self {
        match v {
            1 => SpanKind::Internal,
            2 => SpanKind::Server,
            3 => SpanKind::Client,
            4 => SpanKind::Producer,
            5 => SpanKind::Consumer,
            _ => SpanKind::Internal,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SpanKind::Internal => "Internal",
            SpanKind::Server => "Server",
            SpanKind::Client => "Client",
            SpanKind::Producer => "Producer",
            SpanKind::Consumer => "Consumer",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum SpanStatus {
    Unset,
    Ok,
    Error { message: String },
}

impl SpanStatus {
    pub fn is_error(&self) -> bool {
        matches!(self, SpanStatus::Error { .. })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SpanStatus::Unset => "Unset",
            SpanStatus::Ok => "Ok",
            SpanStatus::Error { .. } => "Error",
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            SpanStatus::Error { message } => Some(message.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
    BoolArray(Vec<bool>),
}

impl AttributeValue {
    pub fn as_display_string(&self) -> String {
        match self {
            AttributeValue::String(s) => s.clone(),
            AttributeValue::Int(i) => i.to_string(),
            AttributeValue::Float(f) => f.to_string(),
            AttributeValue::Bool(b) => b.to_string(),
            AttributeValue::StringArray(arr) => {
                format!("[{}]", arr.join(", "))
            }
            AttributeValue::IntArray(arr) => {
                let s: Vec<String> = arr.iter().map(|i| i.to_string()).collect();
                format!("[{}]", s.join(", "))
            }
            AttributeValue::FloatArray(arr) => {
                let s: Vec<String> = arr.iter().map(|f| f.to_string()).collect();
                format!("[{}]", s.join(", "))
            }
            AttributeValue::BoolArray(arr) => {
                let s: Vec<String> = arr.iter().map(|b| b.to_string()).collect();
                format!("[{}]", s.join(", "))
            }
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            AttributeValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            AttributeValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            AttributeValue::Float(f) => Some(*f),
            AttributeValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp_ns: Timestamp,
    pub attributes: HashMap<String, AttributeValue>,
}
