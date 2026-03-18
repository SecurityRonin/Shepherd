use anyhow::Result;
use serde::Deserialize;

use crate::llm::{ChatMessage, LlmProvider, LlmRequest};
use crate::namegen::NameGenInput;

/// Raw candidate from LLM JSON output, used for deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct RawCandidate {
    pub name: String,
    pub tagline: Option<String>,
    pub reasoning: String,
}

/// Brainstorm names using an LLM provider.
///
/// Sends a system prompt instructing the LLM to return a JSON array of
/// name candidates, each with name, tagline, and reasoning fields.
pub async fn brainstorm_names(
    provider: &dyn LlmProvider,
    input: &NameGenInput,
) -> Result<Vec<RawCandidate>> {
    let system_prompt = r#"You are a creative product naming expert. Generate unique, memorable product names.

Return ONLY a JSON array of objects with these fields:
- "name": the product name (short, memorable, lowercase, no spaces)
- "tagline": a short catchy tagline for the product (optional)
- "reasoning": brief explanation of why this name works

Example format:
[
  {"name": "acme", "tagline": "Build anything", "reasoning": "Simple, memorable, classic"}
]

Return ONLY valid JSON. No markdown, no explanation, just the JSON array."#;

    let vibes_str = if input.vibes.is_empty() {
        String::new()
    } else {
        format!("\nDesired vibes/feel: {}", input.vibes.join(", "))
    };

    let user_prompt = format!(
        "Generate {} unique product name ideas for:\n{}{}\n\nReturn as a JSON array.",
        input.count, input.description, vibes_str
    );

    let mut request = LlmRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_prompt),
    ]);
    request.temperature = 0.9;

    let response = provider.chat(&request).await?;
    parse_brainstorm_response(&response.content)
}

/// Parse the LLM brainstorm response, handling markdown code fences.
pub fn parse_brainstorm_response(content: &str) -> Result<Vec<RawCandidate>> {
    let trimmed = content.trim();

    // Strip markdown code fences if present
    let json_str = if trimmed.starts_with("```") {
        let without_opening = if let Some(after_first_newline) = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
        {
            after_first_newline.trim_start_matches('\n')
        } else {
            trimmed // tarpaulin-start-ignore
        }; // tarpaulin-stop-ignore

        // Remove trailing code fence
        if let Some(stripped) = without_opening.strip_suffix("```") {
            stripped.trim()
        } else {
            without_opening.trim()
        }
    } else {
        trimmed
    };

    let candidates: Vec<RawCandidate> = serde_json::from_str(json_str)?;

    // Post-process: trim names and filter out empty ones
    let candidates: Vec<RawCandidate> = candidates
        .into_iter()
        .map(|mut c| {
            c.name = c.name.trim().to_string();
            c.tagline = c.tagline.map(|t| {
                let trimmed = t.trim().to_string();
                if trimmed.is_empty() {
                    return trimmed;
                }
                trimmed
            });
            // Filter out empty taglines
            if c.tagline.as_ref().is_some_and(|t| t.is_empty()) {
                c.tagline = None;
            }
            c
        })
        .filter(|c| !c.name.is_empty())
        .collect();

    Ok(candidates)
}

/// Scan a name for potential negative associations using an LLM.
///
/// Uses a low temperature (0.3) for more analytical/factual output.
/// Returns a list of concerns, or empty if none found.
pub async fn scan_negative_associations(
    provider: &dyn LlmProvider,
    name: &str,
) -> Result<Vec<String>> {
    let system_prompt = r#"You are a brand safety analyst. Given a product name, identify any negative associations, offensive meanings, unfortunate translations, or cultural sensitivities.

Return ONLY a JSON array of strings, where each string is a brief concern.
If there are no concerns, return an empty array: []

Example: ["Sounds like a slur in German", "Associated with a failed product"]

Return ONLY valid JSON. No markdown, no explanation."#;

    let user_prompt = format!(
        "Analyze the product name \"{name}\" for any negative associations, offensive meanings, \
         unfortunate translations in other languages, or cultural sensitivities."
    );

    let mut request = LlmRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_prompt),
    ]);
    request.temperature = 0.3;

    let response = provider.chat(&request).await?;
    let content = response.content.trim();

    // Strip code fences if present
    let json_str = if content.starts_with("```") {
        let without_opening = content
            .strip_prefix("```json")
            .or_else(|| content.strip_prefix("```"))
            .unwrap_or(content)
            .trim_start_matches('\n');

        without_opening
            .strip_suffix("```")
            .unwrap_or(without_opening)
            .trim()
    } else {
        content
    };

    let associations: Vec<String> = serde_json::from_str(json_str).unwrap_or_default();
    Ok(associations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let input = r#"[
            {"name": "acme", "tagline": "Build anything", "reasoning": "Classic name"},
            {"name": "nexus", "tagline": "Connect everything", "reasoning": "Implies connections"}
        ]"#;

        let result = parse_brainstorm_response(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "acme");
        assert_eq!(result[0].tagline.as_deref(), Some("Build anything"));
        assert_eq!(result[0].reasoning, "Classic name");
        assert_eq!(result[1].name, "nexus");
    }

    #[test]
    fn test_parse_with_code_fences() {
        let input = r#"```json
[
    {"name": "acme", "tagline": "Build anything", "reasoning": "Classic name"}
]
```"#;

        let result = parse_brainstorm_response(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "acme");
    }

    #[test]
    fn test_parse_with_generic_code_fences() {
        let input = r#"```
[
    {"name": "acme", "tagline": "Build anything", "reasoning": "Classic name"}
]
```"#;

        let result = parse_brainstorm_response(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "acme");
    }

    #[test]
    fn test_parse_invalid_json() {
        let input = "this is not json at all";
        let result = parse_brainstorm_response(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_trims_names_and_empty_taglines() {
        let input = r#"[
            {"name": "  spaced  ", "tagline": "  ", "reasoning": "test"},
            {"name": "  ", "tagline": null, "reasoning": "empty name filtered"},
            {"name": "good", "tagline": "  nice tagline  ", "reasoning": "kept"}
        ]"#;

        let result = parse_brainstorm_response(input).unwrap();
        // Empty-after-trim name should be filtered out
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "spaced");
        // Empty tagline should become None
        assert!(result[0].tagline.is_none());
        assert_eq!(result[1].name, "good");
        assert_eq!(result[1].tagline.as_deref(), Some("nice tagline"));
    }

    #[tokio::test]
    async fn brainstorm_names_with_mock() {
        use crate::llm::{LlmRequest, LlmResponse, TokenUsage};

        struct MockProvider;

        #[async_trait::async_trait]
        impl crate::llm::LlmProvider for MockProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                assert!(request.messages[1].content.contains("3 unique"));
                assert!((request.temperature - 0.9).abs() < 0.01);
                Ok(LlmResponse {
                    content: r#"[{"name":"alpha","tagline":"Go fast","reasoning":"Speed"},{"name":"beta","tagline":null,"reasoning":"Testing"}]"#.to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                })
            }
            fn name(&self) -> &str { "mock" }
        }

        let input = NameGenInput {
            description: "A widget".into(),
            vibes: vec![],
            count: 3,
        };
        let result = brainstorm_names(&MockProvider, &input).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[0].tagline.as_deref(), Some("Go fast"));
        assert_eq!(result[1].name, "beta");
        assert!(result[1].tagline.is_none()); // null tagline
    }

    #[tokio::test]
    async fn brainstorm_names_with_vibes() {
        use crate::llm::{LlmRequest, LlmResponse, TokenUsage};

        struct VibeCheckProvider;

        #[async_trait::async_trait]
        impl crate::llm::LlmProvider for VibeCheckProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                let user_msg = &request.messages[1].content;
                assert!(user_msg.contains("fast, modern"), "Should include vibes: {user_msg}");
                Ok(LlmResponse {
                    content: r#"[{"name":"viber","tagline":"Feel it","reasoning":"Vibes"}]"#.to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                })
            }
            fn name(&self) -> &str { "mock" }
        }

        let input = NameGenInput {
            description: "A tool".into(),
            vibes: vec!["fast".into(), "modern".into()],
            count: 5,
        };
        let result = brainstorm_names(&VibeCheckProvider, &input).await.unwrap();
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn scan_negative_associations_found() {
        use crate::llm::{LlmRequest, LlmResponse, TokenUsage};

        struct NegProvider;

        #[async_trait::async_trait]
        impl crate::llm::LlmProvider for NegProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                assert!(request.messages[1].content.contains("badname"));
                Ok(LlmResponse {
                    content: r#"["Offensive in language X", "Similar to failed product Y"]"#.to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                })
            }
            fn name(&self) -> &str { "mock" }
        }

        let result = scan_negative_associations(&NegProvider, "badname").await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("Offensive"));
    }

    #[tokio::test]
    async fn scan_negative_associations_empty() {
        use crate::llm::{LlmRequest, LlmResponse, TokenUsage};

        struct CleanProvider;

        #[async_trait::async_trait]
        impl crate::llm::LlmProvider for CleanProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                Ok(LlmResponse {
                    content: "[]".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                })
            }
            fn name(&self) -> &str { "mock" }
        }

        let result = scan_negative_associations(&CleanProvider, "goodname").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn scan_negative_associations_code_fences() {
        use crate::llm::{LlmRequest, LlmResponse, TokenUsage};

        struct FenceProvider;

        #[async_trait::async_trait]
        impl crate::llm::LlmProvider for FenceProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                Ok(LlmResponse {
                    content: "```json\n[\"concern1\"]\n```".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                })
            }
            fn name(&self) -> &str { "mock" }
        }

        let result = scan_negative_associations(&FenceProvider, "name").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "concern1");
    }

    #[test]
    fn parse_response_opening_fence_only() {
        let input = "```json\n[{\"name\":\"test\",\"tagline\":null,\"reasoning\":\"r\"}]";
        let result = parse_brainstorm_response(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "test");
    }

    #[test]
    fn parse_response_null_tagline() {
        let input = r#"[{"name":"x","tagline":null,"reasoning":"y"}]"#;
        let result = parse_brainstorm_response(input).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].tagline.is_none());
    }

    #[test]
    fn raw_candidate_fields() {
        let c = RawCandidate {
            name: "test".into(),
            tagline: Some("cool".into()),
            reasoning: "because".into(),
        };
        assert_eq!(c.name, "test");
        assert_eq!(c.tagline.as_deref(), Some("cool"));
        assert_eq!(c.reasoning, "because");
    }
}
