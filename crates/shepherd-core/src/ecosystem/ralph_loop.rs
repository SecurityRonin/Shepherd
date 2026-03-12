use std::path::{Path, PathBuf};
use super::superpowers::InstallScope;
use super::{EcosystemPlugin, PluginDetectionResult};

/// Return the shared plugin definition for ralph-loop.
pub fn plugin() -> EcosystemPlugin {
    EcosystemPlugin {
        name: "ralph-loop",
        description: "Ralph Loop — autonomous TDD workflow plugin",
        compatible_agents: &["claude-code"],
        feature_key: "ralph-loop",
        plugin_cache_dirs: &[
            ("claude-code", ".claude/plugins/cache/ralph-loop-setup/ralph-loop"),
        ],
        user_settings_paths: &[("claude-code", ".claude/settings.json")],
        project_settings_paths: &[("claude-code", ".claude/settings.json")],
        install_targets: &[
            ("claude-code", true, "~/.claude/settings.json"),
            ("claude-code", false, ".claude/settings.json"),
        ],
        config_content: "# Ralph Loop — autonomous TDD workflow plugin\n# Installed by Shepherd. See https://github.com/MarioGiancini/ralph-loop-setup\n",
    }
}

// ── Backward-compatible API ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
}

impl From<PluginDetectionResult> for DetectionResult {
    fn from(r: PluginDetectionResult) -> Self {
        Self {
            installed: r.installed,
            scope: r.scope,
            path: r.path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    pub target_path: PathBuf,
    pub config_content: String,
}

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    plugin().detect(agent, home, project_root).into()
}

pub fn is_ralph_loop_compatible(agent: &str) -> bool {
    plugin().is_compatible(agent)
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let cfg = plugin().install_config(agent, scope)?;
        Some(Self {
            agent: cfg.agent,
            scope: cfg.scope,
            target_path: cfg.target_path,
            config_content: cfg.config_content,
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
