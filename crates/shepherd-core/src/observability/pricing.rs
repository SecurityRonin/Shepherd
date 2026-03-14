use serde::{Deserialize, Serialize};

/// Cost per million tokens for a specific model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model_id: String,
    pub provider: String,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
}

impl ModelPricing {
    /// Calculate cost for a given number of input and output tokens.
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }
}

/// Built-in pricing table for major models (USD per million tokens).
/// Prices as of early 2026. Users can override with custom pricing.
pub fn default_pricing() -> Vec<ModelPricing> {
    vec![
        // Anthropic Claude
        ModelPricing {
            model_id: "claude-opus-4".into(),
            provider: "anthropic".into(),
            input_cost_per_million: 15.0,
            output_cost_per_million: 75.0,
        },
        ModelPricing {
            model_id: "claude-sonnet-4".into(),
            provider: "anthropic".into(),
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
        },
        ModelPricing {
            model_id: "claude-haiku-3.5".into(),
            provider: "anthropic".into(),
            input_cost_per_million: 0.80,
            output_cost_per_million: 4.0,
        },
        // OpenAI GPT
        ModelPricing {
            model_id: "gpt-4o".into(),
            provider: "openai".into(),
            input_cost_per_million: 2.50,
            output_cost_per_million: 10.0,
        },
        ModelPricing {
            model_id: "gpt-4o-mini".into(),
            provider: "openai".into(),
            input_cost_per_million: 0.15,
            output_cost_per_million: 0.60,
        },
        ModelPricing {
            model_id: "o3".into(),
            provider: "openai".into(),
            input_cost_per_million: 10.0,
            output_cost_per_million: 40.0,
        },
        ModelPricing {
            model_id: "o3-mini".into(),
            provider: "openai".into(),
            input_cost_per_million: 1.10,
            output_cost_per_million: 4.40,
        },
        ModelPricing {
            model_id: "codex-mini".into(),
            provider: "openai".into(),
            input_cost_per_million: 1.50,
            output_cost_per_million: 6.0,
        },
        // Google Gemini
        ModelPricing {
            model_id: "gemini-2.5-pro".into(),
            provider: "google".into(),
            input_cost_per_million: 1.25,
            output_cost_per_million: 10.0,
        },
        ModelPricing {
            model_id: "gemini-2.5-flash".into(),
            provider: "google".into(),
            input_cost_per_million: 0.15,
            output_cost_per_million: 0.60,
        },
        ModelPricing {
            model_id: "gemini-2.0-flash".into(),
            provider: "google".into(),
            input_cost_per_million: 0.10,
            output_cost_per_million: 0.40,
        },
        // Local / free
        ModelPricing {
            model_id: "ollama".into(),
            provider: "ollama".into(),
            input_cost_per_million: 0.0,
            output_cost_per_million: 0.0,
        },
    ]
}

/// Look up pricing for a model. Tries exact match first, then prefix match.
pub fn find_pricing(model_id: &str) -> Option<ModelPricing> {
    let table = default_pricing();

    // Exact match
    if let Some(p) = table.iter().find(|p| p.model_id == model_id) {
        return Some(p.clone());
    }

    // Prefix match (e.g., "claude-sonnet-4-20250514" matches "claude-sonnet-4")
    if let Some(p) = table.iter().find(|p| model_id.starts_with(&p.model_id)) {
        return Some(p.clone());
    }

    // Provider-level fallback (e.g., any "claude-" model uses sonnet pricing)
    if model_id.starts_with("claude-") {
        return table.iter().find(|p| p.model_id == "claude-sonnet-4").cloned();
    }
    if model_id.starts_with("gpt-") {
        return table.iter().find(|p| p.model_id == "gpt-4o").cloned();
    }
    if model_id.starts_with("gemini-") {
        return table.iter().find(|p| p.model_id == "gemini-2.5-flash").cloned();
    }

    None
}

/// Calculate cost from a model ID and token counts.
pub fn estimate_cost(model_id: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    match find_pricing(model_id) {
        Some(pricing) => pricing.calculate_cost(input_tokens, output_tokens),
        None => 0.0, // Unknown model, can't estimate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_cost_basic() {
        let pricing = ModelPricing {
            model_id: "test".into(),
            provider: "test".into(),
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
        };
        // 1M input + 1M output = $3 + $15 = $18
        let cost = pricing.calculate_cost(1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_cost_small_usage() {
        let pricing = ModelPricing {
            model_id: "test".into(),
            provider: "test".into(),
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
        };
        // 1000 input + 500 output = $0.003 + $0.0075 = $0.0105
        let cost = pricing.calculate_cost(1000, 500);
        assert!((cost - 0.0105).abs() < 1e-10);
    }

    #[test]
    fn calculate_cost_zero_tokens() {
        let pricing = ModelPricing {
            model_id: "test".into(),
            provider: "test".into(),
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
        };
        assert!((pricing.calculate_cost(0, 0)).abs() < f64::EPSILON);
    }

    #[test]
    fn default_pricing_has_entries() {
        let table = default_pricing();
        assert!(table.len() >= 10);
    }

    #[test]
    fn find_pricing_exact_match() {
        let p = find_pricing("claude-sonnet-4").unwrap();
        assert_eq!(p.provider, "anthropic");
        assert!((p.input_cost_per_million - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn find_pricing_prefix_match() {
        let p = find_pricing("claude-sonnet-4-20250514").unwrap();
        assert_eq!(p.model_id, "claude-sonnet-4");
    }

    #[test]
    fn find_pricing_provider_fallback() {
        let p = find_pricing("claude-unknown-model").unwrap();
        assert_eq!(p.model_id, "claude-sonnet-4"); // fallback
    }

    #[test]
    fn find_pricing_unknown_returns_none() {
        assert!(find_pricing("totally-unknown-model").is_none());
    }

    #[test]
    fn find_pricing_ollama_is_free() {
        let p = find_pricing("ollama").unwrap();
        assert!((p.input_cost_per_million).abs() < f64::EPSILON);
        assert!((p.output_cost_per_million).abs() < f64::EPSILON);
    }

    #[test]
    fn estimate_cost_known_model() {
        let cost = estimate_cost("claude-sonnet-4", 10_000, 5_000);
        // $3/M input * 0.01 + $15/M output * 0.005 = $0.03 + $0.075 = $0.105
        assert!((cost - 0.105).abs() < 1e-10);
    }

    #[test]
    fn estimate_cost_unknown_model_returns_zero() {
        let cost = estimate_cost("unknown-model", 10_000, 5_000);
        assert!((cost).abs() < f64::EPSILON);
    }

    #[test]
    fn pricing_serde_roundtrip() {
        let pricing = ModelPricing {
            model_id: "test-model".into(),
            provider: "test".into(),
            input_cost_per_million: 2.5,
            output_cost_per_million: 10.0,
        };
        let json = serde_json::to_string(&pricing).unwrap();
        let parsed: ModelPricing = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model_id, "test-model");
        assert!((parsed.input_cost_per_million - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn opus_more_expensive_than_sonnet() {
        let opus = find_pricing("claude-opus-4").unwrap();
        let sonnet = find_pricing("claude-sonnet-4").unwrap();
        assert!(opus.input_cost_per_million > sonnet.input_cost_per_million);
        assert!(opus.output_cost_per_million > sonnet.output_cost_per_million);
    }

    #[test]
    fn estimate_cost_gpt4o() {
        // gpt-4o: $2.50/M input, $10/M output
        // 1M input + 0 output = $2.50
        let cost = estimate_cost("gpt-4o", 1_000_000, 0);
        assert!((cost - 2.50).abs() < 1e-10);
    }

    #[test]
    fn estimate_cost_zero_tokens_is_zero() {
        let cost = estimate_cost("claude-opus-4", 0, 0);
        assert!((cost).abs() < f64::EPSILON);
    }

    #[test]
    fn find_pricing_gemini_fallback() {
        // "gemini-unknown" → falls back to gemini-2.5-flash
        let p = find_pricing("gemini-unknown-model").unwrap();
        assert_eq!(p.model_id, "gemini-2.5-flash");
    }

    #[test]
    fn find_pricing_gpt_fallback() {
        // "gpt-5" → falls back to gpt-4o
        let p = find_pricing("gpt-5").unwrap();
        assert_eq!(p.model_id, "gpt-4o");
    }

    #[test]
    fn model_pricing_clone() {
        let p = find_pricing("claude-sonnet-4").unwrap();
        let cloned = p.clone();
        assert_eq!(cloned.model_id, "claude-sonnet-4");
        assert!((cloned.input_cost_per_million - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_cost_only_output_tokens() {
        let pricing = ModelPricing {
            model_id: "test".into(),
            provider: "test".into(),
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
        };
        // 0 input, 1M output = $15
        let cost = pricing.calculate_cost(0, 1_000_000);
        assert!((cost - 15.0).abs() < f64::EPSILON);
    }
}
