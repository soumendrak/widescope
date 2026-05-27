use crate::conventions::registry::{Convention, MappingRule};
use crate::conventions::retrieval::discover_retrieved_documents;
use crate::models::llm::{EvalScore, LlmMessage, LlmOperationType, LlmSpanAttributes};
use crate::models::span::{AttributeValue, Span};
use std::collections::HashMap;

pub fn resolve_llm_attributes(
    span: &Span,
    conventions: &[Convention],
) -> Option<LlmSpanAttributes> {
    let retrieved = discover_retrieved_documents(&span.attributes);
    let eval_scores = discover_eval_scores(&span.attributes);
    for convention in conventions {
        if matches_convention(span, convention) {
            let mut llm = apply_mappings(span, convention);
            llm.retrieved_documents = retrieved;
            llm.eval_scores = eval_scores;
            return Some(llm);
        }
    }
    // Either retrieval data or eval scores can show up on spans that aren't
    // otherwise LLM spans (e.g. a dedicated vector-store retriever or an
    // evaluator service annotating a sibling). Surface them via a minimal
    // LlmSpanAttributes wrapper. Operation type follows whichever signal
    // we have: retrieval wins if present, otherwise we flag it as evaluation.
    if !retrieved.is_empty() || !eval_scores.is_empty() {
        let operation_type = if !retrieved.is_empty() {
            LlmOperationType::Retrieval
        } else {
            LlmOperationType::Unknown("evaluation".to_string())
        };
        return Some(LlmSpanAttributes {
            operation_type,
            model_name: None,
            model_provider: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
            input_messages: Vec::new(),
            output_messages: Vec::new(),
            tool_calls: Vec::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            embedding_dimensions: None,
            embedding_count: None,
            retrieved_documents: retrieved,
            eval_scores,
        });
    }
    None
}

fn matches_convention(span: &Span, convention: &Convention) -> bool {
    let detect = &convention.detect;

    if let Some(prefix) = &detect.attribute_prefix {
        if span
            .attributes
            .keys()
            .any(|k| k.starts_with(prefix.as_str()))
        {
            return true;
        }
    }

    if let Some(keys) = &detect.any_key_present {
        if keys.iter().any(|k| span.attributes.contains_key(k)) {
            return true;
        }
    }

    false
}

fn apply_mappings(span: &Span, convention: &Convention) -> LlmSpanAttributes {
    let mut llm = LlmSpanAttributes {
        operation_type: LlmOperationType::Unknown("unknown".to_string()),
        model_name: None,
        model_provider: None,
        input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        estimated_cost_usd: None,
        input_messages: Vec::new(),
        output_messages: Vec::new(),
        tool_calls: Vec::new(),
        temperature: None,
        top_p: None,
        max_tokens: None,
        embedding_dimensions: None,
        embedding_count: None,
        retrieved_documents: Vec::new(),
        eval_scores: Vec::new(),
    };

    for (field_name, rule) in &convention.mappings {
        match rule {
            MappingRule::Attribute(attr_mapping) => {
                let raw_value = span.attributes.get(&attr_mapping.attribute);
                match field_name.as_str() {
                    "operation_type" => {
                        if let Some(val) = raw_value {
                            let s = val.as_display_string();
                            if let Some(values_map) = &attr_mapping.values {
                                if let Some(canonical) = values_map.get(&s) {
                                    llm.operation_type = LlmOperationType::from_str(canonical);
                                } else if let Some(default) = &attr_mapping.default {
                                    llm.operation_type = LlmOperationType::from_str(default);
                                } else {
                                    llm.operation_type = LlmOperationType::from_str(&s);
                                }
                            } else {
                                llm.operation_type = LlmOperationType::from_str(&s);
                            }
                        } else if let Some(default) = &attr_mapping.default {
                            llm.operation_type = LlmOperationType::from_str(default);
                        }
                    }
                    "model_name" => {
                        llm.model_name = raw_value.map(|v| v.as_display_string());
                    }
                    "model_provider" => {
                        llm.model_provider = raw_value.map(|v| v.as_display_string());
                    }
                    "input_tokens" => {
                        llm.input_tokens = raw_value.and_then(coerce_to_u64);
                    }
                    "output_tokens" => {
                        llm.output_tokens = raw_value.and_then(coerce_to_u64);
                    }
                    "total_tokens" => {
                        llm.total_tokens = raw_value.and_then(coerce_to_u64);
                    }
                    "temperature" => {
                        llm.temperature = raw_value.and_then(|v| v.as_float());
                    }
                    "top_p" => {
                        llm.top_p = raw_value.and_then(|v| v.as_float());
                    }
                    "max_tokens" => {
                        llm.max_tokens = raw_value.and_then(coerce_to_u64);
                    }
                    "embedding_dimensions" => {
                        llm.embedding_dimensions = raw_value.and_then(coerce_to_u64);
                    }
                    "embedding_count" => {
                        llm.embedding_count = raw_value.and_then(coerce_to_u64);
                    }
                    _ => {}
                }
            }
            MappingRule::EventSource(evt_mapping) => {
                let messages = extract_messages_from_events(
                    span,
                    &evt_mapping.event_name,
                    &evt_mapping.content_attribute,
                );
                match field_name.as_str() {
                    "input_messages" => llm.input_messages = messages,
                    "output_messages" => llm.output_messages = messages,
                    _ => {}
                }
            }
        }
    }

    if let Some(in_t) = llm.input_tokens {
        if let Some(out_t) = llm.output_tokens {
            if llm.total_tokens.is_none() {
                llm.total_tokens = Some(in_t + out_t);
            }
        }
    }

    llm
}

fn coerce_to_u64(v: &AttributeValue) -> Option<u64> {
    match v {
        AttributeValue::Int(i) => {
            if *i >= 0 {
                Some(*i as u64)
            } else {
                None
            }
        }
        AttributeValue::Float(f) => {
            if *f >= 0.0 {
                Some(*f as u64)
            } else {
                None
            }
        }
        AttributeValue::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

fn extract_messages_from_events(
    span: &Span,
    event_name: &str,
    content_attribute: &str,
) -> Vec<LlmMessage> {
    span.events
        .iter()
        .filter(|e| e.name == event_name)
        .map(|e| {
            let content = e
                .attributes
                .get(content_attribute)
                .map(|v| v.as_display_string());
            LlmMessage {
                role: "user".to_string(),
                content,
            }
        })
        .collect()
}

/// Scan span attributes for evaluation metrics under common prefixes
/// (`eval.*`, `evaluation.*`) and group them into [`EvalScore`] entries.
///
/// Supported shapes per metric `<name>`:
/// - `<prefix>.<name>` = numeric score (bare value form)
/// - `<prefix>.<name>.score`        — numeric score
/// - `<prefix>.<name>.value`        — numeric score (alias for `.score`)
/// - `<prefix>.<name>.threshold`    — numeric threshold
/// - `<prefix>.<name>.passed`       — boolean pass/fail (overrides the
///   threshold-derived value when present)
/// - `<prefix>.<name>.label`        — string label (e.g. "PASS"/"FAIL")
/// - `<prefix>.<name>.explanation`  — string explanation
///
/// Scores are sorted by name for stable rendering.
pub fn discover_eval_scores(attrs: &HashMap<String, AttributeValue>) -> Vec<EvalScore> {
    #[derive(Default)]
    struct Builder {
        value: Option<f64>,
        label: Option<String>,
        threshold: Option<f64>,
        passed: Option<bool>,
        explanation: Option<String>,
    }

    let mut by_name: HashMap<String, Builder> = HashMap::new();
    for (key, value) in attrs {
        let Some((name, field)) = parse_eval_key(key) else {
            continue;
        };
        let b = by_name.entry(name).or_default();
        match field.as_deref() {
            None | Some("score") | Some("value") => {
                if let Some(f) = coerce_to_float(value) {
                    b.value = Some(f);
                }
            }
            Some("threshold") => {
                if let Some(f) = coerce_to_float(value) {
                    b.threshold = Some(f);
                }
            }
            Some("passed") => {
                if let Some(p) = coerce_to_bool(value) {
                    b.passed = Some(p);
                }
            }
            Some("label") => {
                b.label = Some(value.as_display_string());
            }
            Some("explanation") => {
                b.explanation = Some(value.as_display_string());
            }
            _ => {}
        }
    }

    let mut scores: Vec<EvalScore> = by_name
        .into_iter()
        .filter_map(|(name, b)| {
            // Skip empty entries — at least one signal is required.
            if b.value.is_none() && b.label.is_none() {
                return None;
            }
            let passed = b.passed.or(match (b.value, b.threshold) {
                (Some(v), Some(t)) => Some(v >= t),
                _ => None,
            });
            Some(EvalScore {
                name,
                value: b.value,
                label: b.label,
                threshold: b.threshold,
                passed,
                explanation: b.explanation,
            })
        })
        .collect();
    scores.sort_by(|a, b| a.name.cmp(&b.name));
    scores
}

fn parse_eval_key(key: &str) -> Option<(String, Option<String>)> {
    let stripped = key
        .strip_prefix("eval.")
        .or_else(|| key.strip_prefix("evaluation."))?;
    if stripped.is_empty() {
        return None;
    }
    if let Some(dot) = stripped.find('.') {
        let name = stripped[..dot].to_string();
        if name.is_empty() {
            return None;
        }
        let field = stripped[dot + 1..].to_string();
        Some((name, Some(field)))
    } else {
        Some((stripped.to_string(), None))
    }
}

fn coerce_to_float(v: &AttributeValue) -> Option<f64> {
    match v {
        AttributeValue::Float(f) => Some(*f),
        AttributeValue::Int(i) => Some(*i as f64),
        AttributeValue::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn coerce_to_bool(v: &AttributeValue) -> Option<bool> {
    match v {
        AttributeValue::Bool(b) => Some(*b),
        AttributeValue::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "yes" | "pass" | "passed" | "1" => Some(true),
            "false" | "no" | "fail" | "failed" | "0" => Some(false),
            _ => None,
        },
        AttributeValue::Int(i) => Some(*i != 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, AttributeValue)]) -> HashMap<String, AttributeValue> {
        pairs
            .iter()
            .cloned()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn discovers_bare_numeric_score() {
        let a = attrs(&[("eval.toxicity", AttributeValue::Float(0.05))]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].name, "toxicity");
        assert_eq!(scores[0].value, Some(0.05));
    }

    #[test]
    fn discovers_score_threshold_passed() {
        let a = attrs(&[
            ("eval.faithfulness.score", AttributeValue::Float(0.85)),
            ("eval.faithfulness.threshold", AttributeValue::Float(0.70)),
        ]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].name, "faithfulness");
        assert_eq!(scores[0].value, Some(0.85));
        assert_eq!(scores[0].threshold, Some(0.70));
        assert_eq!(scores[0].passed, Some(true));
    }

    #[test]
    fn explicit_passed_overrides_threshold_derived() {
        let a = attrs(&[
            ("eval.relevancy.score", AttributeValue::Float(0.5)),
            ("eval.relevancy.threshold", AttributeValue::Float(0.7)),
            ("eval.relevancy.passed", AttributeValue::Bool(true)),
        ]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores[0].passed, Some(true));
    }

    #[test]
    fn accepts_evaluation_prefix() {
        let a = attrs(&[("evaluation.correctness.score", AttributeValue::Float(0.92))]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].name, "correctness");
    }

    #[test]
    fn label_only_metric_is_kept() {
        let a = attrs(&[(
            "eval.hallucination.label",
            AttributeValue::String("PASS".to_string()),
        )]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].label.as_deref(), Some("PASS"));
        assert_eq!(scores[0].value, None);
    }

    #[test]
    fn empty_metric_is_dropped() {
        let a = attrs(&[(
            "eval.something.explanation",
            AttributeValue::String("…".to_string()),
        )]);
        let scores = discover_eval_scores(&a);
        // Only an explanation, no value/label → discarded.
        assert!(scores.is_empty());
    }

    #[test]
    fn sorted_by_name() {
        let a = attrs(&[
            ("eval.zeta.score", AttributeValue::Float(0.1)),
            ("eval.alpha.score", AttributeValue::Float(0.2)),
            ("eval.mu.score", AttributeValue::Float(0.3)),
        ]);
        let scores = discover_eval_scores(&a);
        let names: Vec<&str> = scores.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn ignores_non_eval_keys() {
        let a = attrs(&[
            ("evaluator.name", AttributeValue::String("x".to_string())),
            ("score.something", AttributeValue::Float(0.5)),
        ]);
        let scores = discover_eval_scores(&a);
        assert!(scores.is_empty());
    }
}
