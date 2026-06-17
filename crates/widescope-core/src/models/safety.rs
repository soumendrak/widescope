use serde::{Deserialize, Serialize};

/// A safety/guardrail signal detected on a span (PII, jailbreak, refusal, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySignal {
    pub category: SafetyCategory,
    pub severity: SafetySeverity,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyCategory {
    Pii,
    Jailbreak,
    Refusal,
    ContentPolicy,
}

impl SafetyCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyCategory::Pii => "pii",
            SafetyCategory::Jailbreak => "jailbreak",
            SafetyCategory::Refusal => "refusal",
            SafetyCategory::ContentPolicy => "content_policy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySeverity {
    Low,
    Medium,
    High,
}
