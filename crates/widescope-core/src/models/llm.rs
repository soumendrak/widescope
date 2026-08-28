use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSpanAttributes {
    pub operation_type: LlmOperationType,
    pub model_name: Option<String>,
    pub model_provider: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub input_messages: Vec<LlmMessage>,
    pub output_messages: Vec<LlmMessage>,
    pub tool_calls: Vec<ToolCall>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u64>,
    pub embedding_dimensions: Option<u64>,
    pub embedding_count: Option<u64>,
    pub retrieved_documents: Vec<RetrievedDocument>,
    pub eval_scores: Vec<EvalScore>,
}

/// A single evaluation metric attached to a span (correctness, faithfulness,
/// hallucination, etc.), normalized from vendor-specific attribute shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScore {
    pub name: String,
    pub value: f64,
    pub threshold: Option<f64>,
    pub passed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmOperationType {
    ChatCompletion,
    TextCompletion,
    Embedding,
    Rerank,
    ToolCall,
    AgentStep,
    ChainStep,
    Retrieval,
    Unknown(String),
}

impl LlmOperationType {
    pub fn as_str(&self) -> String {
        match self {
            LlmOperationType::ChatCompletion => "ChatCompletion".to_string(),
            LlmOperationType::TextCompletion => "TextCompletion".to_string(),
            LlmOperationType::Embedding => "Embedding".to_string(),
            LlmOperationType::Rerank => "Rerank".to_string(),
            LlmOperationType::ToolCall => "ToolCall".to_string(),
            LlmOperationType::AgentStep => "AgentStep".to_string(),
            LlmOperationType::ChainStep => "ChainStep".to_string(),
            LlmOperationType::Retrieval => "Retrieval".to_string(),
            LlmOperationType::Unknown(s) => s.clone(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "chat" | "ChatCompletion" | "chat_completion" => LlmOperationType::ChatCompletion,
            "text_completion" | "TextCompletion" => LlmOperationType::TextCompletion,
            "embeddings" | "embedding" | "Embedding" => LlmOperationType::Embedding,
            "rerank" | "Rerank" => LlmOperationType::Rerank,
            "tool_call" | "ToolCall" => LlmOperationType::ToolCall,
            "agent" | "AgentStep" => LlmOperationType::AgentStep,
            "chain" | "ChainStep" => LlmOperationType::ChainStep,
            "retrieval" | "Retrieval" => LlmOperationType::Retrieval,
            other => LlmOperationType::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub id: Option<String>,
    pub score: Option<f64>,
    pub content_snippet: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_types_round_trip_through_their_wire_names() {
        let pairs = [
            ("chat", LlmOperationType::ChatCompletion, "ChatCompletion"),
            ("chat_completion", LlmOperationType::ChatCompletion, "ChatCompletion"),
            ("text_completion", LlmOperationType::TextCompletion, "TextCompletion"),
            ("embeddings", LlmOperationType::Embedding, "Embedding"),
            ("embedding", LlmOperationType::Embedding, "Embedding"),
            ("rerank", LlmOperationType::Rerank, "Rerank"),
            ("tool_call", LlmOperationType::ToolCall, "ToolCall"),
            ("agent", LlmOperationType::AgentStep, "AgentStep"),
            ("chain", LlmOperationType::ChainStep, "ChainStep"),
            ("retrieval", LlmOperationType::Retrieval, "Retrieval"),
        ];
        for (input, expected, name) in pairs {
            let parsed = LlmOperationType::from_str(input);
            assert_eq!(parsed.as_str(), expected.as_str(), "parsing {input}");
            assert_eq!(parsed.as_str(), name);
        }
    }

    #[test]
    fn an_unrecognized_operation_keeps_its_original_name() {
        let parsed = LlmOperationType::from_str("guardrail");
        assert_eq!(parsed.as_str(), "guardrail");
        assert!(matches!(parsed, LlmOperationType::Unknown(_)));
    }
}

