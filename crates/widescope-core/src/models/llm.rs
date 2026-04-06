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
