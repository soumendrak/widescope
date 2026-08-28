use crate::models::safety::{SafetyCategory, SafetySeverity, SafetySignal};
use crate::models::span::Span;

// ponytail: hardcoded patterns — these are stable OTel GenAI semconv / NeMo
// attribute names, not per-user config. Promote to a conventions/*.json loader
// only if users need to add their own guardrail vendors.
const REFUSAL_PHRASES: &[&str] = &[
    "i cannot",
    "i can't",
    "i apologize",
    "i'm unable",
    "i am unable",
    "i'm sorry, but",
    "as an ai",
    "i won't",
];

/// Detect safety/guardrail signals from a span's attributes and (already
/// resolved) LLM output messages.
pub fn detect_safety_signals(span: &Span) -> Vec<SafetySignal> {
    let mut signals = Vec::new();

    for (key, value) in &span.attributes {
        let k = key.to_ascii_lowercase();
        let v = value.as_display_string();

        // A key matching a category still counts as "clean" when its value
        // explicitly says nothing fired (e.g. gen_ai.pii.detected=false).
        if k.contains("pii") && !is_disabled(&v) {
            signals.push(SafetySignal {
                category: SafetyCategory::Pii,
                severity: SafetySeverity::High,
                detail: format!("{key} = {v}"),
            });
        } else if (k.contains("jailbreak") || k.contains("prompt_injection")) && !is_disabled(&v) {
            signals.push(SafetySignal {
                category: SafetyCategory::Jailbreak,
                severity: SafetySeverity::High,
                detail: format!("{key} = {v}"),
            });
        } else if k.contains("guardrail") && is_triggered(&v) {
            signals.push(SafetySignal {
                category: SafetyCategory::ContentPolicy,
                severity: SafetySeverity::Medium,
                detail: format!("{key} = {v}"),
            });
        }
    }

    // Refusal: scan resolved completion text for known refusal openers.
    if let Some(llm) = &span.llm {
        for msg in &llm.output_messages {
            if let Some(content) = &msg.content {
                let lower = content.to_ascii_lowercase();
                if let Some(phrase) = REFUSAL_PHRASES.iter().find(|p| lower.contains(**p)) {
                    signals.push(SafetySignal {
                        category: SafetyCategory::Refusal,
                        severity: SafetySeverity::Low,
                        detail: format!("model response contains \"{phrase}\""),
                    });
                    break;
                }
            }
        }
    }

    signals
}

fn is_triggered(value: &str) -> bool {
    let v = value.to_ascii_lowercase();
    matches!(
        v.as_str(),
        "true" | "blocked" | "triggered" | "violation" | "flagged" | "fail" | "failed"
    )
}

/// True when a flag value explicitly indicates the signal did NOT fire.
/// A non-boolean payload (e.g. a detected-PII type list) is not "disabled".
fn is_disabled(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    matches!(
        v.as_str(),
        "" | "false" | "0" | "no" | "none" | "null" | "off" | "disabled" | "[]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::llm::{LlmMessage, LlmOperationType, LlmSpanAttributes};
    use crate::models::span::{AttributeValue, SpanKind, SpanStatus};
    use std::collections::HashMap;

    fn span_with(attrs: HashMap<String, AttributeValue>) -> Span {
        Span {
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
            status: SpanStatus::Ok,
            attributes: attrs,
            events: Vec::new(),
            llm: None,
            safety: Vec::new(),
        }
    }

    fn llm_with_output(content: &str) -> LlmSpanAttributes {
        LlmSpanAttributes {
            operation_type: LlmOperationType::ChatCompletion,
            model_name: None,
            model_provider: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
            input_messages: Vec::new(),
            output_messages: vec![LlmMessage {
                role: "assistant".into(),
                content: Some(content.into()),
            }],
            tool_calls: Vec::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            embedding_dimensions: None,
            embedding_count: None,
            retrieved_documents: Vec::new(),
            eval_scores: Vec::new(),
        }
    }

    #[test]
    fn detects_pii_jailbreak_policy_and_refusal() {
        let mut attrs = HashMap::new();
        attrs.insert("gen_ai.pii.detected".into(), AttributeValue::Bool(true));
        attrs.insert("guardrail.jailbreak".into(), AttributeValue::Bool(true));
        attrs.insert(
            "gen_ai.guardrail.policy".into(),
            AttributeValue::String("blocked".into()),
        );
        let mut span = span_with(attrs);
        span.llm = Some(llm_with_output("I cannot help with that request."));

        let signals = detect_safety_signals(&span);
        let cats: Vec<_> = signals.iter().map(|s| s.category).collect();
        assert!(cats.contains(&SafetyCategory::Pii));
        assert!(cats.contains(&SafetyCategory::Jailbreak));
        assert!(cats.contains(&SafetyCategory::ContentPolicy));
        assert!(cats.contains(&SafetyCategory::Refusal));
    }

    #[test]
    fn disabled_flags_do_not_flag() {
        let mut attrs = HashMap::new();
        attrs.insert("gen_ai.pii.detected".into(), AttributeValue::Bool(false));
        attrs.insert(
            "guardrail.jailbreak".into(),
            AttributeValue::String("false".into()),
        );
        attrs.insert("prompt_injection.score".into(), AttributeValue::Int(0));
        let span = span_with(attrs);
        assert!(detect_safety_signals(&span).is_empty());

        // …but a real PII payload (non-boolean) still flags.
        let mut attrs = HashMap::new();
        attrs.insert(
            "gen_ai.pii.types".into(),
            AttributeValue::String("[email, ssn]".into()),
        );
        let span = span_with(attrs);
        assert_eq!(detect_safety_signals(&span).len(), 1);
    }

    #[test]
    fn clean_span_has_no_signals() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "gen_ai.request.model".into(),
            AttributeValue::String("gpt-4o".into()),
        );
        // guardrail attribute that did NOT trigger must not flag
        attrs.insert(
            "gen_ai.guardrail.policy".into(),
            AttributeValue::String("pass".into()),
        );
        let mut span = span_with(attrs);
        span.llm = Some(llm_with_output("Sure, here is the answer."));
        assert!(detect_safety_signals(&span).is_empty());
    }
}
