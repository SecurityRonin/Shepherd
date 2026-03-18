use std::path::{Path, PathBuf};
use super::superpowers::InstallScope;
use super::{EcosystemPlugin, PluginDetectionResult};

/// Return the shared plugin definition for Alaya.
/// Alaya is an MCP memory engine — compatible with any MCP-capable agent.
pub fn plugin() -> EcosystemPlugin {
    EcosystemPlugin {
        name: "alaya",
        description: "Alaya memory engine — episodic/semantic memory for AI agents (github.com/SecurityRonin/alaya)",
        compatible_agents: &["claude-code", "codex", "adal", "gemini-cli", "opencode"],
        feature_key: "alaya",
        plugin_cache_dirs: &[],
        user_settings_paths: &[
            ("claude-code", ".claude.json"),
        ],
        project_settings_paths: &[],
        install_targets: &[
            ("claude-code", true, "~/.claude.json"),
        ],
        config_content: ALAYA_MCP_JSON_NPX,
    }
}

/// Build the MCP JSON entry for Alaya.
/// Prefers the locally-built binary at ~/src/alaya/target/release/alaya-mcp;
/// falls back to `npx -y alaya-mcp` when that binary is absent.
pub fn mcp_entry_json() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_bin = PathBuf::from(&home).join("src/alaya/target/release/alaya-mcp");
    if local_bin.exists() {
        format!(
            r#"{{
  "mcpServers": {{
    "alaya": {{
      "type": "stdio",
      "command": "{}",
      "args": []
    }}
  }}
}}"#,
            local_bin.display()
        )
    } else {
        ALAYA_MCP_JSON_NPX.to_string()
    }
}

const ALAYA_MCP_JSON_NPX: &str = r#"{
  "mcpServers": {
    "alaya": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "alaya-mcp"]
    }
  }
}"#;

// ── Backward-compatible API ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
}

impl From<PluginDetectionResult> for DetectionResult {
    fn from(r: PluginDetectionResult) -> Self {
        Self { installed: r.installed, scope: r.scope, path: r.path }
    }
}

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    // For claude-code, detection means checking ~/.claude.json for the "alaya" key.
    if agent == "claude-code" {
        let config_path = home.join(".claude.json");
        if config_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&config_path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if v.get("mcpServers")
                        .and_then(|m| m.as_object())
                        .map(|o| o.contains_key("alaya"))
                        .unwrap_or(false)
                    {
                        return DetectionResult {
                            installed: true,
                            scope: InstallScope::User,
                            path: Some(config_path),
                        };
                    }
                }
            }
        }
        // Fall through to project scope check
        if let Some(project) = project_root {
            let mcp_json = project.join(".mcp.json");
            if mcp_json.exists() {
                if let Ok(raw) = std::fs::read_to_string(&mcp_json) {
                    if raw.contains("alaya") {
                        return DetectionResult {
                            installed: true,
                            scope: InstallScope::Project,
                            path: Some(mcp_json),
                        };
                    }
                }
            }
        }
    }
    DetectionResult { installed: false, scope: InstallScope::User, path: None }
}

/// Inject the Alaya MCP entry into `~/.claude.json` idempotently.
/// Returns `true` if the config was modified, `false` if already present.
pub fn ensure_installed(home: &Path) -> anyhow::Result<bool> {
    use anyhow::Context as _;
    let config_path = home.join(".claude.json");

    let mut config: serde_json::Value = if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("parsing {}", config_path.display()))?
    } else {
        serde_json::json!({})
    };

    let mcp_obj = config
        .as_object_mut()
        .context("~/.claude.json root is not an object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("mcpServers is not an object")?;

    if mcp_obj.contains_key("alaya") {
        return Ok(false);
    }

    // Build the entry, preferring the local binary.
    let local_bin = home.join("src/alaya/target/release/alaya-mcp");
    let entry = if local_bin.exists() {
        serde_json::json!({
            "type": "stdio",
            "command": local_bin.to_string_lossy(),
            "args": []
        })
    } else {
        serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "alaya-mcp"]
        })
    };

    mcp_obj.insert("alaya".to_string(), entry);

    let serialized = serde_json::to_string_pretty(&config)
        .context("serializing updated Claude config")?;
    let tmp = config_path.with_extension("json.alaya-tmp");
    std::fs::write(&tmp, &serialized)
        .with_context(|| format!("writing temp config to {}", tmp.display()))?;
    std::fs::rename(&tmp, &config_path)
        .with_context(|| format!("renaming temp config to {}", config_path.display()))?;

    tracing::info!("Added alaya MCP server to Claude Code config");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_home() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_plugin_name_and_description() {
        let p = plugin();
        assert_eq!(p.name, "alaya");
        assert!(p.description.contains("memory"));
    }

    #[test]
    fn test_plugin_compatible_agents() {
        let p = plugin();
        assert!(p.is_compatible("claude-code"));
        assert!(p.is_compatible("codex"));
        assert!(p.is_compatible("adal"));
        assert!(p.is_compatible("gemini-cli"));
        assert!(p.is_compatible("opencode"));
        assert!(!p.is_compatible("vim"));
    }

    #[test]
    fn test_detect_not_installed_empty_home() {
        let home = setup_home();
        let result = detect_for_agent("claude-code", home.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_installed_in_claude_json() {
        let home = setup_home();
        std::fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"alaya":{"type":"stdio","command":"/usr/local/bin/alaya-mcp"}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", home.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_not_installed_when_other_servers_present() {
        let home = setup_home();
        std::fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"other":{"type":"http","url":"https://example.com"}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", home.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_installed_in_project_mcp_json() {
        let home = setup_home();
        let project = setup_home();
        std::fs::write(
            project.path().join(".mcp.json"),
            r#"{"mcpServers":{"alaya":{"type":"stdio","command":"npx","args":["-y","alaya-mcp"]}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", home.path(), Some(project.path()));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_ensure_installed_adds_to_empty_config() {
        let home = setup_home();
        std::fs::write(home.path().join(".claude.json"), "{}").unwrap();
        let modified = ensure_installed(home.path()).unwrap();
        assert!(modified);
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        ).unwrap();
        assert!(content["mcpServers"]["alaya"].is_object());
    }

    #[test]
    fn test_ensure_installed_idempotent() {
        let home = setup_home();
        std::fs::write(home.path().join(".claude.json"), "{}").unwrap();
        ensure_installed(home.path()).unwrap();
        let modified = ensure_installed(home.path()).unwrap();
        assert!(!modified, "second call should not modify the file");
    }

    #[test]
    fn test_ensure_installed_preserves_existing_servers() {
        let home = setup_home();
        std::fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"gamma":{"type":"http","url":"https://mcp.gamma.app/mcp"}},"numStartups":3}"#,
        ).unwrap();
        ensure_installed(home.path()).unwrap();
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        ).unwrap();
        assert!(content["mcpServers"]["gamma"].is_object(), "existing server preserved");
        assert!(content["mcpServers"]["alaya"].is_object(), "alaya added");
        assert_eq!(content["numStartups"], 3, "other config keys preserved");
    }

    #[test]
    fn test_ensure_installed_does_not_overwrite_custom_alaya_entry() {
        let home = setup_home();
        let custom = r#"{"mcpServers":{"alaya":{"type":"stdio","command":"/my/custom/alaya-mcp"}}}"#;
        std::fs::write(home.path().join(".claude.json"), custom).unwrap();
        let modified = ensure_installed(home.path()).unwrap();
        assert!(!modified);
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        ).unwrap();
        assert_eq!(content["mcpServers"]["alaya"]["command"], "/my/custom/alaya-mcp");
    }

    #[test]
    fn test_mcp_entry_json_falls_back_to_npx_when_no_local_binary() {
        std::env::set_var("HOME", "/tmp/__no_such_home_alaya_test");
        let json = mcp_entry_json();
        assert!(json.contains("npx"));
        assert!(json.contains("alaya-mcp"));
    }

    #[test]
    fn test_ensure_installed_creates_mcp_servers_key_if_absent() {
        let home = setup_home();
        std::fs::write(home.path().join(".claude.json"), r#"{"numStartups":1}"#).unwrap();
        ensure_installed(home.path()).unwrap();
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        ).unwrap();
        assert!(content["mcpServers"]["alaya"].is_object());
        assert_eq!(content["numStartups"], 1);
    }

    #[test]
    fn test_detect_for_non_claude_code_agent_returns_not_installed() {
        let home = setup_home();
        let result = detect_for_agent("vim", home.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_for_claude_code_with_invalid_json() {
        let home = setup_home();
        std::fs::write(home.path().join(".claude.json"), "not valid json {{").unwrap();
        let result = detect_for_agent("claude-code", home.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_for_claude_code_with_no_mcp_servers_key() {
        let home = setup_home();
        std::fs::write(home.path().join(".claude.json"), r#"{"numStartups": 5}"#).unwrap();
        let result = detect_for_agent("claude-code", home.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_ensure_installed_creates_config_file_if_missing() {
        let home = setup_home();
        // No .claude.json exists
        assert!(!home.path().join(".claude.json").exists());
        // ensure_installed should handle this (creates from scratch)
        // But it expects to read, so it will produce an empty object
        // Actually looking at the code, if config_path doesn't exist it starts with {}
        let modified = ensure_installed(home.path()).unwrap();
        assert!(modified);
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        ).unwrap();
        assert!(content["mcpServers"]["alaya"].is_object());
    }

    #[test]
    fn test_detection_result_from_plugin_detection_result() {
        let pdr = PluginDetectionResult {
            installed: true,
            scope: InstallScope::Project,
            path: Some(PathBuf::from("/tmp/test")),
        };
        let dr: DetectionResult = pdr.into();
        assert!(dr.installed);
        assert_eq!(dr.scope, InstallScope::Project);
        assert_eq!(dr.path, Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_detection_result_debug_and_clone() {
        let dr = DetectionResult {
            installed: false,
            scope: InstallScope::User,
            path: None,
        };
        let cloned = dr.clone();
        assert_eq!(cloned.installed, false);
        let _ = format!("{:?}", dr);
    }

    #[test]
    fn test_plugin_feature_key() {
        let p = plugin();
        assert_eq!(p.feature_key, "alaya");
    }

    #[test]
    fn test_plugin_install_targets() {
        let p = plugin();
        assert!(!p.install_targets.is_empty());
        let (agent, is_default, path) = p.install_targets[0];
        assert_eq!(agent, "claude-code");
        assert!(is_default);
        assert!(path.contains(".claude.json"));
    }

    #[test]
    fn test_detect_project_scope_no_alaya_in_mcp_json() {
        let home = setup_home();
        let project = setup_home();
        std::fs::write(
            project.path().join(".mcp.json"),
            r#"{"mcpServers":{"other-plugin":{"type":"stdio","command":"other"}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", home.path(), Some(project.path()));
        assert!(!result.installed);
    }

    #[test]
    fn test_mcp_entry_json_contains_alaya() {
        let json = mcp_entry_json();
        assert!(json.contains("alaya"));
        assert!(json.contains("mcpServers"));
    }

    #[test]
    fn test_ensure_installed_uses_local_binary_when_present() {
        let home = setup_home();
        // Create the local binary path structure
        let bin_dir = home.path().join("src/alaya/target/release");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("alaya-mcp"), "fake binary").unwrap();

        // Create an empty config so ensure_installed has something to work with
        std::fs::write(home.path().join(".claude.json"), "{}").unwrap();

        let modified = ensure_installed(home.path()).unwrap();
        assert!(modified);

        // Read back and verify the local binary path was used
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        ).unwrap();
        let alaya = &content["mcpServers"]["alaya"];
        assert!(alaya.is_object());
        let command = alaya["command"].as_str().unwrap();
        assert!(
            command.contains("src/alaya/target/release/alaya-mcp"),
            "Expected local binary path in command, got: {command}"
        );
        // Should NOT be npx
        assert_ne!(command, "npx");
    }
}
