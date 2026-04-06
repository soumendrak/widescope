use crate::conventions::registry::{Convention, MappingRule};
use crate::models::llm::{LlmMessage, LlmOperationType, LlmSpanAttributes};
use crate::models::span::{AttributeValue, Span};

pub fn resolve_llm_attributes(span: &Span, conventions: &[Convention]) -> Option<LlmSpanAttributes> {
    for convention in conventions {
        if matches_convention(span, convention) {
            return Some(apply_mappings(span, convention));
        }
    }
    None
}

fn matches_convention(span: &Span, convention: &Convention) -> bool {
    let detect = &convention.detect;

    if let Some(prefix) = &detect.attribute_prefix {
        if span.attributes.keys().any(|k| k.starts_with(prefix.as_str())) {
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
                        llm.input_tokens = raw_value.and_then(|v| coerce_to_u64(v));
                    }
                    "output_tokens" => {
                        llm.output_tokens = raw_value.and_then(|v| coerce_to_u64(v));
                    }
                    "total_tokens" => {
                        llm.total_tokens = raw_value.and_then(|v| coerce_to_u64(v));
                    }
                    "temperature" => {
                        llm.temperature = raw_value.and_then(|v| v.as_float());
                    }
                    "top_p" => {
                        llm.top_p = raw_value.and_then(|v| v.as_float());
                    }
                    "max_tokens" => {
                        llm.max_tokens = raw_value.and_then(|v| coerce_to_u64(v));
                    }
                    "embedding_dimensions" => {
                        llm.embedding_dimensions = raw_value.and_then(|v| coerce_to_u64(v));
                    }
                    "embedding_count" => {
                        llm.embedding_count = raw_value.and_then(|v| coerce_to_u64(v));
                    }
                    _ => {}
                }
            }
            MappingRule::EventSource(evt_mapping) => {
                let messages = extract_messages_from_events(span, &evt_mapping.event_name, &evt_mapping.content_attribute);
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
            if *i >= 0 { Some(*i as u64) } else { None }
        }
        AttributeValue::Float(f) => {
            if *f >= 0.0 { Some(*f as u64) } else { None }
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
        .filter_map(|e| {
            let content = e
                .attributes
                .get(content_attribute)
                .map(|v| v.as_display_string());
            Some(LlmMessage {
                role: "user".to_string(),
                content,
            })
        })
        .collect()
}
