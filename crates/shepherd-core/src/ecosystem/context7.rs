use std::path::{Path, PathBuf};
use super::superpowers::InstallScope;
use super::{EcosystemPlugin, PluginDetectionResult};

/// Return the shared plugin definition for context7.
pub fn plugin() -> EcosystemPlugin {
    EcosystemPlugin {
        name: "context7",
        description: "Context7 MCP server for up-to-date library documentation",
        compatible_agents: &["claude-code"],
        feature_key: "context7",
        plugin_cache_dirs: &[],
        user_settings_paths: &[("claude-code", ".claude/settings.json")],
        project_settings_paths: &[("claude-code", ".claude/settings.json")],
        install_targets: &[
            ("claude-code", true, "~/.claude/settings.json"),
            ("claude-code", false, ".claude/settings.json"),
        ],
        config_content: CONTEXT7_MCP_JSON,
    }
}

const CONTEXT7_MCP_ENTRY: &str = r#""context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp"],
      "env": {}
    }"#;

const CONTEXT7_MCP_JSON: &str = "{\n  \"mcpServers\": {\n    \"context7\": {\n      \"command\": \"npx\",\n      \"args\": [\"-y\", \"@upstash/context7-mcp\"],\n      \"env\": {}\n    }\n  }\n}";

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
    pub mcp_server_json: String,
}

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    plugin().detect(agent, home, project_root).into()
}

pub fn is_context7_compatible(agent: &str) -> bool {
    plugin().is_compatible(agent)
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let cfg = plugin().install_config(agent, scope)?;
        Some(Self {
            agent: cfg.agent,
            scope: cfg.scope,
            target_path: cfg.target_path,
            mcp_server_json: format!("{{\n  \"mcpServers\": {{\n    {CONTEXT7_MCP_ENTRY}\n  }}\n}}"),
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
    fn test_detect_claude_code_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"mcpServers":{"context7":{"command":"npx","args":["-y","@upstash/context7-mcp"]}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_claude_code_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::write(
            project.join(".claude/settings.json"),
            r#"{"mcpServers":{"context7":{"command":"npx"}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_generates_mcp_json() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.mcp_server_json.contains("context7"));
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_install_config_project_scope() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project).unwrap();
        assert!(config.mcp_server_json.contains("context7"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_context7_compatible("claude-code"));
        assert!(!is_context7_compatible("codex"));
        assert!(!is_context7_compatible("aider"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("codex", InstallScope::User);
        assert!(config.is_none());
    }
}
