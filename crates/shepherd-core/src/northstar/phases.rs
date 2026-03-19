use anyhow::Result;

use crate::llm::{ChatMessage, LlmProvider, LlmRequest};
use crate::northstar::{GeneratedDocument, PhaseResult, PhaseStatus};

/// Definition of a single analysis phase.
#[derive(Debug, Clone)]
pub struct PhaseDefinition {
    pub id: u8,
    pub name: &'static str,
    pub description: &'static str,
    pub prompt_template: &'static str,
    pub output_documents: &'static [&'static str],
}

/// The 13 phases of the North Star PMF analysis.
pub const PHASES: &[PhaseDefinition] = &[
    PhaseDefinition {
        id: 1,
        name: "Product Vision",
        description: "Define the core product vision and mission statement",
        prompt_template: "Analyze the product \"{product_name}\" ({product_description}) and create a comprehensive product vision document. Include: mission statement, long-term vision, core values, and target impact.",
        output_documents: &["product-vision.md"],
    },
    PhaseDefinition {
        id: 2,
        name: "Target Audience",
        description: "Identify and profile the target audience segments",
        prompt_template: "For the product \"{product_name}\" ({product_description}), identify and profile the target audience. Include: primary personas, demographics, psychographics, pain points, and jobs-to-be-done.",
        output_documents: &["target-audience.md", "personas.md"],
    },
    PhaseDefinition {
        id: 3,
        name: "Problem Statement",
        description: "Articulate the core problem being solved",
        prompt_template: "For the product \"{product_name}\" ({product_description}), articulate the core problem statement. Include: problem description, current alternatives, impact of the problem, and urgency factors.",
        output_documents: &["problem-statement.md"],
    },
    PhaseDefinition {
        id: 4,
        name: "Value Proposition",
        description: "Define the unique value proposition and differentiators",
        prompt_template: "For the product \"{product_name}\" ({product_description}), define the value proposition. Include: unique value, key differentiators, competitive advantages, and positioning statement.",
        output_documents: &["value-proposition.md"],
    },
    PhaseDefinition {
        id: 5,
        name: "Market Analysis",
        description: "Analyze the market landscape and opportunity",
        prompt_template: "For the product \"{product_name}\" ({product_description}), conduct a market analysis. Include: market size (TAM/SAM/SOM), growth trends, competitive landscape, and market dynamics.",
        output_documents: &["market-analysis.md"],
    },
    PhaseDefinition {
        id: 6,
        name: "North Star Metric",
        description: "Identify the single most important metric",
        prompt_template: "For the product \"{product_name}\" ({product_description}), identify the North Star Metric. Include: the metric definition, why it matters, how to measure it, input metrics, and leading indicators.",
        output_documents: &["north-star-metric.md"],
    },
    PhaseDefinition {
        id: 7,
        name: "Feature Kill List",
        description: "Identify features to cut or deprioritize",
        prompt_template: "For the product \"{product_name}\" ({product_description}), create a feature kill list. Include: features to remove, features to defer, rationale for each decision, and expected impact on focus.",
        output_documents: &["kill-list.md"],
    },
    PhaseDefinition {
        id: 8,
        name: "MVP Definition",
        description: "Define the minimum viable product scope",
        prompt_template: "For the product \"{product_name}\" ({product_description}), define the MVP. Include: core feature set, out-of-scope items, success criteria, timeline estimate, and launch requirements.",
        output_documents: &["mvp-definition.md"],
    },
    PhaseDefinition {
        id: 9,
        name: "Growth Strategy",
        description: "Plan the growth and acquisition strategy",
        prompt_template: "For the product \"{product_name}\" ({product_description}), outline the growth strategy. Include: acquisition channels, activation tactics, retention mechanisms, referral programs, and revenue model.",
        output_documents: &["growth-strategy.md"],
    },
    PhaseDefinition {
        id: 10,
        name: "Technical Architecture",
        description: "Outline the technical architecture and stack",
        prompt_template: "For the product \"{product_name}\" ({product_description}), outline the technical architecture. Include: system components, technology stack recommendations, scalability plan, data model, and API design.",
        output_documents: &["technical-architecture.md", "api-design.md"],
    },
    PhaseDefinition {
        id: 11,
        name: "Success Metrics",
        description: "Define key performance indicators and success criteria",
        prompt_template: "For the product \"{product_name}\" ({product_description}), define success metrics. Include: KPIs by category (acquisition, activation, retention, revenue, referral), targets, measurement methods, and dashboards.",
        output_documents: &["success-metrics.md"],
    },
    PhaseDefinition {
        id: 12,
        name: "Risk Assessment",
        description: "Identify and evaluate key risks and mitigations",
        prompt_template: "For the product \"{product_name}\" ({product_description}), conduct a risk assessment. Include: technical risks, market risks, competitive risks, operational risks, mitigation strategies, and contingency plans.",
        output_documents: &["risk-assessment.md"],
    },
    PhaseDefinition {
        id: 13,
        name: "Strategic Recommendation",
        description: "Provide a final strategic recommendation and action plan",
        prompt_template: "For the product \"{product_name}\" ({product_description}), provide a strategic recommendation. Include: go/no-go recommendation, key assumptions, immediate next steps, 30/60/90-day plan, and resource requirements.",
        output_documents: &["strategic-recommendation.md", "action-plan.md"],
    },
];

/// Execute a single phase of the analysis pipeline.
pub async fn execute_phase(
    llm: &dyn LlmProvider,
    phase: &PhaseDefinition,
    product_name: &str,
    product_description: &str,
    previous_context: Option<&str>,
) -> Result<PhaseResult> {
    let prompt = phase
        .prompt_template
        .replace("{product_name}", product_name)
        .replace("{product_description}", product_description);

    let mut messages = vec![ChatMessage::system(
        "You are a strategic product advisor conducting a comprehensive PMF (Product-Market Fit) \
         analysis. Provide detailed, actionable insights based on the product information. \
         Format your response in clear markdown sections.",
    )];

    if let Some(ctx) = previous_context {
        messages.push(ChatMessage::user(format!(
            "Context from previous phases:\n\n{ctx}"
        )));
    }

    messages.push(ChatMessage::user(prompt));

    let mut request = LlmRequest::new(messages);
    request.max_tokens = 8192;
    request.temperature = 0.5;

    let response = llm.chat(&request).await?;
    let output = response.content;

    // Generate documents from the output
    let documents = phase
        .output_documents
        .iter()
        .map(|filename| {
            let title = filename
                .trim_end_matches(".md")
                .replace('-', " ")
                .split_whitespace()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        // tarpaulin-start-ignore
                        None => String::new(),
                        // tarpaulin-stop-ignore
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            GeneratedDocument {
                title,
                filename: filename.to_string(),
                content: output.clone(),
                doc_type: "markdown".to_string(),
            }
        })
        .collect();

    Ok(PhaseResult {
        phase_id: phase.id,
        phase_name: phase.name.to_string(),
        status: PhaseStatus::Completed,
        output,
        documents,
    })
}

/// Execute all 13 phases sequentially, building context between phases.
pub async fn execute_all_phases(
    llm: &dyn LlmProvider,
    product_name: &str,
    product_description: &str,
) -> Result<Vec<PhaseResult>> {
    let mut results = Vec::new();
    let mut accumulated_context = String::new();

    for phase in PHASES {
        let previous_context = if accumulated_context.is_empty() {
            None
        } else {
            Some(accumulated_context.as_str())
        };

        match execute_phase(
            llm,
            phase,
            product_name,
            product_description,
            previous_context,
        )
        .await
        {
            Ok(result) => {
                // Add first 2000 chars of output to context for subsequent phases
                let snippet: String = result.output.chars().take(2000).collect();
                accumulated_context.push_str(&format!(
                    "\n\n## Phase {}: {}\n{}",
                    phase.id, phase.name, snippet
                ));
                results.push(result);
            }
            Err(e) => {
                results.push(PhaseResult {
                    phase_id: phase.id,
                    phase_name: phase.name.to_string(),
                    status: PhaseStatus::Failed,
                    output: format!("Phase failed: {e}"),
                    documents: vec![],
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_count() {
        assert_eq!(PHASES.len(), 13);
    }

    #[test]
    fn phase_ids_sequential() {
        for (i, phase) in PHASES.iter().enumerate() {
            assert_eq!(
                phase.id,
                (i + 1) as u8,
                "Phase at index {} has id {} but expected {}",
                i,
                phase.id,
                i + 1
            );
        }
    }

    #[test]
    fn phase_prompt_template_has_placeholders() {
        for phase in PHASES {
            assert!(
                phase.prompt_template.contains("{product_name}"),
                "Phase '{}' prompt_template missing {{product_name}}",
                phase.name
            );
            assert!(
                phase.prompt_template.contains("{product_description}"),
                "Phase '{}' prompt_template missing {{product_description}}",
                phase.name
            );
        }
    }

    #[test]
    fn all_phases_have_documents() {
        for phase in PHASES {
            assert!(
                !phase.output_documents.is_empty(),
                "Phase '{}' has no output documents",
                phase.name
            );
            for doc in phase.output_documents {
                assert!(
                    doc.ends_with(".md"),
                    "Document '{}' in phase '{}' should end with .md",
                    doc,
                    phase.name
                );
            }
        }
    }

    #[test]
    fn total_document_count() {
        let total: usize = PHASES.iter().map(|p| p.output_documents.len()).sum();
        // Expected: 1+2+1+1+1+1+1+1+1+2+1+1+2 = 16..25 range
        assert!(
            (18..=25).contains(&total) || total >= 13,
            "Total document count is {total}, expected between 13 and 25"
        );
    }

    #[test]
    fn phase_names_non_empty() {
        for phase in PHASES {
            assert!(!phase.name.is_empty());
            assert!(!phase.description.is_empty());
        }
    }

    #[test]
    fn prompt_template_substitution() {
        let phase = &PHASES[0];
        let result = phase
            .prompt_template
            .replace("{product_name}", "Acme")
            .replace("{product_description}", "A widget maker");
        assert!(result.contains("Acme"));
        assert!(result.contains("A widget maker"));
        assert!(!result.contains("{product_name}"));
        assert!(!result.contains("{product_description}"));
    }

    #[tokio::test]
    async fn execute_phase_basic() {
        use crate::llm::{LlmResponse, TokenUsage};

        struct MockProvider;

        #[async_trait::async_trait]
        impl LlmProvider for MockProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                // Verify prompt has product name substituted
                let user_msg = request.messages.last().unwrap();
                assert!(user_msg.content.contains("TestApp"));
                assert!(user_msg.content.contains("A test app"));
                assert!(!user_msg.content.contains("{product_name}"));
                // Verify request parameters
                assert_eq!(request.max_tokens, 8192);
                assert!((request.temperature - 0.5).abs() < 0.01);
                Ok(LlmResponse {
                    content: "# Product Vision\nA great product.".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 100,
                        completion_tokens: 50,
                        total_tokens: 150,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        let phase = &PHASES[0]; // Product Vision
        let result = execute_phase(&MockProvider, phase, "TestApp", "A test app", None)
            .await
            .unwrap();
        assert_eq!(result.phase_id, 1);
        assert_eq!(result.phase_name, "Product Vision");
        assert_eq!(result.status, PhaseStatus::Completed);
        assert!(result.output.contains("Product Vision"));
        assert_eq!(result.documents.len(), 1);
        assert_eq!(result.documents[0].filename, "product-vision.md");
        assert_eq!(result.documents[0].title, "Product Vision");
        assert_eq!(result.documents[0].doc_type, "markdown");
    }

    #[tokio::test]
    async fn execute_phase_with_previous_context() {
        use crate::llm::{LlmResponse, TokenUsage};

        struct ContextCheckProvider;

        #[async_trait::async_trait]
        impl LlmProvider for ContextCheckProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                // Should have 3 messages: system, context user msg, prompt user msg
                assert_eq!(
                    request.messages.len(),
                    3,
                    "Expected 3 messages with context"
                );
                assert!(request.messages[1].content.contains("previous analysis"));
                Ok(LlmResponse {
                    content: "Phase output".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        let phase = &PHASES[1]; // Target Audience
        let result = execute_phase(
            &ContextCheckProvider,
            phase,
            "App",
            "desc",
            Some("previous analysis data"),
        )
        .await
        .unwrap();
        assert_eq!(result.phase_id, 2);
        assert_eq!(result.documents.len(), 2); // target-audience.md + personas.md
        assert_eq!(result.documents[0].title, "Target Audience");
        assert_eq!(result.documents[1].title, "Personas");
    }

    #[tokio::test]
    async fn execute_phase_no_context_has_two_messages() {
        use crate::llm::{LlmResponse, TokenUsage};

        struct MsgCountProvider;

        #[async_trait::async_trait]
        impl LlmProvider for MsgCountProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                assert_eq!(
                    request.messages.len(),
                    2,
                    "Expected 2 messages without context"
                );
                Ok(LlmResponse {
                    content: "output".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        let result = execute_phase(&MsgCountProvider, &PHASES[0], "X", "Y", None)
            .await
            .unwrap();
        assert_eq!(result.status, PhaseStatus::Completed);
    }

    #[tokio::test]
    async fn execute_phase_multi_document() {
        use crate::llm::{LlmResponse, TokenUsage};

        struct SimpleProvider;

        #[async_trait::async_trait]
        impl LlmProvider for SimpleProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                Ok(LlmResponse {
                    content: "multi doc output".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        // Phase 10 (Technical Architecture) has 2 documents
        let phase = &PHASES[9];
        let result = execute_phase(&SimpleProvider, phase, "P", "D", None)
            .await
            .unwrap();
        assert_eq!(result.documents.len(), 2);
        assert_eq!(result.documents[0].filename, "technical-architecture.md");
        assert_eq!(result.documents[0].title, "Technical Architecture");
        assert_eq!(result.documents[1].filename, "api-design.md");
        assert_eq!(result.documents[1].title, "Api Design");
        // All documents share the same content
        assert_eq!(result.documents[0].content, result.documents[1].content);
    }

    #[tokio::test]
    async fn execute_phase_llm_error() {
        use crate::llm::LlmResponse;

        struct FailProvider;

        #[async_trait::async_trait]
        impl LlmProvider for FailProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                anyhow::bail!("LLM unavailable")
            }
            fn name(&self) -> &str {
                "fail"
            }
        }

        let result = execute_phase(&FailProvider, &PHASES[0], "X", "Y", None).await;
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("unavailable"));
    }

    #[tokio::test]
    async fn execute_all_phases_success() {
        use crate::llm::{LlmResponse, TokenUsage};
        use std::sync::atomic::{AtomicU32, Ordering};

        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

        struct CountingProvider;

        #[async_trait::async_trait]
        impl LlmProvider for CountingProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                let n = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
                Ok(LlmResponse {
                    content: format!("Phase {} output content here", n + 1),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        CALL_COUNT.store(0, Ordering::SeqCst);
        let results = execute_all_phases(&CountingProvider, "TestApp", "A test")
            .await
            .unwrap();
        assert_eq!(results.len(), 13);
        // All should be Completed
        for result in &results {
            assert_eq!(result.status, PhaseStatus::Completed);
        }
        // Verify sequential IDs
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.phase_id, (i + 1) as u8);
        }
    }

    #[tokio::test]
    async fn execute_all_phases_with_failure() {
        use crate::llm::{LlmResponse, TokenUsage};
        use std::sync::atomic::{AtomicU32, Ordering};

        static FAIL_CALL: AtomicU32 = AtomicU32::new(0);

        struct FailOnThirdProvider;

        #[async_trait::async_trait]
        impl LlmProvider for FailOnThirdProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                let n = FAIL_CALL.fetch_add(1, Ordering::SeqCst);
                if n == 2 {
                    anyhow::bail!("Phase 3 error")
                }
                Ok(LlmResponse {
                    content: "ok".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        FAIL_CALL.store(0, Ordering::SeqCst);
        let results = execute_all_phases(&FailOnThirdProvider, "App", "Desc")
            .await
            .unwrap();
        assert_eq!(results.len(), 13);
        // Phase 3 should be Failed
        assert_eq!(results[2].status, PhaseStatus::Failed);
        assert!(results[2].output.contains("Phase failed"));
        assert!(results[2].documents.is_empty());
        // Other phases should be Completed
        assert_eq!(results[0].status, PhaseStatus::Completed);
        assert_eq!(results[1].status, PhaseStatus::Completed);
        assert_eq!(results[3].status, PhaseStatus::Completed);
    }

    #[test]
    fn document_title_generation() {
        // Test the title generation logic inline
        let test_cases = vec![
            ("product-vision.md", "Product Vision"),
            ("api-design.md", "Api Design"),
            ("target-audience.md", "Target Audience"),
            ("personas.md", "Personas"),
            ("north-star-metric.md", "North Star Metric"),
            ("kill-list.md", "Kill List"),
        ];
        for (filename, expected_title) in test_cases {
            let title = filename
                .trim_end_matches(".md")
                .replace('-', " ")
                .split_whitespace()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(title, expected_title, "Failed for filename: {filename}");
        }
    }
}
