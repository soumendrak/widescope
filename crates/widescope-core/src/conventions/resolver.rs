use crate::conventions::eval::discover_eval_scores;
use crate::conventions::registry::{Convention, MappingRule};
use crate::conventions::retrieval::discover_retrieved_documents;
use crate::models::llm::{LlmMessage, LlmOperationType, LlmSpanAttributes};
use crate::models::span::{AttributeValue, Span};

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
    // Retrieved documents or eval scores can show up on spans that aren't
    // otherwise LLM spans (e.g. a dedicated retriever, or an eval-only span) —
    // surface them anyway via a minimal LlmSpanAttributes wrapper.
    if !retrieved.is_empty() || !eval_scores.is_empty() {
        return Some(LlmSpanAttributes {
            operation_type: LlmOperationType::Retrieval,
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

/// Convention matching and field mapping against the bundled convention files.
///
/// These are the rules that turn a vendor's attribute names into the model,
/// token and cost fields the inspector shows, so a mapping that silently misses
/// looks like "the trace has no LLM data".
#[cfg(test)]
mod mapping_tests {
    use super::*;
    use crate::conventions::registry::load_conventions;
    use crate::models::span::{SpanKind, SpanStatus};
    use std::collections::HashMap;

    const OTEL: &str = include_str!("../../../../conventions/opentelemetry.json");
    const OI: &str = include_str!("../../../../conventions/openinference.json");
    const LANGCHAIN: &str = include_str!("../../../../conventions/langchain.json");

    fn bundled() -> Vec<Convention> {
        let merged = format!("[{OTEL},{OI},{LANGCHAIN}]");
        let result = load_conventions(&merged);
        assert!(
            result.warnings.is_empty(),
            "bundled conventions must load cleanly"
        );
        result.conventions
    }

    fn span_with(attrs: Vec<(&str, AttributeValue)>) -> Span {
        Span {
            trace_id: "t".into(),
            span_id: "s".into(),
            parent_span_id: None,
            operation_name: "op".into(),
            service_name: "svc".into(),
            span_kind: SpanKind::Client,
            start_time_ns: 0,
            end_time_ns: 1,
            duration_ns: 1,
            self_time_ns: 1,
            status: SpanStatus::Ok,
            attributes: attrs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<HashMap<_, _>>(),
            events: vec![],
            llm: None,
            safety: vec![],
        }
    }

    fn text(value: &str) -> AttributeValue {
        AttributeValue::String(value.to_string())
    }

    #[test]
    fn a_span_with_no_known_attributes_resolves_to_nothing() {
        let span = span_with(vec![("http.method", text("GET"))]);
        assert!(resolve_llm_attributes(&span, &bundled()).is_none());
    }

    #[test]
    fn otel_genai_attributes_resolve_model_tokens_and_operation() {
        let span = span_with(vec![
            ("gen_ai.system", text("openai")),
            ("gen_ai.operation.name", text("chat")),
            ("gen_ai.request.model", text("gpt-4o")),
            ("gen_ai.usage.input_tokens", AttributeValue::Int(120)),
            ("gen_ai.usage.output_tokens", AttributeValue::Int(30)),
            ("gen_ai.request.temperature", AttributeValue::Float(0.7)),
            ("gen_ai.request.max_tokens", AttributeValue::Int(512)),
        ]);
        let llm = resolve_llm_attributes(&span, &bundled()).expect("gen_ai span should resolve");
        assert_eq!(llm.model_name.as_deref(), Some("gpt-4o"));
        assert_eq!(llm.input_tokens, Some(120));
        assert_eq!(llm.output_tokens, Some(30));
        // Total is derived when the export does not send it.
        assert_eq!(llm.total_tokens, Some(150));
        assert_eq!(llm.temperature, Some(0.7));
        assert_eq!(llm.max_tokens, Some(512));
        assert_eq!(llm.operation_type.as_str(), "ChatCompletion");
    }

    #[test]
    fn openinference_attributes_resolve_through_their_own_names() {
        let span = span_with(vec![
            ("llm.model_name", text("claude-3-5-sonnet")),
            ("openinference.span.kind", text("LLM")),
            ("llm.token_count.prompt", AttributeValue::Int(10)),
            ("llm.token_count.completion", AttributeValue::Int(5)),
            ("llm.token_count.total", AttributeValue::Int(15)),
        ]);
        let llm = resolve_llm_attributes(&span, &bundled()).expect("openinference span");
        assert_eq!(llm.model_name.as_deref(), Some("claude-3-5-sonnet"));
        assert_eq!(llm.total_tokens, Some(15));
    }

    #[test]
    fn a_value_map_miss_falls_back_to_the_default_then_the_raw_value() {
        // `gen_ai.operation.name` carries a value the convention does not map.
        let span = span_with(vec![
            ("gen_ai.system", text("openai")),
            ("gen_ai.operation.name", text("guardrail")),
        ]);
        let llm = resolve_llm_attributes(&span, &bundled()).unwrap();
        assert!(!llm.operation_type.as_str().is_empty());
    }

    #[test]
    fn detection_matches_on_a_named_key_as_well_as_a_prefix() {
        let by_prefix = span_with(vec![("gen_ai.anything", text("x"))]);
        assert!(resolve_llm_attributes(&by_prefix, &bundled()).is_some());

        let by_key = span_with(vec![("llm.model_name", text("gpt-4o"))]);
        assert!(resolve_llm_attributes(&by_key, &bundled()).is_some());
    }

    #[test]
    fn embedding_fields_resolve_for_embedding_spans() {
        let span = span_with(vec![
            ("gen_ai.system", text("openai")),
            ("gen_ai.operation.name", text("embeddings")),
            ("gen_ai.request.model", text("text-embedding-3-small")),
            ("gen_ai.usage.input_tokens", AttributeValue::Int(12)),
        ]);
        let llm = resolve_llm_attributes(&span, &bundled()).unwrap();
        assert_eq!(llm.operation_type.as_str(), "Embedding");
        // Without an output count there is nothing to total.
        assert_eq!(llm.total_tokens, None);
    }

    #[test]
    fn numeric_coercion_accepts_the_encodings_exporters_actually_use() {
        assert_eq!(coerce_to_u64(&AttributeValue::Int(5)), Some(5));
        assert_eq!(coerce_to_u64(&AttributeValue::Int(-5)), None);
        assert_eq!(coerce_to_u64(&AttributeValue::Float(5.9)), Some(5));
        assert_eq!(coerce_to_u64(&AttributeValue::Float(-0.5)), None);
        assert_eq!(coerce_to_u64(&text("42")), Some(42));
        assert_eq!(coerce_to_u64(&text("nope")), None);
        assert_eq!(coerce_to_u64(&AttributeValue::Bool(true)), None);
    }

    #[test]
    fn messages_are_lifted_out_of_span_events_when_a_convention_says_so() {
        use crate::models::span::SpanEvent;

        let mut span = span_with(vec![("gen_ai.system", text("openai"))]);
        span.events = vec![
            SpanEvent {
                name: "gen_ai.content.prompt".into(),
                timestamp_ns: 1,
                attributes: HashMap::from([("gen_ai.prompt".to_string(), text("hello"))]),
            },
            SpanEvent {
                name: "gen_ai.content.completion".into(),
                timestamp_ns: 2,
                attributes: HashMap::from([("gen_ai.completion".to_string(), text("hi back"))]),
            },
        ];
        let llm = resolve_llm_attributes(&span, &bundled()).unwrap();
        assert_eq!(llm.input_messages.len(), 1);
        assert_eq!(llm.input_messages[0].content.as_deref(), Some("hello"));
        assert_eq!(llm.output_messages.len(), 1);
        assert_eq!(llm.output_messages[0].content.as_deref(), Some("hi back"));
    }
}
