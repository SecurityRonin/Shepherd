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
pub fn check_triggers(project_dir: &Path, dismissed: &[String]) -> Vec<TriggerSuggestion> {
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
            // tarpaulin-start-ignore
            Err(e) => {
                tracing::debug!("Trigger detector '{}' failed: {e}", detector.id());
            } // tarpaulin-stop-ignore
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
        std::fs::write(tmp.path().join("package.json"), r#"{"name": "untitled"}"#).unwrap();

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

    #[test]
    fn test_check_triggers_all_dismissed() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), r#"{"name": "untitled"}"#).unwrap();

        let dismissed = vec![
            "namegen_untitled".to_string(),
            "logogen_no_icon".to_string(),
            "northstar_no_docs".to_string(),
        ];
        let suggestions = check_triggers(tmp.path(), &dismissed);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_check_triggers_sorted_by_priority() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a project that triggers both namegen (Medium) and northstar (Low)
        std::fs::write(tmp.path().join("package.json"), r#"{"name": "untitled"}"#).unwrap();

        let suggestions = check_triggers(tmp.path(), &[]);
        if suggestions.len() >= 2 {
            // Higher priority should come first
            assert!(suggestions[0].priority >= suggestions[1].priority);
        }
    }

    #[test]
    fn test_trigger_priority_serde() {
        let json = serde_json::to_string(&TriggerPriority::High).unwrap();
        assert_eq!(json, "\"high\"");
        let json = serde_json::to_string(&TriggerPriority::Medium).unwrap();
        assert_eq!(json, "\"medium\"");
        let json = serde_json::to_string(&TriggerPriority::Low).unwrap();
        assert_eq!(json, "\"low\"");

        let parsed: TriggerPriority = serde_json::from_str("\"high\"").unwrap();
        assert_eq!(parsed, TriggerPriority::High);
    }

    #[test]
    fn test_dismissed_trigger_serde() {
        let dismissed = DismissedTrigger {
            trigger_id: "namegen_untitled".to_string(),
            project_path: "/tmp/project".to_string(),
            dismissed_at: "2026-03-13T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&dismissed).unwrap();
        let parsed: DismissedTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.trigger_id, "namegen_untitled");
        assert_eq!(parsed.project_path, "/tmp/project");
    }

    #[test]
    fn test_check_triggers_with_nonexistent_dir() {
        // Using a nonexistent dir should not panic - detectors handle gracefully
        let suggestions = check_triggers(std::path::Path::new("/nonexistent/project"), &[]);
        // Some detectors may still fire (e.g. northstar checks for missing docs)
        // The important thing is no panic
        let _ = suggestions;
    }

    #[test]
    fn test_trigger_priority_clone() {
        let high = TriggerPriority::High;
        let cloned = high.clone();
        assert_eq!(cloned, TriggerPriority::High);
    }

    #[test]
    fn test_trigger_suggestion_serde() {
        let suggestion = TriggerSuggestion {
            id: "test".into(),
            tool: "test_tool".into(),
            message: "Test message".into(),
            action_label: "Test".into(),
            action_route: "/test".into(),
            priority: TriggerPriority::High,
        };
        let json = serde_json::to_string(&suggestion).unwrap();
        let parsed: TriggerSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.priority, TriggerPriority::High);
    }
}
