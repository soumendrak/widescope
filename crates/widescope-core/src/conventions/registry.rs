use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::models::trace::ParseWarning;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Convention {
    pub name: String,
    pub version: Option<String>,
    pub detect: DetectRule,
    pub mappings: HashMap<String, MappingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectRule {
    pub attribute_prefix: Option<String>,
    pub any_key_present: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MappingRule {
    Attribute(AttributeMapping),
    EventSource(EventSourceMapping),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeMapping {
    pub attribute: String,
    #[serde(rename = "type")]
    pub value_type: Option<String>,
    pub values: Option<HashMap<String, String>>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSourceMapping {
    pub source: String,
    pub event_name: String,
    pub content_attribute: String,
}

pub struct LoadResult {
    pub conventions: Vec<Convention>,
    pub warnings: Vec<ParseWarning>,
}

pub fn load_conventions(json_array: &str) -> LoadResult {
    let mut conventions = Vec::new();
    let mut warnings = Vec::new();

    let parsed: serde_json::Value = match serde_json::from_str(json_array) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(ParseWarning::new(
                "CONVENTION_ERROR",
                format!("Failed to parse conventions array: {}", e),
            ));
            return LoadResult { conventions, warnings };
        }
    };

    let arr = match parsed.as_array() {
        Some(a) => a,
        None => {
            warnings.push(ParseWarning::new(
                "CONVENTION_ERROR",
                "Conventions input is not a JSON array".to_string(),
            ));
            return LoadResult { conventions, warnings };
        }
    };

    for (i, item) in arr.iter().enumerate() {
        match serde_json::from_value::<Convention>(item.clone()) {
            Ok(c) => conventions.push(c),
            Err(e) => {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown");
                warnings.push(
                    ParseWarning::new(
                        "CONVENTION_ERROR",
                        format!("Convention #{} ('{}') failed to parse: {}", i, name, e),
                    )
                    .with_context(serde_json::json!({
                        "file_name": name,
                        "parse_error": e.to_string()
                    })),
                );
            }
        }
    }

    LoadResult { conventions, warnings }
}
