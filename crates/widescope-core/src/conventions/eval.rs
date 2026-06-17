//! Discovery of evaluation scores on span attributes.
//!
//! LLM traces increasingly carry eval metrics (correctness, faithfulness,
//! hallucination, answer relevancy, …) as span attributes produced by
//! frameworks like LangSmith, Arize/OpenInference, MLflow and DeepEval.
//! WideScope walks them here and normalizes the per-metric fields into
//! [`EvalScore`] entries.
//!
//! Supported shapes (case-sensitive prefixes):
//!
//! ```text
//! Structured (preferred):
//!   eval.<name>.score
//!   eval.<name>.threshold
//!   eval.<name>.passed       (bool, optional)
//!
//! Flat (value directly on the metric key):
//!   eval.<name>              = 0.92
//!   score.<name>             = 0.92
//!
//! Other recognized prefixes: evaluation.*, llm.evaluation.*, score.*
//! ```
//!
//! A metric is only emitted if a numeric `value` was found. When `passed` is
//! not given explicitly but a `threshold` is, it is derived as `value >= threshold`.
//! Scores are returned sorted by name for stable display.

use crate::models::llm::EvalScore;
use crate::models::span::AttributeValue;
use std::collections::BTreeMap;
use std::collections::HashMap;

const PREFIXES: &[&str] = &["eval.", "evaluation.", "llm.evaluation.", "score."];
const FIELDS: &[&str] = &["score", "value", "threshold", "passed"];

#[derive(Default)]
struct Builder {
    value: Option<f64>,
    threshold: Option<f64>,
    passed: Option<bool>,
}

pub fn discover_eval_scores(attrs: &HashMap<String, AttributeValue>) -> Vec<EvalScore> {
    let mut by_name: BTreeMap<String, Builder> = BTreeMap::new();
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
                b.passed = coerce_to_bool(value);
            }
            _ => {}
        }
    }

    by_name
        .into_iter()
        .filter_map(|(name, b)| {
            let value = b.value?;
            let passed = b.passed.or_else(|| b.threshold.map(|t| value >= t));
            Some(EvalScore {
                name,
                value,
                threshold: b.threshold,
                passed,
            })
        })
        .collect()
}

/// Returns `(metric_name, field)` where `field` is `None` for the flat shape
/// (`eval.<name> = value`) or `Some(field)` for the structured shape.
fn parse_eval_key(key: &str) -> Option<(String, Option<String>)> {
    let rest = PREFIXES.iter().find_map(|p| key.strip_prefix(p))?;
    if rest.is_empty() {
        return None;
    }
    // Split off a trailing known field, if present. Metric names may contain
    // dots (e.g. "faithfulness.v2"), so only the recognized suffixes count.
    if let Some(dot) = rest.rfind('.') {
        let suffix = &rest[dot + 1..];
        if FIELDS.contains(&suffix) {
            let name = &rest[..dot];
            if !name.is_empty() {
                return Some((name.to_string(), Some(suffix.to_string())));
            }
        }
    }
    Some((rest.to_string(), None))
}

fn coerce_to_float(v: &AttributeValue) -> Option<f64> {
    match v {
        AttributeValue::Float(f) => Some(*f),
        AttributeValue::Int(i) => Some(*i as f64),
        AttributeValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        AttributeValue::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn coerce_to_bool(v: &AttributeValue) -> Option<bool> {
    match v {
        AttributeValue::Bool(b) => Some(*b),
        AttributeValue::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "pass" | "passed" | "yes" | "1" => Some(true),
            "false" | "fail" | "failed" | "no" | "0" => Some(false),
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
    fn parses_structured_with_threshold_and_derived_pass() {
        let a = attrs(&[
            ("eval.faithfulness.score", AttributeValue::Float(0.82)),
            ("eval.faithfulness.threshold", AttributeValue::Float(0.7)),
            ("eval.correctness.score", AttributeValue::Float(0.4)),
            ("eval.correctness.threshold", AttributeValue::Float(0.5)),
        ]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores.len(), 2);
        // Sorted by name: correctness, faithfulness
        assert_eq!(scores[0].name, "correctness");
        assert_eq!(scores[0].passed, Some(false));
        assert_eq!(scores[1].name, "faithfulness");
        assert_eq!(scores[1].value, 0.82);
        assert_eq!(scores[1].threshold, Some(0.7));
        assert_eq!(scores[1].passed, Some(true));
    }

    #[test]
    fn flat_value_shape() {
        let a = attrs(&[("eval.relevancy", AttributeValue::Float(0.9))]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].name, "relevancy");
        assert_eq!(scores[0].value, 0.9);
        assert_eq!(scores[0].passed, None);
    }

    #[test]
    fn explicit_passed_overrides_threshold() {
        let a = attrs(&[
            ("eval.x.score", AttributeValue::Float(0.9)),
            ("eval.x.threshold", AttributeValue::Float(0.5)),
            ("eval.x.passed", AttributeValue::Bool(false)),
        ]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores[0].passed, Some(false));
    }

    #[test]
    fn metric_name_with_dots() {
        let a = attrs(&[("eval.faithfulness.v2.score", AttributeValue::Float(0.6))]);
        let scores = discover_eval_scores(&a);
        assert_eq!(scores[0].name, "faithfulness.v2");
        assert_eq!(scores[0].value, 0.6);
    }

    #[test]
    fn coerces_string_score_and_bool() {
        let a = attrs(&[
            ("score.toxicity", AttributeValue::String("0.05".to_string())),
            ("eval.safe.score", AttributeValue::Float(1.0)),
            (
                "eval.safe.passed",
                AttributeValue::String("pass".to_string()),
            ),
        ]);
        let scores = discover_eval_scores(&a);
        let by: BTreeMap<_, _> = scores.iter().map(|s| (s.name.as_str(), s)).collect();
        assert_eq!(by["toxicity"].value, 0.05);
        assert_eq!(by["safe"].passed, Some(true));
    }

    #[test]
    fn drops_metric_without_value() {
        let a = attrs(&[("eval.x.threshold", AttributeValue::Float(0.5))]);
        assert!(discover_eval_scores(&a).is_empty());
    }

    #[test]
    fn ignores_unrelated_keys() {
        let a = attrs(&[
            ("gen_ai.usage.input_tokens", AttributeValue::Int(10)),
            ("db.statement", AttributeValue::String("select".to_string())),
        ]);
        assert!(discover_eval_scores(&a).is_empty());
    }
}
