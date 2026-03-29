pub mod alaya;
pub mod context7;
pub mod context_hub;
pub mod context_mode;
pub mod docling;
pub mod exa;
pub mod ffmpeg;
pub mod frontend_design;
pub mod playwright;
pub mod ralph_loop;
pub mod rtk;
pub mod serena;
pub mod sourcegraph;
pub mod superpowers;
pub mod whisper;

use std::path::{Path, PathBuf};
use superpowers::InstallScope;

/// Detection result for an ecosystem plugin.
#[derive(Debug, Clone)]
pub struct PluginDetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
}

/// Install configuration for an ecosystem plugin.
#[derive(Debug, Clone)]
pub struct PluginInstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    pub target_path: PathBuf,
    pub config_content: String,
}

impl PluginInstallConfig {
    /// Resolve `~` in `target_path` to the given home directory.
    pub fn resolve_path(&self, home: &Path) -> PathBuf {
        let s = self.target_path.to_string_lossy();
        if s.starts_with("~/") {
            home.join(&s[2..])
        } else {
            self.target_path.clone()
        }
    }

    /// Write the plugin config into the target settings file.
    ///
    /// For JSON files (`.json`): deep-merges `config_content` into the
    /// existing file, preserving other keys.  Creates the file and parent
    /// directories if they don't exist.
    ///
    /// Returns the resolved path that was written to.
    pub fn apply_install(&self, home: &Path) -> Result<PathBuf, ApplyInstallError> {
        let resolved = self.resolve_path(home);

        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ApplyInstallError::Io(parent.to_path_buf(), e))?;
        }

        let is_json = resolved
            .extension()
            .map(|ext| ext == "json")
            .unwrap_or(false);

        if is_json {
            self.apply_json_merge(&resolved)?;
        } else {
            // For non-JSON files (CLAUDE.md, instructions.md, config.toml),
            // append if not already present.
            let existing = std::fs::read_to_string(&resolved).unwrap_or_default();
            if !existing.contains(&self.config_content) {
                let mut content = existing;
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&self.config_content);
                std::fs::write(&resolved, content)
                    .map_err(|e| ApplyInstallError::Io(resolved.clone(), e))?;
            }
        }

        Ok(resolved)
    }

    fn apply_json_merge(&self, path: &Path) -> Result<(), ApplyInstallError> {
        let existing_str = std::fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
        let mut existing: serde_json::Value = serde_json::from_str(&existing_str)
            .map_err(|e| ApplyInstallError::Json(path.to_path_buf(), e))?;

        let incoming: serde_json::Value = serde_json::from_str(&self.config_content)
            .map_err(|e| ApplyInstallError::Json(path.to_path_buf(), e))?;

        json_deep_merge(&mut existing, &incoming);

        let output = serde_json::to_string_pretty(&existing)
            .map_err(|e| ApplyInstallError::Json(path.to_path_buf(), e))?;
        std::fs::write(path, output.as_bytes())
            .map_err(|e| ApplyInstallError::Io(path.to_path_buf(), e))?;

        Ok(())
    }
}

/// Recursively merge `source` into `target`.  Objects are merged key-by-key;
/// all other types overwrite.
fn json_deep_merge(target: &mut serde_json::Value, source: &serde_json::Value) {
    match (target, source) {
        (serde_json::Value::Object(t), serde_json::Value::Object(s)) => {
            for (key, value) in s {
                json_deep_merge(
                    t.entry(key.clone()).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        }
        (target, source) => {
            *target = source.clone();
        }
    }
}

#[derive(Debug)]
pub enum ApplyInstallError {
    Io(PathBuf, std::io::Error),
    Json(PathBuf, serde_json::Error),
}

impl std::fmt::Display for ApplyInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, e) => write!(f, "IO error at {}: {}", path.display(), e),
            Self::Json(path, e) => write!(f, "JSON error at {}: {}", path.display(), e),
        }
    }
}

impl std::error::Error for ApplyInstallError {}

/// A shared, data-driven definition for an ecosystem plugin.
///
/// Each ecosystem module defines a `plugin()` function that returns one of
/// these, moving detection and installation logic into shared methods and
/// eliminating duplicated boilerplate.
#[derive(Debug, Clone)]
pub struct EcosystemPlugin {
    /// Human-readable name (e.g. "context-mode").
    pub name: &'static str,
    /// Short description of what the plugin provides.
    pub description: &'static str,
    /// Agents this plugin is compatible with.
    pub compatible_agents: &'static [&'static str],
    /// Feature string to search for in settings files.
    pub feature_key: &'static str,
    /// Optional plugin cache directories to check (relative to home).
    /// Each entry is `(agent, relative_path)`.
    pub plugin_cache_dirs: &'static [(&'static str, &'static str)],
    /// Settings file paths to check per agent (relative to home for user scope).
    /// Each entry is `(agent, relative_path)`.
    pub user_settings_paths: &'static [(&'static str, &'static str)],
    /// Settings file paths to check per agent (relative to project root for project scope).
    /// Each entry is `(agent, relative_path)`.
    pub project_settings_paths: &'static [(&'static str, &'static str)],
    /// Install target path per agent+scope: `(agent, scope_is_user, relative_path)`.
    pub install_targets: &'static [(&'static str, bool, &'static str)],
    /// Config content to write on install.
    pub config_content: &'static str,
}

impl EcosystemPlugin {
    /// Check whether this plugin is compatible with the given agent.
    pub fn is_compatible(&self, agent: &str) -> bool {
        self.compatible_agents.contains(&agent)
    }

    /// Detect whether this plugin is installed for the given agent.
    pub fn detect(
        &self,
        agent: &str,
        home: &Path,
        project_root: Option<&Path>,
    ) -> PluginDetectionResult {
        if !self.is_compatible(agent) {
            return PluginDetectionResult {
                installed: false,
                scope: InstallScope::User,
                path: None,
            };
        }

        // Check project scope first
        if let Some(project) = project_root {
            if let Some(result) = self.detect_project_scope(agent, project) {
                return result;
            }
        }

        self.detect_user_scope(agent, home)
    }

    fn detect_user_scope(&self, agent: &str, home: &Path) -> PluginDetectionResult {
        // Check plugin cache directories first
        for &(a, rel_path) in self.plugin_cache_dirs {
            if a == agent {
                let path = home.join(rel_path);
                if path.exists() {
                    return PluginDetectionResult {
                        installed: true,
                        scope: InstallScope::User,
                        path: Some(path),
                    };
                }
            }
        }

        // Check settings files
        for &(a, rel_path) in self.user_settings_paths {
            if a == agent {
                let settings_path = home.join(rel_path);
                if settings_path.exists() {
                    let content = std::fs::read_to_string(&settings_path).unwrap_or_default();
                    if content.contains(self.feature_key) {
                        return PluginDetectionResult {
                            installed: true,
                            scope: InstallScope::User,
                            path: Some(settings_path),
                        };
                    }
                }
            }
        }

        PluginDetectionResult {
            installed: false,
            scope: InstallScope::User,
            path: None,
        }
    }

    fn detect_project_scope(&self, agent: &str, project: &Path) -> Option<PluginDetectionResult> {
        for &(a, rel_path) in self.project_settings_paths {
            if a == agent {
                let config_path = project.join(rel_path);
                if config_path.exists() {
                    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
                    if content.contains(self.feature_key) {
                        return Some(PluginDetectionResult {
                            installed: true,
                            scope: InstallScope::Project,
                            path: Some(config_path),
                        });
                    }
                }
            }
        }
        None
    }

    /// Build an install configuration for the given agent and scope.
    pub fn install_config(&self, agent: &str, scope: InstallScope) -> Option<PluginInstallConfig> {
        if !self.is_compatible(agent) {
            return None;
        }

        let is_user = scope == InstallScope::User;
        let target_path = self
            .install_targets
            .iter()
            .find(|&&(a, user, _)| a == agent && user == is_user)
            .map(|&(_, _, path)| PathBuf::from(path))?;

        Some(PluginInstallConfig {
            agent: agent.to_string(),
            scope,
            target_path,
            config_content: self.config_content.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_plugin() -> EcosystemPlugin {
        EcosystemPlugin {
            name: "test-plugin",
            description: "A test plugin",
            compatible_agents: &["claude-code", "aider"],
            feature_key: "test-plugin-key",
            plugin_cache_dirs: &[("claude-code", ".cache/test-plugin")],
            user_settings_paths: &[("claude-code", ".config/claude/settings.json")],
            project_settings_paths: &[("claude-code", ".claude/settings.json")],
            install_targets: &[
                ("claude-code", true, ".config/claude/test-plugin.json"),
                ("claude-code", false, ".claude/test-plugin.json"),
            ],
            config_content: r#"{"enabled": true}"#,
        }
    }

    #[test]
    fn test_is_compatible_true() {
        let plugin = test_plugin();
        assert!(plugin.is_compatible("claude-code"));
        assert!(plugin.is_compatible("aider"));
    }

    #[test]
    fn test_is_compatible_false() {
        let plugin = test_plugin();
        assert!(!plugin.is_compatible("vim"));
        assert!(!plugin.is_compatible(""));
    }

    #[test]
    fn test_detect_incompatible_agent() {
        let plugin = test_plugin();
        let home = std::path::Path::new("/tmp/fake-home");
        let result = plugin.detect("vim", home, None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = test_plugin();
        let result = plugin.detect("claude-code", tmp.path(), None);
        assert!(!result.installed);
        assert_eq!(result.scope, InstallScope::User);
        assert!(result.path.is_none());
    }

    #[test]
    fn test_detect_user_scope_via_cache_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join(".cache/test-plugin");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let plugin = test_plugin();
        let result = plugin.detect("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
        assert!(result.path.is_some());
    }

    #[test]
    fn test_detect_user_scope_via_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join(".config/claude/settings.json");
        std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        std::fs::write(&settings_path, r#"{"mcpServers": {"test-plugin-key": {}}}"#).unwrap();
        let plugin = test_plugin();
        let result = plugin.detect("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let settings_path = project.join(".claude/settings.json");
        std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        std::fs::write(&settings_path, r#"{"mcpServers": {"test-plugin-key": {}}}"#).unwrap();
        let plugin = test_plugin();
        let result = plugin.detect("claude-code", tmp.path(), Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_incompatible() {
        let plugin = test_plugin();
        assert!(plugin.install_config("vim", InstallScope::User).is_none());
    }

    #[test]
    fn test_install_config_user_scope() {
        let plugin = test_plugin();
        let config = plugin
            .install_config("claude-code", InstallScope::User)
            .unwrap();
        assert_eq!(config.agent, "claude-code");
        assert_eq!(config.scope, InstallScope::User);
        assert_eq!(config.config_content, r#"{"enabled": true}"#);
    }

    #[test]
    fn test_install_config_project_scope() {
        let plugin = test_plugin();
        let config = plugin
            .install_config("claude-code", InstallScope::Project)
            .unwrap();
        assert_eq!(config.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_no_matching_target() {
        let plugin = test_plugin();
        // aider is compatible but has no install target
        assert!(plugin.install_config("aider", InstallScope::User).is_none());
    }

    #[test]
    fn test_plugin_detection_result_debug() {
        let result = PluginDetectionResult {
            installed: true,
            scope: InstallScope::User,
            path: Some(std::path::PathBuf::from("/test")),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("installed: true"));
    }

    #[test]
    fn test_plugin_install_config_debug() {
        let config = PluginInstallConfig {
            agent: "claude-code".to_string(),
            scope: InstallScope::User,
            target_path: std::path::PathBuf::from("/test"),
            config_content: "{}".to_string(),
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("claude-code"));
    }

    #[test]
    fn test_detect_returns_none_when_no_project_match() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let plugin = test_plugin();
        // detect with project that has no settings file containing feature_key
        let result = plugin.detect("claude-code", tmp.path(), Some(&project));
        // Should not find project-scope match, falls through to None
        assert!(!result.installed || result.scope != InstallScope::Project);
    }

    // ── resolve_path tests ──────────────────────────────────────────

    #[test]
    fn test_resolve_path_tilde() {
        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/settings.json"),
            config_content: "{}".into(),
        };
        let resolved = config.resolve_path(Path::new("/home/user"));
        assert_eq!(resolved, PathBuf::from("/home/user/.claude/settings.json"));
    }

    #[test]
    fn test_resolve_path_relative() {
        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::Project,
            target_path: PathBuf::from(".claude/settings.json"),
            config_content: "{}".into(),
        };
        let resolved = config.resolve_path(Path::new("/home/user"));
        assert_eq!(resolved, PathBuf::from(".claude/settings.json"));
    }

    // ── apply_install JSON merge tests ──────────────────────────────

    #[test]
    fn test_apply_install_creates_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/settings.json"),
            config_content: r#"{"mcpServers":{"test":{"command":"npx"}}}"#.into(),
        };
        let path = config.apply_install(tmp.path()).unwrap();
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["test"]["command"].as_str() == Some("npx"));
    }

    #[test]
    fn test_apply_install_merges_into_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"mcpServers":{"existing":{"command":"node"}},"other":"keep"}"#,
        )
        .unwrap();

        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/settings.json"),
            config_content: r#"{"mcpServers":{"new-plugin":{"command":"npx"}}}"#.into(),
        };
        let path = config.apply_install(tmp.path()).unwrap();
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // Existing entries preserved
        assert_eq!(content["mcpServers"]["existing"]["command"], "node");
        // New entry added
        assert_eq!(content["mcpServers"]["new-plugin"]["command"], "npx");
        // Other keys preserved
        assert_eq!(content["other"], "keep");
    }

    #[test]
    fn test_apply_install_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/settings.json"),
            config_content: r#"{"mcpServers":{"test":{"command":"npx"}}}"#.into(),
        };
        config.apply_install(tmp.path()).unwrap();
        config.apply_install(tmp.path()).unwrap();
        let path = tmp.path().join(".claude/settings.json");
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // Only one entry, not duplicated
        assert_eq!(content["mcpServers"].as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_apply_install_invalid_existing_json() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.json"), "not json!!!").unwrap();

        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/settings.json"),
            config_content: r#"{"mcpServers":{}}"#.into(),
        };
        let result = config.apply_install(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON error"));
    }

    // ── apply_install non-JSON (append) tests ───────────────────────

    #[test]
    fn test_apply_install_appends_to_md() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "# Existing\n").unwrap();

        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/CLAUDE.md"),
            config_content: "# Superpowers\n".into(),
        };
        config.apply_install(tmp.path()).unwrap();
        let content = std::fs::read_to_string(claude_dir.join("CLAUDE.md")).unwrap();
        assert!(content.contains("# Existing"));
        assert!(content.contains("# Superpowers"));
    }

    #[test]
    fn test_apply_install_md_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let config = PluginInstallConfig {
            agent: "claude-code".into(),
            scope: InstallScope::User,
            target_path: PathBuf::from("~/.claude/CLAUDE.md"),
            config_content: "# Superpowers\n".into(),
        };
        config.apply_install(tmp.path()).unwrap();
        config.apply_install(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".claude/CLAUDE.md")).unwrap();
        assert_eq!(content.matches("# Superpowers").count(), 1);
    }

    // ── json_deep_merge tests ───────────────────────────────────────

    #[test]
    fn test_json_deep_merge_objects() {
        let mut target: serde_json::Value = serde_json::json!({"a": 1, "nested": {"x": 10}});
        let source = serde_json::json!({"b": 2, "nested": {"y": 20}});
        json_deep_merge(&mut target, &source);
        assert_eq!(target["a"], 1);
        assert_eq!(target["b"], 2);
        assert_eq!(target["nested"]["x"], 10);
        assert_eq!(target["nested"]["y"], 20);
    }

    #[test]
    fn test_json_deep_merge_overwrite_scalar() {
        let mut target: serde_json::Value = serde_json::json!({"a": 1});
        let source = serde_json::json!({"a": 99});
        json_deep_merge(&mut target, &source);
        assert_eq!(target["a"], 99);
    }

    // ── ApplyInstallError display ───────────────────────────────────

    #[test]
    fn test_apply_install_error_display() {
        let err = ApplyInstallError::Io(
            PathBuf::from("/test"),
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        );
        let msg = err.to_string();
        assert!(msg.contains("IO error"));
        assert!(msg.contains("/test"));
    }
}
