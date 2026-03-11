use crate::adapters::protocol::StatusSection;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Working(String),
    Idle,
    NeedsInput(String),
    Error(String),
}

pub fn detect_status(output_line: &str, patterns: &StatusSection) -> Option<AgentStatus> {
    for p in &patterns.error_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::Error(output_line.to_string()));
        }
    }
    for p in &patterns.input_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::NeedsInput(output_line.to_string()));
        }
    }
    for p in &patterns.working_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::Working(output_line.to_string()));
        }
    }
    for p in &patterns.idle_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::Idle);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claude_patterns() -> StatusSection {
        StatusSection {
            working_patterns: vec!["Reading ".into(), "Writing ".into(), "Editing ".into()],
            idle_patterns: vec!["\u{256e}\u{2500}".into(), "$ ".into()],
            input_patterns: vec!["[y/n".into(), "Permission".into()],
            error_patterns: vec!["Error:".into(), "FAILED".into()],
        }
    }

    #[test]
    fn test_detect_working() {
        let status = detect_status("│ Reading src/main.rs", &claude_patterns());
        assert!(matches!(status, Some(AgentStatus::Working(_))));
    }

    #[test]
    fn test_detect_idle() {
        let status = detect_status("\u{256e}\u{2500} Done", &claude_patterns());
        assert_eq!(status, Some(AgentStatus::Idle));
    }

    #[test]
    fn test_detect_input() {
        let status = detect_status("Write to schema.sql? [y/n]", &claude_patterns());
        assert!(matches!(status, Some(AgentStatus::NeedsInput(_))));
    }

    #[test]
    fn test_detect_error() {
        let status = detect_status("Error: file not found", &claude_patterns());
        assert!(matches!(status, Some(AgentStatus::Error(_))));
    }

    #[test]
    fn test_error_takes_precedence() {
        let patterns = StatusSection {
            working_patterns: vec!["Reading ".into()],
            idle_patterns: vec![],
            input_patterns: vec![],
            error_patterns: vec!["Error".into()],
        };
        let status = detect_status("Error Reading file", &patterns);
        assert!(matches!(status, Some(AgentStatus::Error(_))));
    }

    #[test]
    fn test_no_match() {
        let status = detect_status("some random output", &claude_patterns());
        assert_eq!(status, None);
    }
}
