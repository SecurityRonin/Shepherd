pub mod types;

use anyhow::Result;
use std::path::{Path, PathBuf};
use types::ShepherdConfig;

pub fn shepherd_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shepherd")
}

pub fn load_config(project_dir: Option<&Path>) -> Result<ShepherdConfig> {
    let global_path = shepherd_dir().join("config.toml");
    // tarpaulin-start-ignore
    let mut config = if global_path.exists() {
        let content = std::fs::read_to_string(&global_path)?;
        toml::from_str(&content)?
    // tarpaulin-stop-ignore
    } else {
        ShepherdConfig::default()
    };

    // Project-level overrides
    if let Some(dir) = project_dir {
        let project_path = dir.join(".shepherd").join("config.toml");
        if project_path.exists() {
            let content = std::fs::read_to_string(&project_path)?;
            let project: ShepherdConfig = toml::from_str(&content)?;
            // Project overrides take precedence
            config.default_agent = project.default_agent;
            config.default_isolation = project.default_isolation;
            config.default_permission_mode = project.default_permission_mode;
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ShepherdConfig::default();
        assert_eq!(config.port, 7532);
        assert_eq!(config.max_agents, 10);
        assert_eq!(config.default_permission_mode, "ask");
    }

    #[test]
    fn test_load_missing_config_returns_defaults() {
        let config = load_config(None).unwrap();
        assert_eq!(config.port, 7532);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = ShepherdConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ShepherdConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.port, config.port);
        assert_eq!(parsed.max_agents, config.max_agents);
    }

    #[test]
    fn test_load_config_with_project_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        let shepherd_dir = project_dir.join(".shepherd");
        std::fs::create_dir_all(&shepherd_dir).unwrap();
        std::fs::write(
            shepherd_dir.join("config.toml"),
            r#"
default_agent = "codex"
default_isolation = "container"
default_permission_mode = "auto"
"#,
        )
        .unwrap();
        let config = load_config(Some(&project_dir)).unwrap();
        // Project overrides should take effect
        assert_eq!(config.default_agent, "codex");
        assert_eq!(config.default_isolation, "container");
        assert_eq!(config.default_permission_mode, "auto");
        // Non-overridden defaults should remain
        assert_eq!(config.port, 7532);
        assert_eq!(config.max_agents, 10);
    }

    #[test]
    fn test_load_config_project_dir_without_config() {
        let tmp = tempfile::tempdir().unwrap();
        // Project dir exists but has no .shepherd/config.toml
        let config = load_config(Some(tmp.path())).unwrap();
        // Should just return defaults
        assert_eq!(config.default_agent, "claude-code");
        assert_eq!(config.default_isolation, "worktree");
        assert_eq!(config.default_permission_mode, "ask");
    }

    #[test]
    fn test_shepherd_dir_returns_path() {
        let dir = shepherd_dir();
        assert!(dir.to_string_lossy().contains(".shepherd"));
    }

    #[test]
    fn test_shepherd_dir_is_absolute() {
        let dir = shepherd_dir();
        assert!(dir.is_absolute() || dir.to_string_lossy().contains(".shepherd"));
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        let shepherd_dir = project_dir.join(".shepherd");
        std::fs::create_dir_all(&shepherd_dir).unwrap();
        std::fs::write(shepherd_dir.join("config.toml"), "invalid [[[toml").unwrap();
        let result = load_config(Some(&project_dir));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_partial_project_override() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        let shepherd_dir = project_dir.join(".shepherd");
        std::fs::create_dir_all(&shepherd_dir).unwrap();
        // Only override one field
        std::fs::write(
            shepherd_dir.join("config.toml"),
            r#"default_agent = "opencode""#,
        )
        .unwrap();
        let config = load_config(Some(&project_dir)).unwrap();
        assert_eq!(config.default_agent, "opencode");
        // Other fields get project defaults (which are the same as global defaults)
        assert_eq!(config.default_isolation, "worktree");
        assert_eq!(config.default_permission_mode, "ask");
    }
}
