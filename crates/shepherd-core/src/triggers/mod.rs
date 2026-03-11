pub mod detectors;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A trigger suggestion to show the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSuggestion {
    pub id: String,
    pub tool: String,
    pub message: String,
    pub action_label: String,
    pub action_route: String,
    pub priority: TriggerPriority,
}

/// Priority determines ordering and visual treatment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPriority {
    Low,
    Medium,
    High,
}

/// Trait for implementing trigger detectors
pub trait TriggerDetector: Send + Sync {
    fn id(&self) -> &str;
    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>>;
}

/// Dismissed triggers stored in SQLite to avoid re-showing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissedTrigger {
    pub trigger_id: String,
    pub project_path: String,
    pub dismissed_at: String,
}

/// Run all detectors and return active suggestions
pub fn check_triggers(
    project_dir: &Path,
    dismissed: &[String],
) -> Vec<TriggerSuggestion> {
    let detectors: Vec<Box<dyn TriggerDetector>> = vec![
        Box::new(detectors::NameGenDetector),
        Box::new(detectors::LogoGenDetector),
        Box::new(detectors::NorthStarDetector),
    ];

    let mut suggestions = Vec::new();

    for detector in &detectors {
        if dismissed.contains(&detector.id().to_string()) {
            continue;
        }

        match detector.detect(project_dir) {
            Ok(Some(suggestion)) => suggestions.push(suggestion),
            Ok(None) => {}
            Err(e) => {
                tracing::debug!("Trigger detector '{}' failed: {e}", detector.id());
            }
        }
    }

    suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_priority_ordering() {
        assert!(TriggerPriority::High > TriggerPriority::Medium);
        assert!(TriggerPriority::Medium > TriggerPriority::Low);
    }

    #[test]
    fn test_check_triggers_respects_dismissed() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "untitled"}"#,
        ).unwrap();

        let suggestions = check_triggers(tmp.path(), &[]);
        let has_namegen = suggestions.iter().any(|s| s.tool == "name_generator");
        assert!(has_namegen);

        let suggestions = check_triggers(tmp.path(), &["namegen_untitled".to_string()]);
        let has_namegen = suggestions.iter().any(|s| s.id == "namegen_untitled");
        assert!(!has_namegen);
    }

    #[test]
    fn test_check_triggers_empty_project() {
        let tmp = tempfile::tempdir().unwrap();
        let suggestions = check_triggers(tmp.path(), &[]);
        let has_northstar = suggestions.iter().any(|s| s.tool == "north_star");
        assert!(has_northstar);
    }
}
