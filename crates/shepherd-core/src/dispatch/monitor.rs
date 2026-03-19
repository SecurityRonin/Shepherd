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
    approve_seq: String,
    deny_seq: String,
}

impl SessionMonitor {
    pub fn new(status: &StatusSection, permissions: &PermissionsSection) -> Self {
        let compile = |patterns: &[String]| -> Vec<Regex> {
            patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
        };
        Self {
            working_patterns: compile(&status.working_patterns),
            idle_patterns: compile(&status.idle_patterns),
            input_patterns: compile(&status.input_patterns),
            error_patterns: compile(&status.error_patterns),
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
                let tool_name = self.extract_tool_name(output).unwrap_or_default();
                let tool_args = self.extract_tool_args(output).unwrap_or_default();
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

    fn extract_tool_name(&self, output: &str) -> Option<String> {
        if output.contains("bash") || output.contains("command") {
            Some("bash".to_string())
        } else if output.contains("write") || output.contains("file") {
            Some("file_write".to_string())
        } else {
            Some("unknown".to_string())
        }
    }

    fn extract_tool_args(&self, output: &str) -> Option<String> {
        Some(output.trim().to_string())
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
}
