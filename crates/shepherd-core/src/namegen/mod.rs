pub mod brainstorm;
pub mod rdap;
pub mod validate;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::llm::LlmProvider;

/// Input parameters for name generation.
#[derive(Debug, Clone)]
pub struct NameGenInput {
    pub description: String,
    pub vibes: Vec<String>,
    pub count: usize,
}

impl Default for NameGenInput {
    fn default() -> Self {
        Self {
            description: String::new(),
            vibes: Vec::new(),
            count: 20,
        }
    }
}

/// A candidate name with its validation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameCandidate {
    pub name: String,
    pub tagline: Option<String>,
    pub reasoning: String,
    pub validation: NameValidation,
}

/// Validation results for a name candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameValidation {
    pub domains: Vec<DomainCheck>,
    pub npm_available: Option<bool>,
    pub pypi_available: Option<bool>,
    pub github_available: Option<bool>,
    pub negative_associations: Vec<String>,
    pub overall_status: ValidationStatus,
}

impl Default for NameValidation {
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            npm_available: None,
            pypi_available: None,
            github_available: None,
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        }
    }
}

/// Result of checking a single domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCheck {
    pub domain: String,
    pub available: Option<bool>,
    pub error: Option<String>,
}

/// Overall validation status for a name candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationStatus {
    AllClear,
    Partial,
    Conflicted,
    Pending,
}

impl Default for ValidationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl ValidationStatus {
    /// Returns a numeric priority for sorting (lower = better).
    fn sort_priority(&self) -> u8 {
        match self {
            ValidationStatus::AllClear => 0,
            ValidationStatus::Partial => 1,
            ValidationStatus::Pending => 2,
            ValidationStatus::Conflicted => 3,
        }
    }
}

/// The result of a name generation run.
#[derive(Debug, Clone)]
pub struct NameGenResult {
    pub candidates: Vec<NameCandidate>,
}

impl NameGenResult {
    /// Return candidates sorted by validation status priority.
    /// AllClear first, then Partial, then Pending, then Conflicted.
    pub fn sorted(mut self) -> Self {
        self.candidates
            .sort_by_key(|c| c.validation.overall_status.sort_priority());
        self
    }
}

/// Calculate the overall validation status from individual checks.
pub fn calculate_status(validation: &NameValidation) -> ValidationStatus {
    let has_negative = !validation.negative_associations.is_empty();

    // If there are negative associations, it's conflicted
    if has_negative {
        return ValidationStatus::Conflicted;
    }

    // Gather all availability signals
    let mut any_unavailable = false;
    let mut has_any_data = false;

    // Check domains
    for domain in &validation.domains {
        if let Some(available) = domain.available {
            has_any_data = true;
            if !available {
                any_unavailable = true;
            }
        }
    }

    // Check registries
    for available in [
        validation.npm_available,
        validation.pypi_available,
        validation.github_available,
    ]
    .into_iter()
    .flatten()
    {
        has_any_data = true;
        if !available {
            any_unavailable = true;
        }
    }

    if !has_any_data {
        return ValidationStatus::Pending;
    }

    if any_unavailable {
        ValidationStatus::Partial
    } else {
        ValidationStatus::AllClear
    }
}

/// Generate names: brainstorm via LLM, validate, scan for negative associations, return sorted.
pub async fn generate_names(
    provider: &dyn LlmProvider,
    input: &NameGenInput,
) -> Result<NameGenResult> {
    // Step 1: Brainstorm names via LLM
    let raw_candidates = brainstorm::brainstorm_names(provider, input).await?;

    // Step 2: Build candidates with validation
    let mut candidates: Vec<NameCandidate> = Vec::new();
    for raw in raw_candidates {
        let name = raw.name.clone();

        // Step 3: Validate (domains + registries)
        let validation_result = validate::validate_name(&name).await;
        let (domains, npm_available, pypi_available, github_available): (
            Vec<DomainCheck>,
            Option<bool>,
            Option<bool>,
            Option<bool>,
        ) = validation_result.unwrap_or_default();

        // Step 4: Scan for negative associations
        let negative_associations = brainstorm::scan_negative_associations(provider, &name)
            .await
            .unwrap_or_default();

        let mut validation = NameValidation {
            domains,
            npm_available,
            pypi_available,
            github_available,
            negative_associations,
            overall_status: ValidationStatus::Pending,
        };

        // Step 5: Calculate status
        validation.overall_status = calculate_status(&validation);

        candidates.push(NameCandidate {
            name: raw.name,
            tagline: raw.tagline,
            reasoning: raw.reasoning,
            validation,
        });
    }

    let result = NameGenResult { candidates };
    Ok(result.sorted())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_count() {
        let input = NameGenInput::default();
        assert_eq!(input.count, 20);
    }

    #[test]
    fn test_sorting() {
        let make_candidate = |name: &str, status: ValidationStatus| NameCandidate {
            name: name.to_string(),
            tagline: None,
            reasoning: String::new(),
            validation: NameValidation {
                overall_status: status,
                ..Default::default()
            },
        };

        let result = NameGenResult {
            candidates: vec![
                make_candidate("conflicted", ValidationStatus::Conflicted),
                make_candidate("allclear", ValidationStatus::AllClear),
                make_candidate("pending", ValidationStatus::Pending),
                make_candidate("partial", ValidationStatus::Partial),
            ],
        };

        let sorted = result.sorted();
        assert_eq!(sorted.candidates[0].name, "allclear");
        assert_eq!(sorted.candidates[1].name, "partial");
        assert_eq!(sorted.candidates[2].name, "pending");
        assert_eq!(sorted.candidates[3].name, "conflicted");
    }

    #[test]
    fn test_calculate_status_all_clear() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "test.com".to_string(),
                available: Some(true),
                error: None,
            }],
            npm_available: Some(true),
            pypi_available: Some(true),
            github_available: Some(true),
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };

        assert_eq!(calculate_status(&validation), ValidationStatus::AllClear);
    }

    #[test]
    fn test_calculate_status_conflicted() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "test.com".to_string(),
                available: Some(true),
                error: None,
            }],
            npm_available: Some(true),
            pypi_available: Some(true),
            github_available: Some(true),
            negative_associations: vec!["bad association".to_string()],
            overall_status: ValidationStatus::Pending,
        };

        assert_eq!(calculate_status(&validation), ValidationStatus::Conflicted);
    }

    #[test]
    fn test_calculate_status_partial() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "test.com".to_string(),
                available: Some(false),
                error: None,
            }],
            npm_available: Some(true),
            pypi_available: Some(true),
            github_available: Some(true),
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };

        assert_eq!(calculate_status(&validation), ValidationStatus::Partial);
    }

    #[test]
    fn test_calculate_status_pending_no_data() {
        let validation = NameValidation::default();
        assert_eq!(calculate_status(&validation), ValidationStatus::Pending);
    }

    #[test]
    fn calculate_status_all_registries_unavailable() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "test.com".to_string(),
                available: Some(false),
                error: None,
            }],
            npm_available: Some(false),
            pypi_available: Some(false),
            github_available: Some(false),
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Partial);
    }

    #[test]
    fn calculate_status_mixed_domains() {
        let validation = NameValidation {
            domains: vec![
                DomainCheck {
                    domain: "a.com".into(),
                    available: Some(true),
                    error: None,
                },
                DomainCheck {
                    domain: "a.io".into(),
                    available: Some(false),
                    error: None,
                },
            ],
            npm_available: Some(true),
            pypi_available: None,
            github_available: None,
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Partial);
    }

    #[test]
    fn calculate_status_only_domains_available() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "test.dev".into(),
                available: Some(true),
                error: None,
            }],
            npm_available: None,
            pypi_available: None,
            github_available: None,
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::AllClear);
    }

    #[test]
    fn calculate_status_domain_no_availability_info() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "test.com".into(),
                available: None,
                error: Some("timeout".into()),
            }],
            npm_available: None,
            pypi_available: None,
            github_available: None,
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Pending);
    }

    #[test]
    fn validation_status_default_is_pending() {
        assert_eq!(ValidationStatus::default(), ValidationStatus::Pending);
    }

    #[test]
    fn name_validation_default_fields() {
        let v = NameValidation::default();
        assert!(v.domains.is_empty());
        assert!(v.npm_available.is_none());
        assert!(v.pypi_available.is_none());
        assert!(v.github_available.is_none());
        assert!(v.negative_associations.is_empty());
        assert_eq!(v.overall_status, ValidationStatus::Pending);
    }

    #[test]
    fn name_gen_input_custom() {
        let input = NameGenInput {
            description: "A widget maker".into(),
            vibes: vec!["fast".into(), "modern".into()],
            count: 10,
        };
        assert_eq!(input.description, "A widget maker");
        assert_eq!(input.vibes.len(), 2);
        assert_eq!(input.count, 10);
    }

    #[test]
    fn name_gen_input_default_fields() {
        let input = NameGenInput::default();
        assert!(input.description.is_empty());
        assert!(input.vibes.is_empty());
        assert_eq!(input.count, 20);
    }

    #[test]
    fn name_candidate_serde_roundtrip() {
        let candidate = NameCandidate {
            name: "acme".to_string(),
            tagline: Some("Build anything".to_string()),
            reasoning: "Classic".to_string(),
            validation: NameValidation {
                domains: vec![DomainCheck {
                    domain: "acme.com".to_string(),
                    available: Some(false),
                    error: None,
                }],
                npm_available: Some(true),
                pypi_available: None,
                github_available: Some(false),
                negative_associations: vec!["sounds aggressive".into()],
                overall_status: ValidationStatus::Conflicted,
            },
        };
        let json = serde_json::to_string(&candidate).unwrap();
        let deser: NameCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "acme");
        assert_eq!(deser.tagline.as_deref(), Some("Build anything"));
        assert_eq!(deser.validation.domains.len(), 1);
        assert_eq!(deser.validation.npm_available, Some(true));
        assert_eq!(deser.validation.negative_associations.len(), 1);
        assert_eq!(
            deser.validation.overall_status,
            ValidationStatus::Conflicted
        );
    }

    #[test]
    fn domain_check_serde() {
        let check = DomainCheck {
            domain: "test.io".into(),
            available: Some(true),
            error: None,
        };
        let json = serde_json::to_string(&check).unwrap();
        let deser: DomainCheck = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.domain, "test.io");
        assert_eq!(deser.available, Some(true));
        assert!(deser.error.is_none());
    }

    #[test]
    fn validation_status_sort_priority_order() {
        assert!(
            ValidationStatus::AllClear.sort_priority() < ValidationStatus::Partial.sort_priority()
        );
        assert!(
            ValidationStatus::Partial.sort_priority() < ValidationStatus::Pending.sort_priority()
        );
        assert!(
            ValidationStatus::Pending.sort_priority()
                < ValidationStatus::Conflicted.sort_priority()
        );
    }

    #[test]
    fn name_gen_result_sorted_stable() {
        let make = |name: &str, status: ValidationStatus| NameCandidate {
            name: name.into(),
            tagline: None,
            reasoning: String::new(),
            validation: NameValidation {
                overall_status: status,
                ..Default::default()
            },
        };

        let result = NameGenResult {
            candidates: vec![
                make("a", ValidationStatus::AllClear),
                make("b", ValidationStatus::AllClear),
                make("c", ValidationStatus::Partial),
            ],
        };
        let sorted = result.sorted();
        // AllClear should come first, then Partial
        assert_eq!(
            sorted.candidates[0].validation.overall_status,
            ValidationStatus::AllClear
        );
        assert_eq!(
            sorted.candidates[1].validation.overall_status,
            ValidationStatus::AllClear
        );
        assert_eq!(
            sorted.candidates[2].validation.overall_status,
            ValidationStatus::Partial
        );
    }

    #[test]
    fn calculate_status_only_npm_unavailable() {
        let validation = NameValidation {
            domains: vec![],
            npm_available: Some(false),
            pypi_available: None,
            github_available: None,
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Partial);
    }

    #[test]
    fn calculate_status_only_pypi_unavailable() {
        let validation = NameValidation {
            domains: vec![],
            npm_available: None,
            pypi_available: Some(false),
            github_available: None,
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Partial);
    }

    #[test]
    fn calculate_status_only_github_unavailable() {
        let validation = NameValidation {
            domains: vec![],
            npm_available: None,
            pypi_available: None,
            github_available: Some(false),
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Partial);
    }

    #[test]
    fn calculate_status_registries_all_available_no_domains() {
        let validation = NameValidation {
            domains: vec![],
            npm_available: Some(true),
            pypi_available: Some(true),
            github_available: Some(true),
            negative_associations: Vec::new(),
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::AllClear);
    }

    #[test]
    fn calculate_status_negative_associations_override_everything() {
        let validation = NameValidation {
            domains: vec![DomainCheck {
                domain: "great.com".into(),
                available: Some(true),
                error: None,
            }],
            npm_available: Some(true),
            pypi_available: Some(true),
            github_available: Some(true),
            negative_associations: vec!["offensive term".into()],
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&validation), ValidationStatus::Conflicted);
    }

    #[test]
    fn name_gen_result_empty_candidates() {
        let result = NameGenResult { candidates: vec![] };
        let sorted = result.sorted();
        assert!(sorted.candidates.is_empty());
    }

    #[test]
    fn name_gen_input_clone() {
        let input = NameGenInput {
            description: "test".into(),
            vibes: vec!["cool".into()],
            count: 5,
        };
        let cloned = input.clone();
        assert_eq!(cloned.description, "test");
        assert_eq!(cloned.vibes.len(), 1);
        assert_eq!(cloned.count, 5);
    }

    #[test]
    fn name_candidate_clone() {
        let candidate = NameCandidate {
            name: "acme".into(),
            tagline: Some("build it".into()),
            reasoning: "good name".into(),
            validation: NameValidation::default(),
        };
        let cloned = candidate.clone();
        assert_eq!(cloned.name, "acme");
        assert_eq!(cloned.tagline, Some("build it".into()));
    }

    #[test]
    fn name_gen_result_clone() {
        let result = NameGenResult {
            candidates: vec![NameCandidate {
                name: "foo".into(),
                tagline: None,
                reasoning: "short".into(),
                validation: NameValidation::default(),
            }],
        };
        let cloned = result.clone();
        assert_eq!(cloned.candidates.len(), 1);
        assert_eq!(cloned.candidates[0].name, "foo");
    }

    #[test]
    fn validation_status_equality() {
        assert_eq!(ValidationStatus::AllClear, ValidationStatus::AllClear);
        assert_ne!(ValidationStatus::AllClear, ValidationStatus::Partial);
        assert_ne!(ValidationStatus::Partial, ValidationStatus::Conflicted);
        assert_ne!(ValidationStatus::Pending, ValidationStatus::AllClear);
    }

    struct MockLlmProvider;

    #[async_trait::async_trait]
    impl crate::llm::LlmProvider for MockLlmProvider {
        async fn chat(
            &self,
            request: &crate::llm::LlmRequest,
        ) -> anyhow::Result<crate::llm::LlmResponse> {
            use crate::llm::Role;

            let system_msg = request
                .messages
                .iter()
                .find(|m| m.role == Role::System)
                .map(|m| m.content.as_str())
                .unwrap_or("");

            let content = if system_msg.contains("naming expert") {
                // Brainstorm response
                r#"[
                    {"name": "testify", "tagline": "Test with confidence", "reasoning": "Testing + verify"},
                    {"name": "validox", "reasoning": "Validation + orthodox"}
                ]"#
                .to_string()
            } else if system_msg.contains("brand safety") {
                // Negative associations scan - no concerns
                "[]".to_string()
            } else {
                "[]".to_string()
            };

            Ok(crate::llm::LlmResponse {
                content,
                model: "mock".to_string(),
                usage: crate::llm::TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 10,
                    total_tokens: 20,
                },
            })
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_generate_names_with_mock_provider() {
        let provider = MockLlmProvider;
        let input = NameGenInput {
            description: "A testing framework".to_string(),
            vibes: vec!["modern".to_string()],
            count: 2,
        };

        let result = generate_names(&provider, &input).await.unwrap();
        assert_eq!(result.candidates.len(), 2);

        // Verify candidates have names from the mock response
        let names: Vec<&str> = result.candidates.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"testify"));
        assert!(names.contains(&"validox"));

        // Verify validation was attempted for each candidate
        for candidate in &result.candidates {
            // Status should be calculated (not the initial Pending default)
            // Actual status depends on network availability during the test:
            // - If network calls succeed: Partial (domains registered) or AllClear
            // - If network calls fail: Pending (no data from unwrap_or_default)
            assert!(matches!(
                candidate.validation.overall_status,
                ValidationStatus::AllClear | ValidationStatus::Partial | ValidationStatus::Pending
            ));
            // Negative associations should be empty (mock returns [])
            assert!(candidate.validation.negative_associations.is_empty());
        }

        // Verify tagline handling
        let testify = result
            .candidates
            .iter()
            .find(|c| c.name == "testify")
            .unwrap();
        assert_eq!(testify.tagline.as_deref(), Some("Test with confidence"));

        let validox = result
            .candidates
            .iter()
            .find(|c| c.name == "validox")
            .unwrap();
        assert!(validox.tagline.is_none());
    }
}
