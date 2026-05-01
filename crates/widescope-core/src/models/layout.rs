use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

pub fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameNode {
    pub span_id: String,
    pub label: String,
    pub x: f64,
    pub width: f64,
    pub depth: u32,
    pub color_key: String,
    pub is_error: bool,
    pub is_llm: bool,
    pub duration_ns: u64,
    pub self_time_ns: u64,
    pub duration_display: String,
    pub self_time_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphLayout {
    pub nodes: Vec<FlameNode>,
    pub max_depth: u32,
    pub trace_duration_ns: u64,
    pub trace_duration_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBlock {
    pub span_id: String,
    pub label: String,
    pub service_name: String,
    pub x_start: f64,
    pub x_end: f64,
    pub row_index: u32,
    pub is_error: bool,
    pub is_llm: bool,
    pub duration_ns: u64,
    pub duration_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineRow {
    pub service_name: String,
    pub row_index: u32,
    pub lane_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLayout {
    pub blocks: Vec<TimelineBlock>,
    pub rows: Vec<TimelineRow>,
    pub trace_duration_ns: u64,
    pub trace_duration_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanDetailResponse {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub span_kind: String,
    #[serde(serialize_with = "serialize_u64_as_string")]
    pub start_time_ns: u64,
    pub start_time_display: String,
    pub duration_ns: u64,
    pub duration_display: String,
    pub self_time_ns: u64,
    pub self_time_display: String,
    pub status: String,
    pub error_message: Option<String>,
    pub attributes: Vec<(String, String)>,
    pub events: Vec<EventDetail>,
    pub llm: Option<LlmDetail>,
    pub children_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDetail {
    pub name: String,
    #[serde(serialize_with = "serialize_u64_as_string")]
    pub timestamp_ns: u64,
    pub timestamp_display: String,
    pub attributes: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDetail {
    pub operation_type: String,
    pub model_name: Option<String>,
    pub model_provider: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub temperature: Option<f64>,
    pub input_messages: Vec<MessageDetail>,
    pub output_messages: Vec<MessageDetail>,
    pub tool_calls: Vec<ToolCallDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDetail {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDetail {
    pub name: String,
    pub arguments: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallRow {
    pub span_id: String,
    pub operation_name: String,
    pub service_name: String,
    pub span_kind: String,
    pub depth: u32,
    pub x_start: f64, // normalized [0, 1]
    pub x_end: f64,   // normalized [0, 1]
    pub color_key: String,
    pub is_error: bool,
    pub is_llm: bool,
    pub has_children: bool,
    pub duration_ns: u64,
    pub duration_display: String,
    pub self_time_ns: u64,
    pub self_time_display: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallLayout {
    pub rows: Vec<WaterfallRow>,
    pub trace_duration_ns: u64,
    pub trace_duration_display: String,
    pub max_depth: u32,
}
