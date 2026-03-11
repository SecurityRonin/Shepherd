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
        self.candidates.sort_by_key(|c| c.validation.overall_status.sort_priority());
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
    let mut all_available = true;
    let mut any_unavailable = false;
    let mut has_any_data = false;

    // Check domains
    for domain in &validation.domains {
        if let Some(available) = domain.available {
            has_any_data = true;
            if !available {
                any_unavailable = true;
                all_available = false;
            }
        }
    }

    // Check registries
    for available in [validation.npm_available, validation.pypi_available, validation.github_available]
        .into_iter()
        .flatten()
    {
        has_any_data = true;
        if !available {
            any_unavailable = true;
            all_available = false;
        }
    }

    if !has_any_data {
        return ValidationStatus::Pending;
    }

    if all_available {
        ValidationStatus::AllClear
    } else if any_unavailable {
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
        let (domains, npm_available, pypi_available, github_available): (Vec<DomainCheck>, Option<bool>, Option<bool>, Option<bool>) =
            validation_result.unwrap_or_default();

        // Step 4: Scan for negative associations
        let negative_associations =
            brainstorm::scan_negative_associations(provider, &name).await.unwrap_or_default();

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
            domains: vec![
                DomainCheck {
                    domain: "test.com".to_string(),
                    available: Some(true),
                    error: None,
                },
            ],
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
            domains: vec![
                DomainCheck {
                    domain: "test.com".to_string(),
                    available: Some(true),
                    error: None,
                },
            ],
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
            domains: vec![
                DomainCheck {
                    domain: "test.com".to_string(),
                    available: Some(false),
                    error: None,
                },
            ],
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
}
