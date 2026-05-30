//! Discovery of retrieved documents on span attributes.
//!
//! RAG / retrieval traces commonly encode the list of fetched documents as
//! indexed attributes. WideScope walks them here and groups the per-index
//! fields into [`RetrievedDocument`] entries.
//!
//! Supported shapes (case-sensitive):
//!
//! ```text
//! OpenInference:
//!   retrieval.documents.<i>.document.id
//!   retrieval.documents.<i>.document.score
//!   retrieval.documents.<i>.document.content
//!   retrieval.documents.<i>.document.metadata
//!
//! Shorter variant (some vendors omit the inner `.document` segment):
//!   retrieval.documents.<i>.id
//!   retrieval.documents.<i>.score
//!   retrieval.documents.<i>.content
//!
//! OTel GenAI proposed:
//!   gen_ai.retrieval.documents.<i>.id  (etc.)
//! ```
//!
//! Documents are returned in ascending index order so the caller sees them
//! in the order the retriever produced them.

use crate::models::llm::RetrievedDocument;
use crate::models::span::AttributeValue;
use std::collections::BTreeMap;
use std::collections::HashMap;

const PREFIXES: &[&str] = &["retrieval.documents.", "gen_ai.retrieval.documents."];

#[derive(Default)]
struct Builder {
    id: Option<String>,
    score: Option<f64>,
    content_snippet: Option<String>,
}

pub fn discover_retrieved_documents(
    attrs: &HashMap<String, AttributeValue>,
) -> Vec<RetrievedDocument> {
    let mut by_index: BTreeMap<u32, Builder> = BTreeMap::new();
    for (key, value) in attrs {
        let Some((idx, field)) = parse_doc_key(key) else {
            continue;
        };
        let b = by_index.entry(idx).or_default();
        match field.as_str() {
            "id" | "document.id" => {
                b.id = Some(value.as_display_string());
            }
            "score" | "document.score" => {
                if let Some(f) = coerce_to_float(value) {
                    b.score = Some(f);
                }
            }
            "content" | "document.content" => {
                let s = value.as_display_string();
                b.content_snippet = Some(truncate_snippet(&s, 512));
            }
            _ => {}
        }
    }

    by_index
        .into_values()
        .filter_map(|b| {
            // Keep an entry if at least one of id/score/content is present.
            if b.id.is_none() && b.score.is_none() && b.content_snippet.is_none() {
                None
            } else {
                Some(RetrievedDocument {
                    id: b.id,
                    score: b.score,
                    content_snippet: b.content_snippet,
                })
            }
        })
        .collect()
}

fn parse_doc_key(key: &str) -> Option<(u32, String)> {
    for prefix in PREFIXES {
        if let Some(rest) = key.strip_prefix(prefix) {
            // rest = "<idx>.<field>" or "<idx>.document.<field>"
            let dot = rest.find('.')?;
            let idx: u32 = rest[..dot].parse().ok()?;
            let field = rest[dot + 1..].to_string();
            if field.is_empty() {
                return None;
            }
            return Some((idx, field));
        }
    }
    None
}

fn coerce_to_float(v: &AttributeValue) -> Option<f64> {
    match v {
        AttributeValue::Float(f) => Some(*f),
        AttributeValue::Int(i) => Some(*i as f64),
        AttributeValue::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn truncate_snippet(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars).collect();
        out.push('…');
        out
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
    fn parses_openinference_documents() {
        let a = attrs(&[
            (
                "retrieval.documents.0.document.id",
                AttributeValue::String("d1".to_string()),
            ),
            (
                "retrieval.documents.0.document.score",
                AttributeValue::Float(0.92),
            ),
            (
                "retrieval.documents.0.document.content",
                AttributeValue::String("Paris is the capital".to_string()),
            ),
            (
                "retrieval.documents.1.document.id",
                AttributeValue::String("d2".to_string()),
            ),
            (
                "retrieval.documents.1.document.score",
                AttributeValue::Float(0.61),
            ),
        ]);
        let docs = discover_retrieved_documents(&a);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id.as_deref(), Some("d1"));
        assert_eq!(docs[0].score, Some(0.92));
        assert_eq!(
            docs[0].content_snippet.as_deref(),
            Some("Paris is the capital")
        );
        assert_eq!(docs[1].id.as_deref(), Some("d2"));
        assert_eq!(docs[1].score, Some(0.61));
    }

    #[test]
    fn parses_short_variant_without_inner_document_segment() {
        let a = attrs(&[
            (
                "retrieval.documents.0.id",
                AttributeValue::String("d1".to_string()),
            ),
            ("retrieval.documents.0.score", AttributeValue::Float(0.5)),
        ]);
        let docs = discover_retrieved_documents(&a);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id.as_deref(), Some("d1"));
    }

    #[test]
    fn parses_gen_ai_prefix() {
        let a = attrs(&[
            (
                "gen_ai.retrieval.documents.0.id",
                AttributeValue::String("x".to_string()),
            ),
            (
                "gen_ai.retrieval.documents.0.score",
                AttributeValue::Float(0.7),
            ),
        ]);
        let docs = discover_retrieved_documents(&a);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id.as_deref(), Some("x"));
    }

    #[test]
    fn coerces_string_score() {
        let a = attrs(&[(
            "retrieval.documents.0.score",
            AttributeValue::String("0.45".to_string()),
        )]);
        let docs = discover_retrieved_documents(&a);
        assert_eq!(docs[0].score, Some(0.45));
    }

    #[test]
    fn truncates_long_content() {
        let long = "x".repeat(1000);
        let a = attrs(&[(
            "retrieval.documents.0.content",
            AttributeValue::String(long.clone()),
        )]);
        let docs = discover_retrieved_documents(&a);
        let snippet = docs[0].content_snippet.as_ref().unwrap();
        assert!(snippet.chars().count() <= 513); // 512 + ellipsis
        assert!(snippet.ends_with('…'));
    }

    #[test]
    fn ascending_index_order() {
        let a = attrs(&[
            (
                "retrieval.documents.5.id",
                AttributeValue::String("e".to_string()),
            ),
            (
                "retrieval.documents.2.id",
                AttributeValue::String("b".to_string()),
            ),
            (
                "retrieval.documents.0.id",
                AttributeValue::String("a".to_string()),
            ),
            (
                "retrieval.documents.10.id",
                AttributeValue::String("k".to_string()),
            ),
        ]);
        let docs = discover_retrieved_documents(&a);
        let ids: Vec<&str> = docs.iter().map(|d| d.id.as_deref().unwrap()).collect();
        assert_eq!(ids, vec!["a", "b", "e", "k"]);
    }

    #[test]
    fn ignores_non_numeric_index() {
        let a = attrs(&[(
            "retrieval.documents.foo.id",
            AttributeValue::String("x".to_string()),
        )]);
        let docs = discover_retrieved_documents(&a);
        assert!(docs.is_empty());
    }

    #[test]
    fn ignores_unrelated_attribute_keys() {
        let a = attrs(&[
            ("rerank.top_n", AttributeValue::Int(3)),
            ("db.statement", AttributeValue::String("select".to_string())),
        ]);
        let docs = discover_retrieved_documents(&a);
        assert!(docs.is_empty());
    }
}
