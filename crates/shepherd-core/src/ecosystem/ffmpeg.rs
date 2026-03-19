use super::superpowers::InstallScope;
use super::{EcosystemPlugin, PluginDetectionResult};
use std::path::{Path, PathBuf};

/// Return the shared plugin definition for ffmpeg-mcp.
pub fn plugin() -> EcosystemPlugin {
    EcosystemPlugin {
        name: "ffmpeg",
        description: "FFmpeg MCP server for media processing, conversion, and analysis",
        compatible_agents: &["claude-code", "codex", "gemini-cli"],
        feature_key: "ffmpeg-mcp",
        plugin_cache_dirs: &[],
        user_settings_paths: &[
            ("claude-code", ".claude/settings.json"),
            ("codex", ".codex/config.json"),
            ("gemini-cli", ".gemini/settings.json"),
        ],
        project_settings_paths: &[
            ("claude-code", ".claude/settings.json"),
            ("codex", ".codex/config.json"),
            ("gemini-cli", ".gemini/settings.json"),
        ],
        install_targets: &[
            ("claude-code", true, "~/.claude/settings.json"),
            ("claude-code", false, ".claude/settings.json"),
            ("codex", true, "~/.codex/config.json"),
            ("codex", false, ".codex/config.json"),
            ("gemini-cli", true, "~/.gemini/settings.json"),
            ("gemini-cli", false, ".gemini/settings.json"),
        ],
        config_content: FFMPEG_MCP_JSON,
    }
}

const FFMPEG_MCP_ENTRY: &str = r#""ffmpeg-mcp": {
      "command": "npx",
      "args": ["-y", "ffmpeg-mcp"],
      "env": {}
    }"#;

const FFMPEG_MCP_JSON: &str = "{\n  \"mcpServers\": {\n    \"ffmpeg-mcp\": {\n      \"command\": \"npx\",\n      \"args\": [\"-y\", \"ffmpeg-mcp\"],\n      \"env\": {}\n    }\n  }\n}";

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

pub fn is_ffmpeg_compatible(agent: &str) -> bool {
    plugin().is_compatible(agent)
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let cfg = plugin().install_config(agent, scope)?;
        Some(Self {
            agent: cfg.agent,
            scope: cfg.scope,
            target_path: cfg.target_path,
            mcp_server_json: format!("{{\n  \"mcpServers\": {{\n    {FFMPEG_MCP_ENTRY}\n  }}\n}}"),
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
            r#"{"mcpServers":{"ffmpeg-mcp":{"command":"npx","args":["-y","ffmpeg-mcp"]}}}"#,
        )
        .unwrap();
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
            r#"{"mcpServers":{"ffmpeg-mcp":{"command":"npx"}}}"#,
        )
        .unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_generates_mcp_json() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.mcp_server_json.contains("ffmpeg-mcp"));
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_install_config_project_scope() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project).unwrap();
        assert!(config.mcp_server_json.contains("ffmpeg-mcp"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_ffmpeg_compatible("claude-code"));
        assert!(is_ffmpeg_compatible("codex"));
        assert!(is_ffmpeg_compatible("gemini-cli"));
        assert!(!is_ffmpeg_compatible("aider"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }

    #[test]
    fn test_detect_gemini_cli_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        std::fs::write(
            gemini_dir.join("settings.json"),
            r#"{"mcpServers":{"ffmpeg-mcp":{"command":"npx"}}}"#,
        )
        .unwrap();
        let result = detect_for_agent("gemini-cli", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }
}
