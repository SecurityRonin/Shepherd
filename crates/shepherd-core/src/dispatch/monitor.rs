//! SessionMonitor — parses PTY output to detect permission requests
//! and agent status changes based on adapter-defined patterns.

use crate::adapters::protocol::{PermissionsSection, StatusSection};
use regex::Regex;

/// What the monitor detected in the output.
#[derive(Debug, Clone, PartialEq)]
pub enum Detection {
    /// Agent is actively working.
    Working,
    /// Agent is idle / finished.
    Idle,
    /// Agent is requesting permission for an action.
    PermissionRequest {
        tool_name: String,
        tool_args: String,
    },
    /// Agent encountered an error.
    Error(String),
    /// No pattern matched.
    None,
}

/// Monitors PTY output for a single task's agent session.
pub struct SessionMonitor {
    working_patterns: Vec<Regex>,
    idle_patterns: Vec<Regex>,
    input_patterns: Vec<Regex>,
    error_patterns: Vec<Regex>,
    extraction_patterns: Vec<Regex>,
    approve_seq: String,
    deny_seq: String,
}

impl SessionMonitor {
    pub fn new(status: &StatusSection, permissions: &PermissionsSection) -> Self {
        let compile = |patterns: &[String]| -> Vec<Regex> {
            patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
        };
        let extraction_patterns = permissions
            .extraction_patterns
            .iter()
            .filter_map(|p| Regex::new(&p.regex).ok())
            .collect();
        Self {
            working_patterns: compile(&status.working_patterns),
            idle_patterns: compile(&status.idle_patterns),
            input_patterns: compile(&status.input_patterns),
            error_patterns: compile(&status.error_patterns),
            extraction_patterns,
            approve_seq: permissions.approve.clone(),
            deny_seq: permissions.deny.clone(),
        }
    }

    /// Analyze a chunk of PTY output. Returns the most significant detection.
    /// Checks patterns in priority order: error > input > working > idle.
    pub fn analyze(&self, output: &str) -> Detection {
        for re in &self.error_patterns {
            if let Some(m) = re.find(output) {
                return Detection::Error(m.as_str().to_string());
            }
        }
        for re in &self.input_patterns {
            if re.is_match(output) {
                let (tool_name, tool_args) = self.extract_tool_info(output);
                return Detection::PermissionRequest {
                    tool_name,
                    tool_args,
                };
            }
        }
        for re in &self.working_patterns {
            if re.is_match(output) {
                return Detection::Working;
            }
        }
        for re in &self.idle_patterns {
            if re.is_match(output) {
                return Detection::Idle;
            }
        }
        Detection::None
    }

    /// Get the byte sequence to send to PTY stdin to approve.
    pub fn approve_sequence(&self) -> &str {
        &self.approve_seq
    }

    /// Get the byte sequence to send to PTY stdin to deny.
    pub fn deny_sequence(&self) -> &str {
        &self.deny_seq
    }

    /// Try extraction_patterns first (named capture groups), fall back to heuristic.
    fn extract_tool_info(&self, output: &str) -> (String, String) {
        for re in &self.extraction_patterns {
            if let Some(caps) = re.captures(output) {
                let tool_name = caps
                    .name("tool_name")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let tool_args = caps
                    .name("tool_args")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                if !tool_name.is_empty() {
                    return (tool_name, tool_args);
                }
            }
        }
        // Fallback: legacy heuristic for adapters without extraction_patterns
        let tool_name = if output.contains("bash") || output.contains("command") {
            "bash".to_string()
        } else if output.contains("write") || output.contains("file") {
            "file_write".to_string()
        } else {
            "unknown".to_string()
        };
        (tool_name, output.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::protocol::{PermissionsSection, StatusSection};

    fn claude_code_status() -> StatusSection {
        StatusSection {
            working_patterns: vec![r"⠋|⠙|⠹|⠸|⠼|⠴|⠦|⠧|⠇|⠏".into()],
            idle_patterns: vec![r"\$\s*$".into()],
            input_patterns: vec![r"Allow|Do you want to".into()],
            error_patterns: vec![r"Error:|panic!|FAILED".into()],
        }
    }

    fn claude_code_permissions() -> PermissionsSection {
        PermissionsSection {
            approve: "y\n".into(),
            approve_all: "!\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        }
    }

    #[test]
    fn detects_working_state() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.analyze("⠋ Processing files..."), Detection::Working);
    }

    #[test]
    fn detects_idle_state() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.analyze("$ "), Detection::Idle);
    }

    #[test]
    fn detects_permission_request() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("Allow bash command: cargo test?") {
            Detection::PermissionRequest { tool_name, .. } => {
                assert_eq!(tool_name, "bash");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn detects_error() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("Error: file not found") {
            Detection::Error(msg) => assert!(msg.contains("Error")),
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn error_takes_priority_over_working() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("⠋ Error: compilation failed") {
            Detection::Error(_) => {} // Error should win
            other => panic!("Expected Error priority, got {:?}", other),
        }
    }

    #[test]
    fn no_match_returns_none() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.analyze("just some normal output"), Detection::None);
    }

    #[test]
    fn approve_and_deny_sequences() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.approve_sequence(), "y\n");
        assert_eq!(monitor.deny_sequence(), "n\n");
    }

    #[test]
    fn detects_file_write_permission() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("Do you want to write to config.toml?") {
            Detection::PermissionRequest { tool_name, .. } => {
                assert_eq!(tool_name, "file_write");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn detects_unknown_tool_permission() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("Allow network access to example.com?") {
            Detection::PermissionRequest { tool_name, .. } => {
                assert_eq!(tool_name, "unknown");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    // ── Extraction pattern tests ─────────────────────────────────

    fn permissions_with_patterns() -> PermissionsSection {
        PermissionsSection {
            approve: "y\n".into(),
            approve_all: "!\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![
                crate::adapters::protocol::ExtractionPattern {
                    regex: r"(?P<tool_name>Bash|Write|Read|Edit)\((?P<tool_args>[^)]+)\)".into(),
                },
                crate::adapters::protocol::ExtractionPattern {
                    regex: r"(?P<tool_name>\w+):\s+(?P<tool_args>.+)".into(),
                },
            ],
        }
    }

    #[test]
    fn extracts_tool_name_and_args_via_pattern() {
        let monitor = SessionMonitor::new(&claude_code_status(), &permissions_with_patterns());
        match monitor.analyze("Allow Bash(cargo test --release)?") {
            Detection::PermissionRequest {
                tool_name,
                tool_args,
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_args, "cargo test --release");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn extracts_write_tool_via_pattern() {
        let monitor = SessionMonitor::new(&claude_code_status(), &permissions_with_patterns());
        match monitor.analyze("Allow Write(src/main.rs)?") {
            Detection::PermissionRequest {
                tool_name,
                tool_args,
            } => {
                assert_eq!(tool_name, "Write");
                assert_eq!(tool_args, "src/main.rs");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn falls_back_to_second_pattern() {
        let monitor = SessionMonitor::new(&claude_code_status(), &permissions_with_patterns());
        match monitor.analyze("Allow execute: npm install lodash") {
            Detection::PermissionRequest {
                tool_name,
                tool_args,
            } => {
                assert_eq!(tool_name, "execute");
                assert_eq!(tool_args, "npm install lodash");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn no_extraction_patterns_uses_fallback() {
        // With empty extraction_patterns, should still work (fallback to legacy logic)
        let perms = PermissionsSection {
            approve: "y\n".into(),
            approve_all: "!\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        };
        let monitor = SessionMonitor::new(&claude_code_status(), &perms);
        match monitor.analyze("Allow bash command: cargo test?") {
            Detection::PermissionRequest { tool_name, .. } => {
                assert_eq!(tool_name, "bash");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn extraction_pattern_with_only_tool_name() {
        let perms = PermissionsSection {
            approve: "y\n".into(),
            approve_all: "!\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![crate::adapters::protocol::ExtractionPattern {
                regex: r"Allow (?P<tool_name>\w+)".into(),
            }],
        };
        let monitor = SessionMonitor::new(&claude_code_status(), &perms);
        match monitor.analyze("Allow Bash(cargo test)?") {
            Detection::PermissionRequest {
                tool_name,
                tool_args,
            } => {
                assert_eq!(tool_name, "Bash");
                // tool_args should be empty when no capture group for it
                assert!(tool_args.is_empty());
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }
}
