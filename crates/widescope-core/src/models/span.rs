use crate::models::llm::LlmSpanAttributes;
use crate::models::safety::SafetySignal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    #[serde(default)]
    pub safety: Vec<SafetySignal>,
}

impl Span {
    /// Highest-severity safety category on this span, for badge/coloring.
    pub fn top_safety_category(&self) -> Option<String> {
        self.safety
            .iter()
            .max_by_key(|s| s.severity)
            .map(|s: &SafetySignal| s.category.as_str().to_string())
    }
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

// PartialEq so tests can assert a decoded attribute equals what the export
// carried, rather than comparing display strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::safety::{SafetyCategory, SafetySeverity};

    fn signal(category: SafetyCategory, severity: SafetySeverity) -> SafetySignal {
        SafetySignal {
            category,
            severity,
            detail: String::new(),
        }
    }

    #[test]
    fn the_worst_safety_signal_wins_the_badge() {
        let mut span = Span {
            trace_id: "t".into(),
            span_id: "s".into(),
            parent_span_id: None,
            operation_name: "op".into(),
            service_name: "svc".into(),
            span_kind: SpanKind::Internal,
            start_time_ns: 0,
            end_time_ns: 1,
            duration_ns: 1,
            self_time_ns: 1,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: vec![],
            llm: None,
            safety: vec![],
        };
        assert_eq!(span.top_safety_category(), None);

        span.safety = vec![
            signal(SafetyCategory::Refusal, SafetySeverity::Low),
            signal(SafetyCategory::Jailbreak, SafetySeverity::High),
        ];
        assert_eq!(span.top_safety_category().as_deref(), Some("jailbreak"));
    }

    #[test]
    fn every_attribute_value_has_a_display_form() {
        let cases: Vec<(AttributeValue, &str)> = vec![
            (AttributeValue::String("hi".into()), "hi"),
            (AttributeValue::Int(-2), "-2"),
            (AttributeValue::Float(1.5), "1.5"),
            (AttributeValue::Bool(true), "true"),
            (
                AttributeValue::StringArray(vec!["a".into(), "b".into()]),
                "[a, b]",
            ),
            (AttributeValue::IntArray(vec![1, 2]), "[1, 2]"),
            (AttributeValue::FloatArray(vec![1.5, 2.5]), "[1.5, 2.5]"),
            (AttributeValue::BoolArray(vec![true, false]), "[true, false]"),
        ];
        for (value, expected) in cases {
            assert_eq!(value.as_display_string(), expected);
        }
    }

    #[test]
    fn typed_accessors_only_answer_for_their_own_variant() {
        assert_eq!(AttributeValue::String("x".into()).as_str(), Some("x"));
        assert_eq!(AttributeValue::Int(1).as_str(), None);

        assert_eq!(AttributeValue::Int(3).as_int(), Some(3));
        assert_eq!(AttributeValue::Float(3.0).as_int(), None);

        // Floats accept ints, because token counts arrive as either.
        assert_eq!(AttributeValue::Float(2.5).as_float(), Some(2.5));
        assert_eq!(AttributeValue::Int(2).as_float(), Some(2.0));
        assert_eq!(AttributeValue::Bool(true).as_float(), None);
    }

    #[test]
    fn span_kinds_and_statuses_have_stable_names() {
        for (kind, name) in [
            (SpanKind::Internal, "Internal"),
            (SpanKind::Server, "Server"),
            (SpanKind::Client, "Client"),
            (SpanKind::Producer, "Producer"),
            (SpanKind::Consumer, "Consumer"),
        ] {
            assert_eq!(kind.as_str(), name);
        }

        assert_eq!(SpanStatus::Unset.as_str(), "Unset");
        assert_eq!(SpanStatus::Ok.as_str(), "Ok");
        assert!(!SpanStatus::Ok.is_error());
        assert_eq!(SpanStatus::Ok.error_message(), None);

        let err = SpanStatus::Error { message: "bad".into() };
        assert!(err.is_error());
        assert_eq!(err.error_message(), Some("bad"));
    }
}

