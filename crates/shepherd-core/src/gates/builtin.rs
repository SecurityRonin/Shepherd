use std::path::Path;
use std::time::Instant;
use tokio::process::Command;

use super::{GateResult, GateType};

/// Known project types for auto-detection.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    Node,
    TypeScript,
    Python,
    Mixed(Vec<ProjectType>),
    Unknown,
}

/// Detect the project type based on marker files in the project directory.
pub fn detect_project_type(project_dir: &Path) -> ProjectType {
    let has_cargo = project_dir.join("Cargo.toml").exists();
    let has_tsconfig = project_dir.join("tsconfig.json").exists();
    let has_package_json = project_dir.join("package.json").exists();
    let has_pyproject = project_dir.join("pyproject.toml").exists();
    let has_setup_py = project_dir.join("setup.py").exists();
    let has_python = has_pyproject || has_setup_py;

    let mut types = Vec::new();

    if has_cargo {
        types.push(ProjectType::Rust);
    }
    // TypeScript wins over Node if tsconfig exists
    if has_tsconfig {
        types.push(ProjectType::TypeScript);
    } else if has_package_json {
        types.push(ProjectType::Node);
    }
    if has_python {
        types.push(ProjectType::Python);
    }

    match types.len() {
        0 => ProjectType::Unknown,
        1 => types.into_iter().next().unwrap(),
        _ => ProjectType::Mixed(types),
    }
}

/// Flatten a ProjectType (including Mixed) into a Vec of individual types.
pub fn all_types(project_type: &ProjectType) -> Vec<&ProjectType> {
    match project_type {
        ProjectType::Mixed(types) => types.iter().collect(),
        ProjectType::Unknown => vec![],
        other => vec![other],
    }
}

/// Run a shell command with a timeout, returning (success, output).
async fn run_command(
    project_dir: &Path,
    program: &str,
    args: &[&str],
    timeout_seconds: u64,
) -> (bool, String) {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds),
        Command::new(program)
            .args(args)
            .current_dir(project_dir)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            (output.status.success(), combined)
        }
        Ok(Err(e)) => (false, format!("Failed to execute command: {e}")),
        Err(_) => (false, format!("Command timed out after {timeout_seconds}s")),
    }
}

/// Run the lint gate for a given project type.
pub async fn run_lint(
    project_dir: &Path,
    project_type: &ProjectType,
    timeout_seconds: u64,
) -> GateResult {
    let start = Instant::now();
    let (program, args, name) = match project_type {
        ProjectType::Rust => ("cargo", vec!["clippy", "--", "-D", "warnings"], "rust-lint"),
        ProjectType::Node | ProjectType::TypeScript => {
            ("npx", vec!["eslint", "."], "js-lint")
        }
        ProjectType::Python => ("ruff", vec!["check", "."], "python-lint"),
        _ => {
            return GateResult {
                gate_name: "lint".into(),
                passed: true,
                output: "No lint tool for project type".into(),
                duration_ms: 0,
                gate_type: GateType::Lint,
            }
        }
    };

    let (passed, output) = run_command(project_dir, program, &args, timeout_seconds).await;
    GateResult {
        gate_name: name.into(),
        passed,
        output,
        duration_ms: start.elapsed().as_millis() as u64,
        gate_type: GateType::Lint,
    }
}

/// Run the format check gate for a given project type.
pub async fn run_format_check(
    project_dir: &Path,
    project_type: &ProjectType,
    timeout_seconds: u64,
) -> GateResult {
    let start = Instant::now();
    let (program, args, name) = match project_type {
        ProjectType::Rust => (
            "cargo",
            vec!["fmt", "--", "--check"],
            "rust-format",
        ),
        ProjectType::Node | ProjectType::TypeScript => {
            ("npx", vec!["prettier", "--check", "."], "js-format")
        }
        ProjectType::Python => ("ruff", vec!["format", "--check", "."], "python-format"),
        _ => {
            return GateResult {
                gate_name: "format-check".into(),
                passed: true,
                output: "No format tool for project type".into(),
                duration_ms: 0,
                gate_type: GateType::Format,
            }
        }
    };

    let (passed, output) = run_command(project_dir, program, &args, timeout_seconds).await;
    GateResult {
        gate_name: name.into(),
        passed,
        output,
        duration_ms: start.elapsed().as_millis() as u64,
        gate_type: GateType::Format,
    }
}

/// Run the type check gate for a given project type.
pub async fn run_type_check(
    project_dir: &Path,
    project_type: &ProjectType,
    timeout_seconds: u64,
) -> GateResult {
    let start = Instant::now();
    let (program, args, name) = match project_type {
        ProjectType::Rust => ("cargo", vec!["check"], "rust-typecheck"),
        ProjectType::TypeScript => ("npx", vec!["tsc", "--noEmit"], "ts-typecheck"),
        ProjectType::Python => ("mypy", vec!["."], "python-typecheck"),
        _ => {
            return GateResult {
                gate_name: "type-check".into(),
                passed: true,
                output: "No type check tool for project type".into(),
                duration_ms: 0,
                gate_type: GateType::TypeCheck,
            }
        }
    };

    let (passed, output) = run_command(project_dir, program, &args, timeout_seconds).await;
    GateResult {
        gate_name: name.into(),
        passed,
        output,
        duration_ms: start.elapsed().as_millis() as u64,
        gate_type: GateType::TypeCheck,
    }
}

/// Run the test gate for a given project type.
pub async fn run_tests(
    project_dir: &Path,
    project_type: &ProjectType,
    timeout_seconds: u64,
) -> GateResult {
    let start = Instant::now();
    let (program, args, name) = match project_type {
        ProjectType::Rust => ("cargo", vec!["test"], "rust-test"),
        ProjectType::Node | ProjectType::TypeScript => {
            // Try vitest first by checking for vitest in package.json, fallback to jest
            let vitest_config = project_dir.join("vitest.config.ts");
            let vitest_config_js = project_dir.join("vitest.config.js");
            if vitest_config.exists() || vitest_config_js.exists() {
                ("npx", vec!["vitest", "run"], "js-test")
            } else {
                ("npx", vec!["jest"], "js-test")
            }
        }
        ProjectType::Python => ("pytest", vec![], "python-test"),
        _ => {
            return GateResult {
                gate_name: "test".into(),
                passed: true,
                output: "No test tool for project type".into(),
                duration_ms: 0,
                gate_type: GateType::Test,
            }
        }
    };

    let (passed, output) = run_command(project_dir, program, &args, timeout_seconds).await;
    GateResult {
        gate_name: name.into(),
        passed,
        output,
        duration_ms: start.elapsed().as_millis() as u64,
        gate_type: GateType::Test,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_rust_project() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Rust);
    }

    #[test]
    fn detect_node_project() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Node);
    }

    #[test]
    fn detect_typescript_project() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        // TypeScript wins over Node when tsconfig exists
        assert_eq!(detect_project_type(dir.path()), ProjectType::TypeScript);
    }

    #[test]
    fn detect_python_project() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Python);
    }

    #[test]
    fn detect_mixed_project() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        let result = detect_project_type(dir.path());
        match result {
            ProjectType::Mixed(types) => {
                assert!(types.contains(&ProjectType::Rust));
                assert!(types.contains(&ProjectType::TypeScript));
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Mixed project type"),
        }
    }

    #[test]
    fn detect_unknown_project() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Unknown);
    }

    #[test]
    fn all_types_single() {
        let types = all_types(&ProjectType::Rust);
        assert_eq!(types.len(), 1);
        assert_eq!(*types[0], ProjectType::Rust);
    }

    #[test]
    fn all_types_mixed() {
        let mixed = ProjectType::Mixed(vec![ProjectType::Rust, ProjectType::TypeScript]);
        let types = all_types(&mixed);
        assert_eq!(types.len(), 2);
        assert_eq!(*types[0], ProjectType::Rust);
        assert_eq!(*types[1], ProjectType::TypeScript);
    }

    #[test]
    fn all_types_unknown() {
        let types = all_types(&ProjectType::Unknown);
        assert!(types.is_empty());
    }

    #[test]
    fn all_types_node() {
        let types = all_types(&ProjectType::Node);
        assert_eq!(types.len(), 1);
        assert_eq!(*types[0], ProjectType::Node);
    }

    #[test]
    fn all_types_python() {
        let types = all_types(&ProjectType::Python);
        assert_eq!(types.len(), 1);
        assert_eq!(*types[0], ProjectType::Python);
    }

    #[test]
    fn detect_python_setup_py() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("setup.py"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Python);
    }

    #[test]
    fn detect_mixed_rust_python() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        match detect_project_type(dir.path()) {
            ProjectType::Mixed(types) => {
                assert!(types.contains(&ProjectType::Rust));
                assert!(types.contains(&ProjectType::Python));
            }
            other => panic!("Expected Mixed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn run_command_success() {
        let dir = TempDir::new().unwrap();
        let (passed, output) = run_command(dir.path(), "echo", &["hello"], 10).await;
        assert!(passed);
        assert!(output.contains("hello"));
    }

    #[tokio::test]
    async fn run_command_failure() {
        let dir = TempDir::new().unwrap();
        let (passed, _output) = run_command(dir.path(), "false", &[], 10).await;
        assert!(!passed);
    }

    #[tokio::test]
    async fn run_command_nonexistent_program() {
        let dir = TempDir::new().unwrap();
        let (passed, output) = run_command(dir.path(), "nonexistent_program_xyz", &[], 10).await;
        assert!(!passed);
        assert!(output.contains("Failed to execute"));
    }

    #[tokio::test]
    async fn run_command_timeout() {
        let dir = TempDir::new().unwrap();
        let (passed, output) = run_command(dir.path(), "sleep", &["30"], 1).await;
        assert!(!passed);
        assert!(output.contains("timed out"));
    }

    #[tokio::test]
    async fn run_lint_unknown_project() {
        let dir = TempDir::new().unwrap();
        let result = run_lint(dir.path(), &ProjectType::Unknown, 10).await;
        assert!(result.passed);
        assert!(result.output.contains("No lint tool"));
        assert_eq!(result.gate_type, GateType::Lint);
        assert_eq!(result.duration_ms, 0);
    }

    #[tokio::test]
    async fn run_lint_mixed_project() {
        let dir = TempDir::new().unwrap();
        let mixed = ProjectType::Mixed(vec![ProjectType::Rust]);
        let result = run_lint(dir.path(), &mixed, 10).await;
        assert!(result.passed);
        assert!(result.output.contains("No lint tool"));
    }

    #[tokio::test]
    async fn run_format_check_unknown() {
        let dir = TempDir::new().unwrap();
        let result = run_format_check(dir.path(), &ProjectType::Unknown, 10).await;
        assert!(result.passed);
        assert!(result.output.contains("No format tool"));
        assert_eq!(result.gate_type, GateType::Format);
    }

    #[tokio::test]
    async fn run_type_check_unknown() {
        let dir = TempDir::new().unwrap();
        let result = run_type_check(dir.path(), &ProjectType::Unknown, 10).await;
        assert!(result.passed);
        assert!(result.output.contains("No type check tool"));
        assert_eq!(result.gate_type, GateType::TypeCheck);
    }

    #[tokio::test]
    async fn run_type_check_node_skipped() {
        let dir = TempDir::new().unwrap();
        let result = run_type_check(dir.path(), &ProjectType::Node, 10).await;
        // Node has no type check (only TypeScript does)
        assert!(result.passed);
        assert!(result.output.contains("No type check tool"));
    }

    #[tokio::test]
    async fn run_tests_unknown() {
        let dir = TempDir::new().unwrap();
        let result = run_tests(dir.path(), &ProjectType::Unknown, 10).await;
        assert!(result.passed);
        assert!(result.output.contains("No test tool"));
        assert_eq!(result.gate_type, GateType::Test);
    }

    #[tokio::test]
    async fn run_tests_node_jest() {
        // Without vitest config, should choose jest
        let dir = TempDir::new().unwrap();
        let result = run_tests(dir.path(), &ProjectType::Node, 5).await;
        // npx jest will fail since no project, but it exercises the jest path
        assert_eq!(result.gate_name, "js-test");
        assert_eq!(result.gate_type, GateType::Test);
    }

    #[tokio::test]
    async fn run_tests_node_vitest() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("vitest.config.ts"), "").unwrap();
        let result = run_tests(dir.path(), &ProjectType::Node, 5).await;
        assert_eq!(result.gate_name, "js-test");
        assert_eq!(result.gate_type, GateType::Test);
    }

    #[tokio::test]
    async fn run_tests_vitest_js_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("vitest.config.js"), "").unwrap();
        let result = run_tests(dir.path(), &ProjectType::TypeScript, 5).await;
        assert_eq!(result.gate_name, "js-test");
    }

    #[tokio::test]
    async fn run_lint_rust_gate_name() {
        let dir = TempDir::new().unwrap();
        let result = run_lint(dir.path(), &ProjectType::Rust, 5).await;
        assert_eq!(result.gate_name, "rust-lint");
        assert_eq!(result.gate_type, GateType::Lint);
    }

    #[tokio::test]
    async fn run_lint_node_gate_name() {
        let dir = TempDir::new().unwrap();
        let result = run_lint(dir.path(), &ProjectType::Node, 5).await;
        assert_eq!(result.gate_name, "js-lint");
    }

    #[tokio::test]
    async fn run_lint_typescript_gate_name() {
        let dir = TempDir::new().unwrap();
        let result = run_lint(dir.path(), &ProjectType::TypeScript, 5).await;
        assert_eq!(result.gate_name, "js-lint");
    }

    #[tokio::test]
    async fn run_lint_python_gate_name() {
        let dir = TempDir::new().unwrap();
        let result = run_lint(dir.path(), &ProjectType::Python, 5).await;
        assert_eq!(result.gate_name, "python-lint");
    }

    #[tokio::test]
    async fn run_format_check_rust() {
        let dir = TempDir::new().unwrap();
        let result = run_format_check(dir.path(), &ProjectType::Rust, 5).await;
        assert_eq!(result.gate_name, "rust-format");
        assert_eq!(result.gate_type, GateType::Format);
    }

    #[tokio::test]
    async fn run_format_check_node() {
        let dir = TempDir::new().unwrap();
        let result = run_format_check(dir.path(), &ProjectType::Node, 5).await;
        assert_eq!(result.gate_name, "js-format");
    }

    #[tokio::test]
    async fn run_format_check_python() {
        let dir = TempDir::new().unwrap();
        let result = run_format_check(dir.path(), &ProjectType::Python, 5).await;
        assert_eq!(result.gate_name, "python-format");
    }

    #[tokio::test]
    async fn run_type_check_rust() {
        let dir = TempDir::new().unwrap();
        let result = run_type_check(dir.path(), &ProjectType::Rust, 5).await;
        assert_eq!(result.gate_name, "rust-typecheck");
        assert_eq!(result.gate_type, GateType::TypeCheck);
    }

    #[tokio::test]
    async fn run_type_check_typescript() {
        let dir = TempDir::new().unwrap();
        let result = run_type_check(dir.path(), &ProjectType::TypeScript, 5).await;
        assert_eq!(result.gate_name, "ts-typecheck");
    }

    #[tokio::test]
    async fn run_type_check_python() {
        let dir = TempDir::new().unwrap();
        let result = run_type_check(dir.path(), &ProjectType::Python, 5).await;
        assert_eq!(result.gate_name, "python-typecheck");
    }

    #[tokio::test]
    async fn run_tests_rust() {
        let dir = TempDir::new().unwrap();
        let result = run_tests(dir.path(), &ProjectType::Rust, 5).await;
        assert_eq!(result.gate_name, "rust-test");
        assert_eq!(result.gate_type, GateType::Test);
    }

    #[tokio::test]
    async fn run_tests_python() {
        let dir = TempDir::new().unwrap();
        let result = run_tests(dir.path(), &ProjectType::Python, 5).await;
        assert_eq!(result.gate_name, "python-test");
    }

    #[tokio::test]
    async fn run_command_with_stderr() {
        let dir = TempDir::new().unwrap();
        let (passed, output) = run_command(dir.path(), "sh", &["-c", "echo err >&2; exit 1"], 10).await;
        assert!(!passed);
        assert!(output.contains("err"));
    }

    #[test]
    fn gate_result_fields() {
        let r = GateResult {
            gate_name: "test".into(),
            passed: true,
            output: "ok".into(),
            duration_ms: 42,
            gate_type: GateType::Lint,
        };
        assert_eq!(r.gate_name, "test");
        assert!(r.passed);
        assert_eq!(r.duration_ms, 42);
        assert_eq!(r.gate_type, GateType::Lint);
    }
}
