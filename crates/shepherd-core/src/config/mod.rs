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
    let mut config = if global_path.exists() {
        let content = std::fs::read_to_string(&global_path)?;
        toml::from_str(&content)?
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
}
