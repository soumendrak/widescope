//! Discovery of guardrail / safety signals on span attributes.
//!
//! WideScope doesn't run pattern matching on prompt content (too noisy without
//! a model in the loop). Instead, it surfaces signals that an upstream system
//! has already written into the trace as structured attributes.
//!
//! Supported prefixes (case-sensitive):
//!   - `guardrail.<name>` / `guardrail.<name>.<field>`
//!   - `safety.<name>` / `safety.<name>.<field>`
//!   - `gen_ai.guardrail.<name>` / `gen_ai.guardrail.<name>.<field>`
//!   - `gen_ai.safety.<name>` / `gen_ai.safety.<name>.<field>`
//!   - `pii.<field>`              — collapsed to a single "pii" signal
//!
//! Per signal, recognised fields:
//!   - `<bare>`         — boolean triggered flag (or numeric score)
//!   - `.triggered` / `.violated` / `.detected` — explicit bool
//!   - `.score`         — numeric score
//!   - `.severity`      — string ("low"/"medium"/"high") or numeric
//!   - `.category`      — explicit category override
//!   - `.detail` / `.message` / `.explanation` — string detail

use crate::models::span::AttributeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySignal {
    pub name: String,
    pub category: SafetyCategory,
    pub triggered: bool,
    pub score: Option<f64>,
    pub severity: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SafetyCategory {
    Pii,
    Jailbreak,
    Refusal,
    ContentPolicy,
    Toxicity,
    Hallucination,
    Other,
}

impl SafetyCategory {
    pub fn from_name(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        if lower.contains("pii") || lower.contains("personal") {
            SafetyCategory::Pii
        } else if lower.contains("jailbreak") || lower.contains("prompt_injection") {
            SafetyCategory::Jailbreak
        } else if lower.contains("refusal") {
            SafetyCategory::Refusal
        } else if lower.contains("toxic") {
            SafetyCategory::Toxicity
        } else if lower.contains("hallucin") {
            SafetyCategory::Hallucination
        } else if lower.contains("policy")
            || lower.contains("content")
            || lower.contains("moderation")
        {
            SafetyCategory::ContentPolicy
        } else {
            SafetyCategory::Other
        }
    }

    pub fn from_explicit(label: &str) -> Self {
        match label.to_ascii_lowercase().as_str() {
            "pii" | "personal_info" => SafetyCategory::Pii,
            "jailbreak" | "prompt_injection" => SafetyCategory::Jailbreak,
            "refusal" => SafetyCategory::Refusal,
            "toxicity" | "toxic" => SafetyCategory::Toxicity,
            "hallucination" | "hallucin" => SafetyCategory::Hallucination,
            "content_policy" | "policy" | "moderation" | "content" => SafetyCategory::ContentPolicy,
            other => SafetyCategory::from_name(other),
        }
    }
}

#[derive(Default)]
struct Builder {
    triggered: Option<bool>,
    score: Option<f64>,
    severity: Option<String>,
    detail: Option<String>,
    explicit_category: Option<SafetyCategory>,
}

pub fn discover_safety_signals(attrs: &HashMap<String, AttributeValue>) -> Vec<SafetySignal> {
    let mut by_name: HashMap<String, Builder> = HashMap::new();
    // Track PII sub-keys we see so we can roll them up into the detail string.
    let mut pii_findings: Vec<String> = Vec::new();

    for (key, value) in attrs {
        let Some(parsed) = parse_safety_key(key) else {
            continue;
        };
        let SafetyKey {
            name,
            field,
            is_flat_pii,
        } = parsed;
        let b = by_name.entry(name).or_default();

        // `pii.<anything>` is presence-based: any key under `pii.` means a
        // PII finding was made. The sub-key becomes part of the detail rollup.
        if is_flat_pii {
            b.triggered = Some(true);
            if let Some(f) = field {
                pii_findings.push(f);
            }
            continue;
        }

        match field.as_deref() {
            None => {
                // Bare key — treat as triggered if bool, or store as score if numeric.
                if let Some(t) = coerce_to_bool(value) {
                    b.triggered = Some(t);
                } else if let Some(f) = coerce_to_float(value) {
                    b.score = Some(f);
                    b.triggered.get_or_insert(true);
                }
            }
            Some("triggered") | Some("violated") | Some("detected") | Some("flagged") => {
                if let Some(t) = coerce_to_bool(value) {
                    b.triggered = Some(t);
                }
            }
            Some("score") | Some("confidence") => {
                if let Some(f) = coerce_to_float(value) {
                    b.score = Some(f);
                }
            }
            Some("severity") => {
                b.severity = Some(value.as_display_string());
            }
            Some("category") | Some("type") => {
                b.explicit_category =
                    Some(SafetyCategory::from_explicit(&value.as_display_string()));
            }
            Some("detail") | Some("message") | Some("explanation") | Some("reason") => {
                b.detail = Some(value.as_display_string());
            }
            _ => {}
        }
    }

    if !pii_findings.is_empty() {
        pii_findings.sort();
        let b = by_name.entry("pii".to_string()).or_default();
        if b.detail.is_none() {
            b.detail = Some(format!("findings: {}", pii_findings.join(", ")));
        }
    }

    let mut signals: Vec<SafetySignal> = by_name
        .into_iter()
        .filter_map(|(name, b)| {
            // Drop entries with no concrete signal.
            if b.triggered.is_none() && b.score.is_none() && b.detail.is_none() {
                return None;
            }
            let category = b
                .explicit_category
                .unwrap_or_else(|| SafetyCategory::from_name(&name));
            Some(SafetySignal {
                name,
                category,
                triggered: b.triggered.unwrap_or(false),
                score: b.score,
                severity: b.severity,
                detail: b.detail,
            })
        })
        .collect();

    // Sort: triggered first (visibility), then by name.
    signals.sort_by(|a, b| match (b.triggered, a.triggered) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.name.cmp(&b.name),
    });
    signals
}

struct SafetyKey {
    name: String,
    field: Option<String>,
    is_flat_pii: bool,
}

/// Returns the bare signal name and an optional field suffix when the key
/// matches a recognised safety prefix.
fn parse_safety_key(key: &str) -> Option<SafetyKey> {
    // Try multi-segment prefixes first so they don't get absorbed by the
    // shorter ones.
    let prefixes: &[&str] = &[
        "gen_ai.guardrail.",
        "gen_ai.safety.",
        "guardrail.",
        "safety.",
    ];
    for prefix in prefixes {
        if let Some(rest) = key.strip_prefix(prefix) {
            let (name, field) = split_name_field(rest)?;
            return Some(SafetyKey {
                name,
                field,
                is_flat_pii: false,
            });
        }
    }
    // PII has a flatter shape — every `pii.<anything>` collapses to one signal.
    if let Some(rest) = key.strip_prefix("pii.") {
        if rest.is_empty() {
            return None;
        }
        return Some(SafetyKey {
            name: "pii".to_string(),
            field: Some(rest.to_string()),
            is_flat_pii: true,
        });
    }
    None
}

fn split_name_field(rest: &str) -> Option<(String, Option<String>)> {
    if rest.is_empty() {
        return None;
    }
    if let Some(dot) = rest.find('.') {
        let name = rest[..dot].to_string();
        if name.is_empty() {
            return None;
        }
        Some((name, Some(rest[dot + 1..].to_string())))
    } else {
        Some((rest.to_string(), None))
    }
}

fn coerce_to_bool(v: &AttributeValue) -> Option<bool> {
    match v {
        AttributeValue::Bool(b) => Some(*b),
        AttributeValue::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "yes" | "triggered" | "violated" | "detected" | "1" => Some(true),
            "false" | "no" | "ok" | "passed" | "0" => Some(false),
            _ => None,
        },
        AttributeValue::Int(i) => Some(*i != 0),
        _ => None,
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
    fn parses_guardrail_pii_triggered() {
        let a = attrs(&[
            ("guardrail.pii.triggered", AttributeValue::Bool(true)),
            (
                "guardrail.pii.detail",
                AttributeValue::String("email detected".to_string()),
            ),
        ]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].name, "pii");
        assert_eq!(signals[0].category, SafetyCategory::Pii);
        assert!(signals[0].triggered);
        assert_eq!(signals[0].detail.as_deref(), Some("email detected"));
    }

    #[test]
    fn parses_jailbreak_with_score_no_threshold() {
        let a = attrs(&[("guardrail.jailbreak.score", AttributeValue::Float(0.87))]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].category, SafetyCategory::Jailbreak);
        assert_eq!(signals[0].score, Some(0.87));
        // Score alone doesn't set triggered = true.
        assert!(!signals[0].triggered);
    }

    #[test]
    fn bare_bool_marks_triggered() {
        let a = attrs(&[("guardrail.refusal", AttributeValue::Bool(true))]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].category, SafetyCategory::Refusal);
        assert!(signals[0].triggered);
    }

    #[test]
    fn bare_numeric_assumes_triggered() {
        let a = attrs(&[("safety.toxicity", AttributeValue::Float(0.6))]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].category, SafetyCategory::Toxicity);
        assert!(signals[0].triggered);
        assert_eq!(signals[0].score, Some(0.6));
    }

    #[test]
    fn explicit_category_overrides_name_inference() {
        let a = attrs(&[
            ("guardrail.thing.triggered", AttributeValue::Bool(true)),
            (
                "guardrail.thing.category",
                AttributeValue::String("pii".to_string()),
            ),
        ]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals[0].category, SafetyCategory::Pii);
    }

    #[test]
    fn flat_pii_keys_collapse_to_one_signal() {
        let a = attrs(&[
            (
                "pii.email",
                AttributeValue::String("user@example.com".to_string()),
            ),
            ("pii.phone", AttributeValue::String("555-1234".to_string())),
        ]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].name, "pii");
        assert_eq!(signals[0].category, SafetyCategory::Pii);
    }

    #[test]
    fn ignores_unrelated_keys() {
        let a = attrs(&[
            ("eval.faithfulness.score", AttributeValue::Float(0.9)),
            ("http.status_code", AttributeValue::Int(200)),
        ]);
        let signals = discover_safety_signals(&a);
        assert!(signals.is_empty());
    }

    #[test]
    fn empty_metric_dropped() {
        let a = attrs(&[(
            "guardrail.something.severity",
            AttributeValue::String("medium".to_string()),
        )]);
        // Only a severity, no triggered/score/detail → drop.
        let signals = discover_safety_signals(&a);
        assert!(signals.is_empty());
    }

    #[test]
    fn triggered_signals_sort_first() {
        let a = attrs(&[
            ("guardrail.aaa.triggered", AttributeValue::Bool(false)),
            ("guardrail.bbb.triggered", AttributeValue::Bool(true)),
            ("guardrail.zzz.triggered", AttributeValue::Bool(true)),
        ]);
        let signals = discover_safety_signals(&a);
        let names: Vec<&str> = signals.iter().map(|s| s.name.as_str()).collect();
        // Triggered (bbb, zzz alphabetically) come before non-triggered (aaa).
        assert_eq!(names, vec!["bbb", "zzz", "aaa"]);
    }

    #[test]
    fn gen_ai_safety_prefix_works() {
        let a = attrs(&[(
            "gen_ai.safety.hallucination.triggered",
            AttributeValue::Bool(true),
        )]);
        let signals = discover_safety_signals(&a);
        assert_eq!(signals[0].category, SafetyCategory::Hallucination);
        assert!(signals[0].triggered);
    }
}
