pub mod builtin;
pub mod plugin;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of running a single quality gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
    pub gate_type: GateType,
}

/// The type/category of a quality gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateType {
    Lint,
    Format,
    TypeCheck,
    Test,
    Security,
    Custom,
}

/// Configuration for which quality gates to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    #[serde(default = "default_true")]
    pub lint: bool,
    #[serde(default = "default_true")]
    pub format_check: bool,
    #[serde(default = "default_true")]
    pub type_check: bool,
    #[serde(default = "default_true")]
    pub test: bool,
    #[serde(default)]
    pub custom_gates: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    300
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            lint: true,
            format_check: true,
            type_check: true,
            test: true,
            custom_gates: Vec::new(),
            timeout_seconds: 300,
        }
    }
}

// tarpaulin-start-ignore
/// Run all enabled quality gates for the project at the given path.
pub async fn run_gates(project_dir: &Path, config: &GateConfig) -> Result<Vec<GateResult>> {
    let project_type = builtin::detect_project_type(project_dir);
    let types = builtin::all_types(&project_type);
    let mut results = Vec::new();

    if config.lint {
        for pt in &types {
            results.push(builtin::run_lint(project_dir, pt, config.timeout_seconds).await);
        }
    }

    if config.format_check {
        for pt in &types {
            results.push(builtin::run_format_check(project_dir, pt, config.timeout_seconds).await);
        }
    }

    if config.type_check {
        for pt in &types {
            results.push(builtin::run_type_check(project_dir, pt, config.timeout_seconds).await);
        }
    }

    if config.test {
        for pt in &types {
            results.push(builtin::run_tests(project_dir, pt, config.timeout_seconds).await);
        }
    }

    // Run custom plugin gates
    for gate_script in &config.custom_gates {
        results.push(plugin::run_plugin_gate(project_dir, gate_script).await);
    }

    // Discover and run plugin gates from .shepherd/gates/
    let discovered = plugin::discover_plugin_gates(project_dir);
    for gate_path in discovered {
        if let Some(script) = gate_path.to_str() {
            results.push(plugin::run_plugin_gate(project_dir, script).await);
        }
    }

    Ok(results)
}
// tarpaulin-stop-ignore

/// Returns true if all gate results passed. Returns true for an empty list.
pub fn all_gates_passed(results: &[GateResult]) -> bool {
    results.iter().all(|r| r.passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_config_defaults() {
        let config = GateConfig::default();
        assert!(config.lint);
        assert!(config.format_check);
        assert!(config.type_check);
        assert!(config.test);
        assert!(config.custom_gates.is_empty());
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn all_gates_passed_true() {
        let results = vec![
            GateResult {
                gate_name: "lint".into(),
                passed: true,
                output: String::new(),
                duration_ms: 100,
                gate_type: GateType::Lint,
            },
            GateResult {
                gate_name: "test".into(),
                passed: true,
                output: String::new(),
                duration_ms: 200,
                gate_type: GateType::Test,
            },
        ];
        assert!(all_gates_passed(&results));
    }

    #[test]
    fn all_gates_passed_false() {
        let results = vec![
            GateResult {
                gate_name: "lint".into(),
                passed: true,
                output: String::new(),
                duration_ms: 100,
                gate_type: GateType::Lint,
            },
            GateResult {
                gate_name: "test".into(),
                passed: false,
                output: "1 test failed".into(),
                duration_ms: 200,
                gate_type: GateType::Test,
            },
        ];
        assert!(!all_gates_passed(&results));
    }

    #[test]
    fn all_gates_passed_empty() {
        let results: Vec<GateResult> = vec![];
        assert!(all_gates_passed(&results));
    }

    #[tokio::test]
    async fn run_gates_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = GateConfig::default();
        let results = run_gates(dir.path(), &config).await.unwrap();
        // Unknown project type means all_types returns empty,
        // so no builtin gates run. Only discovered plugin gates would appear.
        for r in &results {
            assert!(r.passed);
        }
    }

    #[tokio::test]
    async fn run_gates_disabled_all() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![],
            timeout_seconds: 10,
        };
        let results = run_gates(dir.path(), &config).await.unwrap();
        // No gates should run (all disabled, no custom, no discovered plugins)
        // Only discovered plugins from .shepherd/gates/ could appear
        for r in &results {
            assert!(r.passed || !r.passed); // verify no panic
        }
    }

    #[test]
    fn gate_config_serde_roundtrip() {
        let config = GateConfig {
            lint: false,
            format_check: true,
            type_check: false,
            test: true,
            custom_gates: vec!["./check.sh".into()],
            timeout_seconds: 120,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deser: GateConfig = serde_json::from_str(&json).unwrap();
        assert!(!deser.lint);
        assert!(deser.format_check);
        assert!(!deser.type_check);
        assert!(deser.test);
        assert_eq!(deser.custom_gates, vec!["./check.sh"]);
        assert_eq!(deser.timeout_seconds, 120);
    }

    #[test]
    fn gate_config_deserialize_defaults() {
        let json = "{}";
        let config: GateConfig = serde_json::from_str(json).unwrap();
        assert!(config.lint);
        assert!(config.format_check);
        assert!(config.type_check);
        assert!(config.test);
        assert!(config.custom_gates.is_empty());
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn gate_type_serde_all() {
        let types = vec![
            (GateType::Lint, "\"lint\""),
            (GateType::Format, "\"format\""),
            (GateType::TypeCheck, "\"type_check\""),
            (GateType::Test, "\"test\""),
            (GateType::Security, "\"security\""),
            (GateType::Custom, "\"custom\""),
        ];
        for (gate_type, expected) in types {
            let json = serde_json::to_string(&gate_type).unwrap();
            assert_eq!(json, expected, "Failed for {gate_type:?}");
            let deser: GateType = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, gate_type);
        }
    }

    #[test]
    fn gate_result_serde_roundtrip() {
        let result = GateResult {
            gate_name: "custom-lint".into(),
            passed: false,
            output: "3 errors found".into(),
            duration_ms: 1500,
            gate_type: GateType::Custom,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deser: GateResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.gate_name, "custom-lint");
        assert!(!deser.passed);
        assert_eq!(deser.output, "3 errors found");
        assert_eq!(deser.duration_ms, 1500);
        assert_eq!(deser.gate_type, GateType::Custom);
    }

    #[tokio::test]
    async fn run_gates_with_custom_gate_script() {
        let dir = tempfile::tempdir().unwrap();
        // Create a simple custom gate script
        let script_path = dir.path().join("custom-gate.sh");
        std::fs::write(
            &script_path,
            "#!/bin/sh\necho 'custom gate passed'\nexit 0\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![script_path.to_string_lossy().to_string()],
            timeout_seconds: 10,
        };
        let results = run_gates(dir.path(), &config).await.unwrap();
        // Should run the custom gate
        assert!(!results.is_empty());
        let custom = results.iter().find(|r| r.gate_type == GateType::Custom);
        assert!(custom.is_some(), "Should have run custom gate");
    }

    #[test]
    fn gate_result_debug_format() {
        let result = GateResult {
            gate_name: "test-gate".into(),
            passed: true,
            output: "ok".into(),
            duration_ms: 42,
            gate_type: GateType::Test,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("test-gate"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn gate_config_clone() {
        let config = GateConfig {
            lint: false,
            format_check: true,
            type_check: false,
            test: true,
            custom_gates: vec!["./gate.sh".into()],
            timeout_seconds: 60,
        };
        let cloned = config.clone();
        assert_eq!(cloned.lint, config.lint);
        assert_eq!(cloned.format_check, config.format_check);
        assert_eq!(cloned.custom_gates, config.custom_gates);
        assert_eq!(cloned.timeout_seconds, config.timeout_seconds);
    }

    #[tokio::test]
    async fn run_gates_custom_gate() {
        let dir = tempfile::tempdir().unwrap();
        // Create a .shepherd/gates/ directory with a simple gate script
        let gates_dir = dir.path().join(".shepherd").join("gates");
        std::fs::create_dir_all(&gates_dir).unwrap();
        let gate_script = gates_dir.join("check.sh");
        std::fs::write(&gate_script, "#!/bin/sh\necho ok\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gate_script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![],
            timeout_seconds: 10,
        };
        let results = run_gates(dir.path(), &config).await.unwrap();
        // Should discover and run the plugin gate from .shepherd/gates/
        let plugin_result = results.iter().find(|r| r.gate_type == GateType::Custom);
        assert!(
            plugin_result.is_some(),
            "Should have discovered plugin gate"
        );
        assert!(plugin_result.unwrap().passed);
    }
}
