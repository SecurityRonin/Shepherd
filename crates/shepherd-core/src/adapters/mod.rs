pub mod protocol;

use anyhow::{Context, Result};
use protocol::AdapterConfig;
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterConfig>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toml") {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading adapter {}", path.display()))?;
                let config: AdapterConfig = toml::from_str(&content)
                    .with_context(|| format!("parsing adapter {}", path.display()))?;
                let key = path.file_stem().unwrap().to_string_lossy().to_string();
                tracing::info!("Loaded adapter: {} ({})", config.agent.name, key);
                self.adapters.insert(key, config);
            }
        }
        Ok(())
    }

    /// Register an adapter programmatically (useful for testing).
    pub fn register(&mut self, id: String, config: AdapterConfig) {
        self.adapters.insert(id, config);
    }

    pub fn get(&self, id: &str) -> Option<&AdapterConfig> {
        self.adapters.get(id)
    }

    pub fn list(&self) -> Vec<(&str, &AdapterConfig)> {
        self.adapters.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_adapter_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("test-agent.toml"),
            r#"
[agent]
name = "Test Agent"
command = "test-cli"

[status]
working_patterns = ["Working"]
idle_patterns = ["$"]
input_patterns = ["?"]
error_patterns = ["Error"]

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"
"#,
        )
        .unwrap();

        let mut registry = AdapterRegistry::new();
        registry.load_dir(dir.path()).unwrap();
        assert_eq!(registry.len(), 1);
        let adapter = registry.get("test-agent").unwrap();
        assert_eq!(adapter.agent.name, "Test Agent");
        assert_eq!(adapter.agent.command, "test-cli");
    }

    #[test]
    fn test_empty_registry() {
        let registry = AdapterRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.list().is_empty());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_load_dir_nonexistent() {
        let mut registry = AdapterRegistry::new();
        let result = registry.load_dir(std::path::Path::new("/nonexistent/dir"));
        assert!(result.is_ok()); // Returns Ok(()) for missing dirs
        assert!(registry.is_empty());
    }

    #[test]
    fn test_load_dir_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = AdapterRegistry::new();
        registry.load_dir(dir.path()).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_load_dir_skips_non_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.txt"), "not a toml").unwrap();
        fs::write(dir.path().join("config.json"), "{}").unwrap();
        let mut registry = AdapterRegistry::new();
        registry.load_dir(dir.path()).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_list_returns_all_adapters() {
        let dir = tempfile::tempdir().unwrap();
        let toml_content = r#"
[agent]
name = "Agent A"
command = "agent-a"

[status]
working_patterns = ["Working"]
idle_patterns = ["$"]
input_patterns = ["?"]
error_patterns = ["Error"]

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"
"#;
        fs::write(dir.path().join("agent-a.toml"), toml_content).unwrap();
        let mut registry = AdapterRegistry::new();
        registry.load_dir(dir.path()).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "agent-a");
        assert_eq!(list[0].1.agent.name, "Agent A");
    }

    #[test]
    fn test_load_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.toml"), "not valid toml [[[").unwrap();
        let mut registry = AdapterRegistry::new();
        let result = registry.load_dir(dir.path());
        assert!(result.is_err());
    }
}
