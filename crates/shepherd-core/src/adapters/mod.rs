pub mod protocol;

use anyhow::{Context, Result};
use protocol::AdapterConfig;
use std::collections::HashMap;
use std::path::Path;

/// Default adapter configs, embedded at compile time.
const DEFAULT_ADAPTERS: &[(&str, &str)] = &[
    (
        "claude-code.toml",
        include_str!("defaults/claude-code.toml"),
    ),
    ("codex.toml", include_str!("defaults/codex.toml")),
    ("aider.toml", include_str!("defaults/aider.toml")),
    ("gemini-cli.toml", include_str!("defaults/gemini-cli.toml")),
    ("adal.toml", include_str!("defaults/adal.toml")),
    ("opencode.toml", include_str!("defaults/opencode.toml")),
    ("goose.toml", include_str!("defaults/goose.toml")),
    ("plandex.toml", include_str!("defaults/plandex.toml")),
    ("gptme.toml", include_str!("defaults/gptme.toml")),
];

/// Install default adapter configs to the given directory if they don't already exist.
/// Returns the number of configs installed.
pub fn install_defaults(adapters_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(adapters_dir)?;
    let mut installed = 0;
    for (filename, content) in DEFAULT_ADAPTERS {
        let path = adapters_dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, content)?;
            installed += 1;
        }
    }
    Ok(installed)
}

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

    #[test]
    fn test_install_defaults_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let count = install_defaults(dir.path()).unwrap();
        assert_eq!(count, 9);
        assert!(dir.path().join("claude-code.toml").exists());
        assert!(dir.path().join("codex.toml").exists());
        assert!(dir.path().join("aider.toml").exists());
        assert!(dir.path().join("gemini-cli.toml").exists());
        assert!(dir.path().join("adal.toml").exists());
        assert!(dir.path().join("opencode.toml").exists());
        assert!(dir.path().join("goose.toml").exists());
        assert!(dir.path().join("plandex.toml").exists());
        assert!(dir.path().join("gptme.toml").exists());
    }

    #[test]
    fn test_install_defaults_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("claude-code.toml"), "custom").unwrap();
        let count = install_defaults(dir.path()).unwrap();
        assert_eq!(count, 8); // 8 new, 1 skipped
        let content = fs::read_to_string(dir.path().join("claude-code.toml")).unwrap();
        assert_eq!(content, "custom"); // Not overwritten
    }

    #[test]
    fn test_install_defaults_creates_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("nested").join("adapters");
        let count = install_defaults(&sub).unwrap();
        assert_eq!(count, 9);
        assert!(sub.exists());
    }

    #[test]
    fn test_default_adapters_all_parse() {
        let dir = tempfile::tempdir().unwrap();
        install_defaults(dir.path()).unwrap();
        let mut registry = AdapterRegistry::new();
        registry.load_dir(dir.path()).unwrap();
        assert_eq!(registry.len(), 9);
        // Verify each one loaded with the correct ID (filename stem)
        assert!(registry.get("claude-code").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("aider").is_some());
        assert!(registry.get("gemini-cli").is_some());
        assert!(registry.get("adal").is_some());
        assert!(registry.get("opencode").is_some());
        assert!(registry.get("goose").is_some());
        assert!(registry.get("plandex").is_some());
        assert!(registry.get("gptme").is_some());
    }

    #[test]
    fn test_install_defaults_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let count1 = install_defaults(dir.path()).unwrap();
        assert_eq!(count1, 9);
        let count2 = install_defaults(dir.path()).unwrap();
        assert_eq!(count2, 0); // All already exist
    }
}
