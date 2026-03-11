pub mod protocol;

use anyhow::{Context, Result};
use protocol::AdapterConfig;
use std::collections::HashMap;
use std::path::Path;

pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterConfig>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "toml") {
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

    pub fn get(&self, id: &str) -> Option<&AdapterConfig> {
        self.adapters.get(id)
    }

    pub fn list(&self) -> Vec<(&str, &AdapterConfig)> {
        self.adapters.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
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
}
