use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum InstallScope {
    User,
    Project,
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
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
    let path = match agent {
        "claude-code" => home.join(".claude/plugins/cache/claude-plugins-official/superpowers"),
        "codex" => home.join(".codex/superpowers"),
        "opencode" => home.join(".opencode/superpowers"),
        _ => {
            return DetectionResult {
                installed: false,
                scope: InstallScope::User,
                path: None,
                version: None,
            }
        }
    };
    if path.exists() {
        let version = detect_version(&path);
        DetectionResult {
            installed: true,
            scope: InstallScope::User,
            path: Some(path),
            version,
        }
    } else {
        DetectionResult {
            installed: false,
            scope: InstallScope::User,
            path: None,
            version: None,
        }
    }
}

fn detect_project_scope(agent: &str, project: &Path) -> Option<DetectionResult> {
    let config_path = match agent {
        "claude-code" => project.join(".claude/settings.json"),
        "codex" => project.join(".codex/instructions.md"),
        "opencode" => project.join(".opencode/config.toml"),
        // tarpaulin-start-ignore
        _ => return None,
        // tarpaulin-stop-ignore
    };
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if content.contains("superpowers") {
            return Some(DetectionResult {
                installed: true,
                scope: InstallScope::Project,
                path: Some(config_path),
                version: None,
            });
        }
    }
    None
}

fn detect_version(path: &Path) -> Option<String> {
    std::fs::read_dir(path)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                Some(name)
            } else {
                None
            }
        })
        .max()
}

pub fn is_superpowers_compatible(agent: &str) -> bool {
    matches!(agent, "claude-code" | "codex" | "opencode")
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let (target_path, config_content) = match (agent, &scope) {
            ("claude-code", InstallScope::User) => (
                PathBuf::from("~/.claude/CLAUDE.md"),
                "# Obra Superpowers\n# Installed by Shepherd. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("claude-code", InstallScope::Project) => (
                PathBuf::from(".claude/CLAUDE.md"),
                "# Obra Superpowers\n# Installed by Shepherd. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("codex", InstallScope::User) => (
                PathBuf::from("~/.codex/instructions.md"),
                "# Obra Superpowers skills available. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("codex", InstallScope::Project) => (
                PathBuf::from(".codex/instructions.md"),
                "# Obra Superpowers skills available. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("opencode", InstallScope::User) => (
                PathBuf::from("~/.opencode/config.toml"),
                "# superpowers = true\n# See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("opencode", InstallScope::Project) => (
                PathBuf::from(".opencode/config.toml"),
                "# superpowers = true\n# See https://github.com/obra/superpowers\n".to_string(),
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
    fn test_detect_claude_code_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp
            .path()
            .join(".claude/plugins/cache/claude-plugins-official/superpowers");
        std::fs::create_dir_all(&skills_dir).unwrap();
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
            r#"{"plugins":["superpowers"]}"#,
        )
        .unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_targets_agent_dir() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_install_config_project_scope() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project).unwrap();
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_superpowers_compatible("claude-code"));
        assert!(is_superpowers_compatible("codex"));
        assert!(is_superpowers_compatible("opencode"));
        assert!(!is_superpowers_compatible("aider"));
        assert!(!is_superpowers_compatible("gemini-cli"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }

    #[test]
    fn test_detect_codex_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join(".codex/superpowers");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let result = detect_for_agent("codex", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
        assert!(result.path.is_some());
    }

    #[test]
    fn test_detect_opencode_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join(".opencode/superpowers");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let result = detect_for_agent("opencode", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
        assert!(result.path.is_some());
    }

    #[test]
    fn test_detect_unknown_agent_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let result = detect_for_agent("unknown-agent", tmp.path(), None);
        assert!(!result.installed);
        assert_eq!(result.scope, InstallScope::User);
        assert!(result.path.is_none());
        assert!(result.version.is_none());
    }

    #[test]
    fn test_detect_codex_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".codex")).unwrap();
        std::fs::write(
            project.join(".codex/instructions.md"),
            "Enable superpowers for this project",
        )
        .unwrap();
        let result = detect_for_agent("codex", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_detect_opencode_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".opencode")).unwrap();
        std::fs::write(project.join(".opencode/config.toml"), "superpowers = true").unwrap();
        let result = detect_for_agent("opencode", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_detect_project_scope_config_exists_no_superpowers() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::write(
            project.join(".claude/settings.json"),
            r#"{"plugins":["other-plugin"]}"#,
        )
        .unwrap();
        // Config file exists but doesn't mention superpowers, so falls through
        // to user scope detection (which also won't find it)
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_version_with_versions() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("superpowers");
        std::fs::create_dir_all(base.join("1.0.0")).unwrap();
        std::fs::create_dir_all(base.join("2.1.0")).unwrap();
        std::fs::create_dir_all(base.join("0.5.0")).unwrap();
        // Also create a non-version dir
        std::fs::create_dir_all(base.join("cache")).unwrap();

        let version = detect_version(&base);
        assert!(version.is_some());
        // max() on string comparison: "2.1.0" > "1.0.0" > "0.5.0"
        assert_eq!(version.unwrap(), "2.1.0");
    }

    #[test]
    fn test_detect_version_no_versions() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("superpowers");
        std::fs::create_dir_all(base.join("cache")).unwrap();
        std::fs::create_dir_all(base.join("temp")).unwrap();

        let version = detect_version(&base);
        assert!(version.is_none());
    }

    #[test]
    fn test_detect_version_nonexistent_dir() {
        let version = detect_version(Path::new("/nonexistent/path"));
        assert!(version.is_none());
    }

    #[test]
    fn test_detect_user_scope_with_version() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp
            .path()
            .join(".claude/plugins/cache/claude-plugins-official/superpowers");
        std::fs::create_dir_all(skills_dir.join("1.2.3")).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_install_config_codex_user() {
        let config = InstallConfig::for_agent("codex", InstallScope::User).unwrap();
        assert_eq!(config.agent, "codex");
        assert!(config.target_path.to_string_lossy().contains(".codex"));
        assert!(
            config.config_content.contains("superpowers")
                || config.config_content.contains("Superpowers")
        );
    }

    #[test]
    fn test_install_config_codex_project() {
        let config = InstallConfig::for_agent("codex", InstallScope::Project).unwrap();
        assert_eq!(config.agent, "codex");
        assert!(config.target_path.to_string_lossy().contains(".codex"));
    }

    #[test]
    fn test_install_config_opencode_user() {
        let config = InstallConfig::for_agent("opencode", InstallScope::User).unwrap();
        assert_eq!(config.agent, "opencode");
        assert!(config.target_path.to_string_lossy().contains(".opencode"));
        assert!(config.config_content.contains("superpowers"));
    }

    #[test]
    fn test_install_config_opencode_project() {
        let config = InstallConfig::for_agent("opencode", InstallScope::Project).unwrap();
        assert_eq!(config.agent, "opencode");
        assert!(config.target_path.to_string_lossy().contains(".opencode"));
    }

    #[test]
    fn test_project_scope_takes_precedence_over_user() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        // Set up project-level detection
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::write(
            project.join(".claude/settings.json"),
            r#"{"superpowers": true}"#,
        )
        .unwrap();
        // Also set up user-level detection
        let home = tmp.path().join("home");
        let user_dir = home.join(".claude/plugins/cache/claude-plugins-official/superpowers");
        std::fs::create_dir_all(&user_dir).unwrap();

        let result = detect_for_agent("claude-code", &home, Some(&project));
        // Project scope should be returned first
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }
}
