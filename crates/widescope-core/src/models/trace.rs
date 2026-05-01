use crate::models::resource::Resource;
use crate::models::span::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: String,
    pub spans: Vec<Span>,
    pub span_index: HashMap<String, usize>,
    pub children_index: HashMap<String, Vec<String>>,
    pub resources: HashMap<String, Resource>,
    pub root_span_ids: Vec<String>,
    pub total_duration_ns: u64,
    pub span_count: usize,
    pub service_count: usize,
    pub has_errors: bool,
    pub detected_format: InputFormat,
    pub warnings: Vec<ParseWarning>,
}

impl Trace {
    pub fn get_span(&self, span_id: &str) -> Option<&Span> {
        self.span_index.get(span_id).map(|&idx| &self.spans[idx])
    }

    pub fn get_children(&self, span_id: &str) -> &[String] {
        self.children_index
            .get(span_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputFormat {
    OtlpJson,
    JaegerJson,
    OpenInferenceJson,
    Unknown,
}

impl InputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputFormat::OtlpJson => "OtlpJson",
            InputFormat::JaegerJson => "JaegerJson",
            InputFormat::OpenInferenceJson => "OpenInferenceJson",
            InputFormat::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseWarning {
    pub code: String,
    pub message: String,
    pub count: usize,
    pub context: Option<serde_json::Value>,
}

impl ParseWarning {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        ParseWarning {
            code: code.into(),
            message: message.into(),
            count: 1,
            context: None,
        }
    }

    pub fn with_context(mut self, ctx: serde_json::Value) -> Self {
        self.context = Some(ctx);
        self
    }

    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
}
