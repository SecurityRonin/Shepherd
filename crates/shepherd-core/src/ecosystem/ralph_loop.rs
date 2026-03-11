use std::path::{Path, PathBuf};
use super::superpowers::InstallScope;

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    pub target_path: PathBuf,
    pub config_content: String,
}

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    if let Some(project) = project_root {
        if let Some(result) = detect_project_scope(agent, project) {
            return result;
        }
    }
    detect_user_scope(agent, home)
}

fn detect_user_scope(agent: &str, home: &Path) -> DetectionResult {
    match agent {
        "claude-code" => {
            // Check plugin cache directory
            let plugin_dir = home.join(".claude/plugins/cache/ralph-loop-setup/ralph-loop");
            if plugin_dir.exists() {
                return DetectionResult { installed: true, scope: InstallScope::User, path: Some(plugin_dir) };
            }
            // Also check settings.json for plugin reference
            let settings = home.join(".claude/settings.json");
            if settings.exists() {
                let content = std::fs::read_to_string(&settings).unwrap_or_default();
                if content.contains("ralph-loop") {
                    return DetectionResult { installed: true, scope: InstallScope::User, path: Some(settings) };
                }
            }
            DetectionResult { installed: false, scope: InstallScope::User, path: None }
        }
        _ => DetectionResult { installed: false, scope: InstallScope::User, path: None },
    }
}

fn detect_project_scope(agent: &str, project: &Path) -> Option<DetectionResult> {
    let config_path = match agent {
        "claude-code" => project.join(".claude/settings.json"),
        _ => return None,
    };
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if content.contains("ralph-loop") {
            return Some(DetectionResult {
                installed: true,
                scope: InstallScope::Project,
                path: Some(config_path),
            });
        }
    }
    None
}

pub fn is_ralph_loop_compatible(agent: &str) -> bool {
    matches!(agent, "claude-code")
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let (target_path, config_content) = match (agent, &scope) {
            ("claude-code", InstallScope::User) => (
                PathBuf::from("~/.claude/settings.json"),
                "# Ralph Loop — autonomous TDD workflow plugin\n# Installed by Shepherd. See https://github.com/MarioGiancini/ralph-loop-setup\n".to_string(),
            ),
            ("claude-code", InstallScope::Project) => (
                PathBuf::from(".claude/settings.json"),
                "# Ralph Loop — autonomous TDD workflow plugin\n# Installed by Shepherd. See https://github.com/MarioGiancini/ralph-loop-setup\n".to_string(),
            ),
            _ => return None,
        };
        Some(Self {
            agent: agent.to_string(),
            scope,
            target_path,
            config_content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_claude_code_plugin_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin_dir = tmp.path().join(".claude/plugins/cache/ralph-loop-setup/ralph-loop");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_claude_code_settings_ref() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"plugins":["ralph-loop"]}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::write(
            project.join(".claude/settings.json"),
            r#"{"plugins":["ralph-loop"]}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_targets_claude() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_ralph_loop_compatible("claude-code"));
        assert!(!is_ralph_loop_compatible("codex"));
        assert!(!is_ralph_loop_compatible("aider"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }
}
