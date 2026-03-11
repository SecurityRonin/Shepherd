use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;

use super::{GateResult, GateType};

/// Run a plugin gate script, capturing its output and exit code.
pub async fn run_plugin_gate(project_dir: &Path, script: &str) -> GateResult {
    let start = Instant::now();
    let gate_name = Path::new(script)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(script)
        .to_string();

    let result = Command::new("sh")
        .arg("-c")
        .arg(script)
        .current_dir(project_dir)
        .env("SHEPHERD_PROJECT_DIR", project_dir.as_os_str())
        .output()
        .await;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            GateResult {
                gate_name,
                passed: output.status.success(),
                output: combined,
                duration_ms: start.elapsed().as_millis() as u64,
                gate_type: GateType::Custom,
            }
        }
        Err(e) => GateResult {
            gate_name,
            passed: false,
            output: format!("Failed to run plugin gate: {e}"),
            duration_ms: start.elapsed().as_millis() as u64,
            gate_type: GateType::Custom,
        },
    }
}

/// Discover plugin gate scripts in .shepherd/gates/ directory.
/// Returns sorted paths for .sh, .bash, .py, .js, .ts files.
pub fn discover_plugin_gates(project_dir: &Path) -> Vec<PathBuf> {
    let gates_dir = project_dir.join(".shepherd").join("gates");
    if !gates_dir.is_dir() {
        return Vec::new();
    }

    let valid_extensions = ["sh", "bash", "py", "js", "ts"];

    let mut paths: Vec<PathBuf> = std::fs::read_dir(&gates_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| valid_extensions.contains(&ext))
                .unwrap_or(false)
        })
        .collect();

    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn discover_no_gates_dir() {
        let dir = TempDir::new().unwrap();
        let result = discover_plugin_gates(dir.path());
        assert!(result.is_empty());
    }

    #[test]
    fn discover_plugin_gates_finds_scripts() {
        let dir = TempDir::new().unwrap();
        let gates_dir = dir.path().join(".shepherd").join("gates");
        std::fs::create_dir_all(&gates_dir).unwrap();

        std::fs::write(gates_dir.join("check-lint.sh"), "#!/bin/sh\nexit 0").unwrap();
        std::fs::write(gates_dir.join("security.py"), "print('ok')").unwrap();
        std::fs::write(gates_dir.join("readme.md"), "# Not a gate").unwrap();

        let result = discover_plugin_gates(dir.path());
        assert_eq!(result.len(), 2);
        // Should find .sh and .py but skip .md
        let names: Vec<String> = result
            .iter()
            .filter_map(|p| p.file_name().and_then(|f| f.to_str()).map(String::from))
            .collect();
        assert!(names.contains(&"check-lint.sh".to_string()));
        assert!(names.contains(&"security.py".to_string()));
        assert!(!names.contains(&"readme.md".to_string()));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_plugin_gate_success() {
        let dir = TempDir::new().unwrap();
        let script = dir.path().join("gate.sh");
        std::fs::write(&script, "#!/bin/sh\necho 'all good'\nexit 0").unwrap();

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let result =
            run_plugin_gate(dir.path(), script.to_str().unwrap()).await;
        assert!(result.passed);
        assert!(result.output.contains("all good"));
        assert_eq!(result.gate_type, GateType::Custom);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_plugin_gate_failure() {
        let dir = TempDir::new().unwrap();
        let script = dir.path().join("gate.sh");
        std::fs::write(&script, "#!/bin/sh\necho 'lint error'\nexit 1").unwrap();

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let result =
            run_plugin_gate(dir.path(), script.to_str().unwrap()).await;
        assert!(!result.passed);
        assert!(result.output.contains("lint error"));
        assert_eq!(result.gate_type, GateType::Custom);
    }
}
