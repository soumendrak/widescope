use crate::models::trace::ParseWarning;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingFile {
    pub name: String,
    pub version: Option<String>,
    pub currency: Option<String>,
    pub unit: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelRate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRate {
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub input: f64,
    pub output: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PricingTable {
    entries: Vec<ModelRate>,
    index: HashMap<String, Vec<usize>>,
}

impl PricingTable {
    pub fn new() -> Self {
        PricingTable::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn insert(&mut self, idx: usize, key: &str) {
        let normalized = normalize(key);
        if normalized.is_empty() {
            return;
        }
        self.index.entry(normalized).or_default().push(idx);
    }

    pub fn load(&mut self, json: &str) -> Result<usize, ParseWarning> {
        let file: PricingFile = serde_json::from_str(json).map_err(|e| {
            ParseWarning::new(
                "PRICING_ERROR",
                format!("Failed to parse pricing JSON: {}", e),
            )
        })?;

        let mut loaded = 0;
        for model in file.models {
            let idx = self.entries.len();
            self.insert(idx, &model.name);
            for alias in &model.aliases {
                self.insert(idx, alias);
            }
            self.entries.push(model);
            loaded += 1;
        }
        Ok(loaded)
    }

    pub fn lookup(&self, model_name: &str, provider: Option<&str>) -> Option<&ModelRate> {
        let key = normalize(model_name);
        if key.is_empty() {
            return None;
        }
        let provider_norm = provider.map(normalize);

        // Try exact match, then progressively peel trailing numeric / version
        // segments. This catches real-world names like "gpt-4o-mini-2024-07-18"
        // when only "gpt-4o-mini" is registered.
        let mut candidate_key = key;
        loop {
            if let Some(candidates) = self.index.get(&candidate_key) {
                return self.choose(candidates, provider_norm.as_deref());
            }
            candidate_key = strip_one_numeric_suffix(&candidate_key)?;
        }
    }

    fn choose(&self, candidates: &[usize], provider: Option<&str>) -> Option<&ModelRate> {
        if candidates.len() == 1 {
            return self.entries.get(candidates[0]);
        }
        if let Some(p) = provider {
            for &idx in candidates {
                if let Some(entry) = self.entries.get(idx) {
                    if normalize(&entry.provider) == p {
                        return Some(entry);
                    }
                }
            }
        }
        self.entries.get(candidates[0])
    }

    pub fn compute_cost(
        &self,
        model_name: &str,
        provider: Option<&str>,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Option<f64> {
        let rate = self.lookup(model_name, provider)?;
        let input_cost = (input_tokens as f64) * rate.input / 1_000_000.0;
        let output_cost = (output_tokens as f64) * rate.output / 1_000_000.0;
        Some(input_cost + output_cost)
    }
}

fn normalize(s: &str) -> String {
    s.trim().trim_start_matches("models/").to_ascii_lowercase()
}

fn strip_one_numeric_suffix(s: &str) -> Option<String> {
    // Peel one trailing "-<numeric>" or "-v<numeric>" segment.
    // Caller may invoke repeatedly to walk back through composite suffixes
    // like "-2024-07-18".
    let dash = s.rfind('-')?;
    let suffix = &s[dash + 1..];
    if suffix.is_empty() {
        return None;
    }
    let is_numeric = suffix.chars().all(|c| c.is_ascii_digit());
    let is_version = suffix.starts_with('v')
        && suffix.len() >= 2
        && suffix[1..].chars().all(|c| c.is_ascii_digit() || c == '.');
    if is_numeric || is_version {
        Some(s[..dash].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pricing() -> &'static str {
        r#"{
            "name": "test",
            "models": [
                { "provider": "openai", "name": "gpt-4o", "aliases": ["gpt-4o-2024-08-06"], "input": 2.5, "output": 10.0 },
                { "provider": "openai", "name": "gpt-4o-mini", "aliases": [], "input": 0.15, "output": 0.60 },
                { "provider": "anthropic", "name": "claude-3-5-sonnet", "aliases": [], "input": 3.0, "output": 15.0 },
                { "provider": "openai", "name": "text-embedding-3-small", "aliases": [], "input": 0.02, "output": 0.0 }
            ]
        }"#
    }

    #[test]
    fn loads_models() {
        let mut table = PricingTable::new();
        let n = table.load(sample_pricing()).unwrap();
        assert_eq!(n, 4);
        assert_eq!(table.len(), 4);
    }

    #[test]
    fn looks_up_canonical_name() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        let rate = table.lookup("gpt-4o", None).unwrap();
        assert_eq!(rate.name, "gpt-4o");
        assert_eq!(rate.input, 2.5);
    }

    #[test]
    fn lookup_is_case_insensitive() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        let rate = table.lookup("GPT-4o", None).unwrap();
        assert_eq!(rate.name, "gpt-4o");
    }

    #[test]
    fn looks_up_by_alias() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        let rate = table.lookup("gpt-4o-2024-08-06", None).unwrap();
        assert_eq!(rate.name, "gpt-4o");
    }

    #[test]
    fn strips_models_prefix() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        let rate = table.lookup("models/gpt-4o", None).unwrap();
        assert_eq!(rate.name, "gpt-4o");
    }

    #[test]
    fn returns_none_for_unknown() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        assert!(table.lookup("some-unknown-model", None).is_none());
    }

    #[test]
    fn strips_date_suffix_when_alias_missing() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        // alias "gpt-4o-mini-2024-07-18" is not in the test data — falls back to "gpt-4o-mini"
        let rate = table.lookup("gpt-4o-mini-2024-07-18", None).unwrap();
        assert_eq!(rate.name, "gpt-4o-mini");
    }

    #[test]
    fn computes_cost_per_million() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        // 1M input × $2.50 + 500K output × $10.00 = $2.50 + $5.00 = $7.50
        let cost = table
            .compute_cost("gpt-4o", None, 1_000_000, 500_000)
            .unwrap();
        assert!((cost - 7.50).abs() < 1e-9, "got {}", cost);
    }

    #[test]
    fn embedding_model_zero_output_cost() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        let cost = table
            .compute_cost("text-embedding-3-small", None, 1_000_000, 0)
            .unwrap();
        assert!((cost - 0.02).abs() < 1e-9, "got {}", cost);
    }

    #[test]
    fn returns_none_when_model_not_in_table() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        assert!(table
            .compute_cost("unknown-model", None, 100, 100)
            .is_none());
    }

    #[test]
    fn invalid_json_returns_warning() {
        let mut table = PricingTable::new();
        let err = table.load("not json").unwrap_err();
        assert_eq!(err.code, "PRICING_ERROR");
    }

    #[test]
    fn empty_model_name_is_ignored() {
        let mut table = PricingTable::new();
        table.load(sample_pricing()).unwrap();
        assert!(table.lookup("", None).is_none());
        assert!(table.lookup("   ", None).is_none());
    }

    #[test]
    fn provider_disambiguates_duplicate_names() {
        let json = r#"{
            "name": "test",
            "models": [
                { "provider": "alpha", "name": "shared", "aliases": [], "input": 1.0, "output": 2.0 },
                { "provider": "beta",  "name": "shared", "aliases": [], "input": 5.0, "output": 6.0 }
            ]
        }"#;
        let mut table = PricingTable::new();
        table.load(json).unwrap();
        assert_eq!(table.lookup("shared", Some("beta")).unwrap().input, 5.0);
        assert_eq!(table.lookup("shared", Some("alpha")).unwrap().input, 1.0);
    }

    #[test]
    fn shipped_pricing_file_parses() {
        let raw = include_str!("../../../../conventions/pricing.json");
        let mut table = PricingTable::new();
        let loaded = table.load(raw).unwrap();
        assert!(
            loaded >= 20,
            "expected ≥20 models in shipped pricing.json, got {}",
            loaded
        );
        // Sanity: a few well-known models must resolve.
        assert!(table.lookup("gpt-4o", None).is_some());
        assert!(table.lookup("claude-3-5-sonnet-20241022", None).is_some());
        assert!(table.lookup("gemini-1.5-flash", None).is_some());
    }

    #[test]
    fn end_to_end_cost_on_sample_otlp_fixture() {
        use crate::conventions::registry::load_conventions;
        use crate::conventions::resolver::resolve_llm_attributes;
        use crate::parsers::otlp_json::parse_otlp_with_warnings;

        // Load OTel conventions so the parser fills in model_name + tokens.
        let otel_raw = include_str!("../../../../conventions/opentelemetry.json");
        let merged = format!("[{}]", otel_raw);
        let conv = load_conventions(&merged);

        // Load the shipped pricing table.
        let pricing_raw = include_str!("../../../../conventions/pricing.json");
        let mut table = PricingTable::new();
        table.load(pricing_raw).unwrap();

        // Parse the sample trace and resolve LLM attributes.
        let trace_raw = include_str!("../../../../test-fixtures/otlp/sample_llm_pipeline.json");
        let value: serde_json::Value = serde_json::from_str(trace_raw).unwrap();
        let mut spans = parse_otlp_with_warnings(&value).unwrap().spans;
        for span in &mut spans {
            span.llm = resolve_llm_attributes(span, &conv.conventions);
        }

        // The chat span uses gpt-4o with 512 input + 256 output tokens.
        // 512×$2.50/M + 256×$10.00/M = $0.00128 + $0.00256 = $0.00384
        let chat = spans
            .iter()
            .find(|s| s.llm.as_ref().and_then(|l| l.model_name.as_deref()) == Some("gpt-4o"))
            .expect("gpt-4o span present in fixture");
        let llm = chat.llm.as_ref().unwrap();
        let cost = table
            .compute_cost(
                llm.model_name.as_deref().unwrap(),
                llm.model_provider.as_deref(),
                llm.input_tokens.unwrap(),
                llm.output_tokens.unwrap(),
            )
            .unwrap();
        assert!(
            (cost - 0.00384).abs() < 1e-9,
            "expected $0.00384 for 512 in + 256 out gpt-4o, got {}",
            cost
        );
    }
}

/// Model-name normalization: pricing tables list canonical names, traces carry
/// dated and versioned variants.
#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn normalization_strips_the_vendor_path_and_case() {
        assert_eq!(normalize("  models/Gemini-1.5-Pro "), "gemini-1.5-pro");
        assert_eq!(normalize("GPT-4o"), "gpt-4o");
    }

    #[test]
    fn one_numeric_or_version_suffix_is_peeled_at_a_time() {
        assert_eq!(
            strip_one_numeric_suffix("gpt-4o-2024"),
            Some("gpt-4o".into())
        );
        assert_eq!(
            strip_one_numeric_suffix("gpt-4o-2024-07-18"),
            Some("gpt-4o-2024-07".into())
        );
        assert_eq!(
            strip_one_numeric_suffix("claude-3-5-sonnet-v2"),
            Some("claude-3-5-sonnet".into())
        );
        assert_eq!(strip_one_numeric_suffix("gpt-4o-mini"), None);
        assert_eq!(strip_one_numeric_suffix("gpt4o"), None);
        assert_eq!(strip_one_numeric_suffix("gpt-4o-"), None);
    }
}
