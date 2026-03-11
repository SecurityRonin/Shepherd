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
    pub mcp_server_json: String,
}

const CONTEXT_MODE_MCP_ENTRY: &str = r#""context-mode": {
      "command": "npx",
      "args": ["-y", "context-mode"],
      "env": {}
    }"#;

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    if let Some(project) = project_root {
        if let Some(result) = detect_project_scope(agent, project) {
            return result;
        }
    }
    detect_user_scope(agent, home)
}

fn detect_user_scope(agent: &str, home: &Path) -> DetectionResult {
    let settings_path = match agent {
        "claude-code" => home.join(".claude/settings.json"),
        _ => return DetectionResult { installed: false, scope: InstallScope::User, path: None },
    };
    check_settings_file(&settings_path, InstallScope::User)
}

fn detect_project_scope(agent: &str, project: &Path) -> Option<DetectionResult> {
    let settings_path = match agent {
        "claude-code" => project.join(".claude/settings.json"),
        _ => return None,
    };
    let result = check_settings_file(&settings_path, InstallScope::Project);
    if result.installed { Some(result) } else { None }
}

fn check_settings_file(path: &Path, scope: InstallScope) -> DetectionResult {
    if path.exists() {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        if content.contains("context-mode") {
            return DetectionResult {
                installed: true,
                scope,
                path: Some(path.to_path_buf()),
            };
        }
    }
    DetectionResult { installed: false, scope, path: None }
}

pub fn is_context_mode_compatible(agent: &str) -> bool {
    matches!(agent, "claude-code")
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let target_path = match (agent, &scope) {
            ("claude-code", InstallScope::User) => PathBuf::from("~/.claude/settings.json"),
            ("claude-code", InstallScope::Project) => PathBuf::from(".claude/settings.json"),
            _ => return None,
        };
        Some(Self {
            agent: agent.to_string(),
            scope,
            target_path,
            mcp_server_json: format!("{{\n  \"mcpServers\": {{\n    {CONTEXT_MODE_MCP_ENTRY}\n  }}\n}}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::superpowers::InstallScope;

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
            r#"{"mcpServers":{"context-mode":{"command":"npx","args":["-y","context-mode"]}}}"#,
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
            r#"{"mcpServers":{"context-mode":{"command":"npx"}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_generates_mcp_json() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.mcp_server_json.contains("context-mode"));
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_install_config_project_scope() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project).unwrap();
        assert!(config.mcp_server_json.contains("context-mode"));
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_context_mode_compatible("claude-code"));
        assert!(!is_context_mode_compatible("codex"));
        assert!(!is_context_mode_compatible("aider"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }
}
