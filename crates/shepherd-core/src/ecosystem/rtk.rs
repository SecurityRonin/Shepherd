use super::superpowers::InstallScope;
use super::{EcosystemPlugin, PluginDetectionResult};
use std::path::{Path, PathBuf};

pub fn plugin() -> EcosystemPlugin {
    EcosystemPlugin {
        name: "rtk",
        description: "RTK CLI proxy for 60-90% token reduction on bash command output",
        compatible_agents: &["claude-code"],
        feature_key: "rtk",
        plugin_cache_dirs: &[],
        user_settings_paths: &[("claude-code", ".claude/settings.json")],
        project_settings_paths: &[],
        install_targets: &[("claude-code", true, "~/.claude/settings.json")],
        config_content: RTK_HOOKS_JSON,
    }
}

const RTK_HOOKS_JSON: &str = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": ["~/.claude/hooks/rtk-rewrite.sh"]
      }
    ]
  }
}"#;

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
    pub hooks_json: String,
    pub install_command: String,
}

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    plugin().detect(agent, home, project_root).into()
}

pub fn is_rtk_compatible(agent: &str) -> bool {
    plugin().is_compatible(agent)
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let cfg = plugin().install_config(agent, scope)?;
        let install_command = match cfg.scope {
            InstallScope::User => "rtk init -g",
            InstallScope::Project => "rtk init",
        };
        Some(Self {
            agent: cfg.agent,
            scope: cfg.scope,
            target_path: cfg.target_path,
            hooks_json: RTK_HOOKS_JSON.to_string(),
            install_command: install_command.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::superpowers::InstallScope;
    use super::*;

    #[test]
    fn plugin_name_is_rtk() {
        let p = plugin();
        assert_eq!(p.name, "rtk");
    }

    #[test]
    fn plugin_feature_key_is_rtk() {
        let p = plugin();
        assert_eq!(p.feature_key, "rtk");
    }

    #[test]
    fn plugin_description_mentions_token_reduction() {
        let p = plugin();
        assert!(p.description.contains("token"));
    }

    #[test]
    fn plugin_has_no_cache_dirs() {
        // RTK is hooks-based, not MCP — no cache directories
        let p = plugin();
        assert!(p.plugin_cache_dirs.is_empty());
    }

    #[test]
    fn plugin_has_no_project_settings() {
        // RTK only installs at user scope
        let p = plugin();
        assert!(p.project_settings_paths.is_empty());
    }

    #[test]
    fn compatible_with_claude_code() {
        assert!(is_rtk_compatible("claude-code"));
    }

    #[test]
    fn incompatible_with_other_agents() {
        assert!(!is_rtk_compatible("codex"));
        assert!(!is_rtk_compatible("aider"));
        assert!(!is_rtk_compatible(""));
    }

    #[test]
    fn detect_not_installed_in_empty_home() {
        let tmp = tempfile::tempdir().unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn detect_installed_when_settings_contains_rtk() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":["~/.claude/hooks/rtk-rewrite.sh"]}]}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn detect_not_installed_when_settings_has_no_rtk() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"mcpServers":{"context-mode":{"command":"npx"}}}"#,
        )
        .unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn install_config_user_scope_has_init_global() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert_eq!(config.install_command, "rtk init -g");
    }

    #[test]
    fn install_config_user_scope_has_hooks_json() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.hooks_json.contains("PreToolUse"));
        assert!(config.hooks_json.contains("rtk-rewrite"));
    }

    #[test]
    fn install_config_user_scope_targets_settings() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn install_config_project_scope_returns_none() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project);
        assert!(config.is_none());
    }

    #[test]
    fn install_config_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }
}
