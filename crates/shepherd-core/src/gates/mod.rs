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

/// Run all enabled quality gates for the project at the given path.
pub async fn run_gates(project_dir: &Path, config: &GateConfig) -> Result<Vec<GateResult>> {
    let project_type = builtin::detect_project_type(project_dir);
    let types = builtin::all_types(&project_type);
    let mut results = Vec::new();

    if config.lint {
        for pt in &types {
            results.push(
                builtin::run_lint(project_dir, pt, config.timeout_seconds).await,
            );
        }
    }

    if config.format_check {
        for pt in &types {
            results.push(
                builtin::run_format_check(project_dir, pt, config.timeout_seconds).await,
            );
        }
    }

    if config.type_check {
        for pt in &types {
            results.push(
                builtin::run_type_check(project_dir, pt, config.timeout_seconds).await,
            );
        }
    }

    if config.test {
        for pt in &types {
            results.push(
                builtin::run_tests(project_dir, pt, config.timeout_seconds).await,
            );
        }
    }

    // Run custom plugin gates
    for gate_script in &config.custom_gates {
        results.push(
            plugin::run_plugin_gate(project_dir, gate_script).await,
        );
    }

    // Discover and run plugin gates from .shepherd/gates/
    let discovered = plugin::discover_plugin_gates(project_dir);
    for gate_path in discovered {
        if let Some(script) = gate_path.to_str() {
            results.push(
                plugin::run_plugin_gate(project_dir, script).await,
            );
        }
    }

    Ok(results)
}

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
}
