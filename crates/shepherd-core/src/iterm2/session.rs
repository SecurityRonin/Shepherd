// crates/shepherd-core/src/iterm2/session.rs
use crate::iterm2::client::iterm2;

/// Flatten a GetBufferResponse content list to plain text.
/// HardEol (or None) lines get a '\n'; SoftEol (soft-wrap) lines are joined without separator.
pub fn flatten_buffer(lines: &[iterm2::LineContents]) -> String {
    let mut out = String::new();
    for line in lines {
        if let Some(ref text) = line.text {
            out.push_str(text);
        }
        let is_soft = line.continuation
            .map(|c| c == iterm2::line_contents::Continuation::SoftEol as i32)
            .unwrap_or(false);
        if !is_soft {
            out.push('\n');
        }
    }
    out
}

/// Detect a Claude Code permission prompt in terminal output.
/// Claude Code prompts look like: "Allow <tool>?\n(y/n)"
/// Returns the extracted tool description if found.
pub fn detect_permission_prompt(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    if lower.contains("(y/n)") {
        if let Some(allow_pos) = lower.rfind("allow ") {
            let fragment = &text[allow_pos..];
            if let Some(q_pos) = fragment.find('?') {
                let tool_desc = fragment[6..q_pos].trim().to_string();
                return Some(tool_desc);
            }
        }
    }
    None
}

/// An adopted iTerm2 session actively managed by Shepherd.
pub struct AdoptedSession {
    pub task_id: i64,
    pub iterm2_session_id: String,
    pub cwd: String,
}

impl AdoptedSession {
    pub fn new(task_id: i64, iterm2_session_id: String, cwd: String) -> Self {
        Self { task_id, iterm2_session_id, cwd }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterm2::client::iterm2;

    #[test]
    fn test_adopted_session_new_stores_fields() {
        let s = AdoptedSession::new(99, "iterm2-sess-xyz".to_string(), "/home/user/proj".to_string());
        assert_eq!(s.task_id, 99);
        assert_eq!(s.iterm2_session_id, "iterm2-sess-xyz");
        assert_eq!(s.cwd, "/home/user/proj");
    }

    fn make_line(text: &str, hard_eol: bool) -> iterm2::LineContents {
        iterm2::LineContents {
            text: Some(text.to_string()),
            continuation: Some(if hard_eol {
                iterm2::line_contents::Continuation::HardEol as i32
            } else {
                iterm2::line_contents::Continuation::SoftEol as i32
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_flatten_hard_eol_appends_newline() {
        let lines = vec![
            make_line("hello", true),
            make_line("world", true),
        ];
        assert_eq!(flatten_buffer(&lines), "hello\nworld\n");
    }

    #[test]
    fn test_flatten_soft_wrap_no_newline() {
        let lines = vec![
            make_line("abc", false), // soft-wrap: no newline
            make_line("def", true),  // hard eol: newline
        ];
        assert_eq!(flatten_buffer(&lines), "abcdef\n");
    }

    #[test]
    fn test_flatten_empty_buffer() {
        assert_eq!(flatten_buffer(&[]), "");
    }

    #[test]
    fn test_flatten_default_continuation_is_hard_eol() {
        // When continuation is None, default is HardEol
        let line = iterm2::LineContents {
            text: Some("hi".to_string()),
            continuation: None,
            ..Default::default()
        };
        assert_eq!(flatten_buffer(&[line]), "hi\n");
    }

    #[test]
    fn test_detect_permission_prompt_bash_tool() {
        let text = "Allow bash tool?\n(y/n) [y]: ";
        assert!(detect_permission_prompt(text).is_some());
    }

    #[test]
    fn test_detect_permission_prompt_write_tool() {
        let text = "Allow write to file?\n(y/n): ";
        assert!(detect_permission_prompt(text).is_some());
    }

    #[test]
    fn test_detect_permission_prompt_no_match() {
        let text = "Normal terminal output with no prompt";
        assert!(detect_permission_prompt(text).is_none());
    }

    #[test]
    fn test_detect_permission_prompt_case_insensitive() {
        let text = "ALLOW Edit Tool?\n(Y/N) [y]: ";
        let tool = detect_permission_prompt(text);
        assert!(tool.is_some());
    }

    #[test]
    fn test_detect_permission_prompt_yn_without_allow() {
        // Has (y/n) but no "allow" keyword
        let text = "Continue?\n(y/n): ";
        assert!(detect_permission_prompt(text).is_none());
    }

    #[test]
    fn test_detect_permission_prompt_allow_without_yn() {
        // Has "allow" but no (y/n)
        let text = "Allow bash tool? Press enter to continue.";
        assert!(detect_permission_prompt(text).is_none());
    }

    #[test]
    fn test_flatten_buffer_no_text() {
        // Line with no text should still add newline for hard eol
        let line = iterm2::LineContents {
            text: None,
            continuation: Some(iterm2::line_contents::Continuation::HardEol as i32),
            ..Default::default()
        };
        assert_eq!(flatten_buffer(&[line]), "\n");
    }

    #[test]
    fn test_detect_permission_prompt_extracts_tool_name() {
        let text = "Allow bash tool?\n(y/n) [y]: ";
        let tool = detect_permission_prompt(text).unwrap();
        assert!(tool.contains("bash"), "tool name should contain 'bash', got: {tool}");
    }
}
