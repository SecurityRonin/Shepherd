use axum::{http::StatusCode, Json};
use serde_json::Value;

/// Known agent CLI plugins and their binary names.
const KNOWN_PLUGINS: &[(&str, &str)] = &[
    ("claude-code", "claude"),
    ("aider", "aider"),
    ("codex", "codex"),
    ("goose", "goose"),
    ("opencode", "opencode"),
    ("amp", "amp"),
    ("cline", "cline"),
    ("roo", "roo"),
];

/// Check if a binary exists on PATH.
fn binary_exists_on_path(binary: &str) -> bool {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return true;
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{}.exe", binary));
                if exe.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

#[tracing::instrument]
pub async fn detected_plugins() -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let detected: Vec<&str> = KNOWN_PLUGINS
        .iter()
        .filter(|(_, binary)| binary_exists_on_path(binary))
        .map(|(id, _)| *id)
        .collect();
    Ok(Json(serde_json::json!({ "detected": detected })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_exists_finds_common_tools() {
        #[cfg(unix)]
        assert!(binary_exists_on_path("ls"));
        #[cfg(windows)]
        assert!(binary_exists_on_path("cmd"));
    }

    #[test]
    fn binary_exists_returns_false_for_nonexistent() {
        assert!(!binary_exists_on_path(
            "definitely_not_a_real_binary_xyz123"
        ));
    }
}
