use anyhow::Result;
use std::path::Path;

use super::{TriggerDetector, TriggerPriority, TriggerSuggestion};

/// Detects when the project has no meaningful product name
pub struct NameGenDetector;

impl TriggerDetector for NameGenDetector {
    fn id(&self) -> &str { "namegen_untitled" }

    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>> {
        let pkg_json = project_dir.join("package.json");
        if pkg_json.exists() {
            let content = std::fs::read_to_string(&pkg_json)?;
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
                    let is_untitled = matches!(
                        name.to_lowercase().as_str(),
                        "untitled" | "my-app" | "my-project" | "app" | "project"
                    );
                    if is_untitled {
                        return Ok(Some(TriggerSuggestion {
                            id: self.id().into(),
                            tool: "name_generator".into(),
                            message: "Want help brainstorming a product name?".into(),
                            action_label: "Open Name Generator".into(),
                            action_route: "/tools/namegen".into(),
                            priority: TriggerPriority::Medium,
                        }));
                    }
                }
            }
        }

        let cargo_toml = project_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if let Ok(parsed) = content.parse::<toml::Value>() {
                if let Some(name) = parsed.get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    let is_untitled = matches!(
                        name.to_lowercase().as_str(),
                        "untitled" | "my-app" | "my-project" | "app" | "project"
                    );
                    if is_untitled {
                        return Ok(Some(TriggerSuggestion {
                            id: self.id().into(),
                            tool: "name_generator".into(),
                            message: "Want help brainstorming a product name?".into(),
                            action_label: "Open Name Generator".into(),
                            action_route: "/tools/namegen".into(),
                            priority: TriggerPriority::Medium,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Detects when the project has no favicon or app icon
pub struct LogoGenDetector;

impl TriggerDetector for LogoGenDetector {
    fn id(&self) -> &str { "logogen_no_icon" }

    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>> {
        let icon_locations = [
            "public/favicon.ico",
            "public/favicon.svg",
            "assets/icon.png",
            "src-tauri/icons/icon.png",
            "static/favicon.ico",
            "app/favicon.ico",
        ];

        let has_icon = icon_locations.iter().any(|loc| project_dir.join(loc).exists());

        if !has_icon {
            let is_web_project = project_dir.join("package.json").exists()
                || project_dir.join("public").exists()
                || project_dir.join("index.html").exists();

            if is_web_project {
                return Ok(Some(TriggerSuggestion {
                    id: self.id().into(),
                    tool: "logo_generator".into(),
                    message: "No app icon found. Generate a logo?".into(),
                    action_label: "Open Logo Generator".into(),
                    action_route: "/tools/logogen".into(),
                    priority: TriggerPriority::Low,
                }));
            }
        }

        Ok(None)
    }
}

/// Detects when the project has no strategy/docs
pub struct NorthStarDetector;

impl TriggerDetector for NorthStarDetector {
    fn id(&self) -> &str { "northstar_no_docs" }

    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>> {
        let docs_dir = project_dir.join("docs");
        let ai_context = project_dir.join("ai-context.yml");
        let has_strategy = docs_dir.exists() && docs_dir.is_dir();
        let has_ai_context = ai_context.exists();

        if !has_strategy && !has_ai_context {
            return Ok(Some(TriggerSuggestion {
                id: self.id().into(),
                tool: "north_star".into(),
                message: "Define your product strategy?".into(),
                action_label: "Open North Star Wizard".into(),
                action_route: "/tools/northstar".into(),
                priority: TriggerPriority::Low,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namegen_detector_untitled_package() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "untitled", "version": "1.0.0"}"#,
        ).unwrap();

        let detector = NameGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, "name_generator");
    }

    #[test]
    fn test_namegen_detector_proper_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "shepherd", "version": "1.0.0"}"#,
        ).unwrap();

        let detector = NameGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_logogen_detector_no_icon() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let detector = LogoGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, "logo_generator");
    }

    #[test]
    fn test_logogen_detector_has_icon() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("public")).unwrap();
        std::fs::write(tmp.path().join("public/favicon.ico"), "icon").unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let detector = LogoGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_northstar_detector_no_docs() {
        let tmp = tempfile::tempdir().unwrap();

        let detector = NorthStarDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, "north_star");
    }

    #[test]
    fn test_northstar_detector_has_docs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("docs")).unwrap();

        let detector = NorthStarDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_northstar_detector_has_ai_context() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("ai-context.yml"), "product: test").unwrap();

        let detector = NorthStarDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }
}
