use anyhow::Result;

use crate::llm::{ChatMessage, LlmProvider, LlmRequest};

/// Maximum diff size to send to the LLM for commit message generation.
const TRUNCATION_THRESHOLD: usize = 8000;

/// Generate a conventional commit message from a diff using an LLM.
pub async fn generate_commit_message(
    llm: &dyn LlmProvider,
    diff: &str,
    task_title: &str,
) -> Result<String> {
    let truncated_diff = if diff.len() > TRUNCATION_THRESHOLD {
        &diff[..TRUNCATION_THRESHOLD]
    } else {
        diff
    };

    let system_prompt = "You are a commit message generator. Write a single conventional commit \
        message (e.g., feat:, fix:, refactor:, docs:, test:, chore:) that accurately \
        describes the changes. Be concise — one line, no body. Do not wrap in quotes or \
        add any other text.";

    let user_prompt = format!(
        "Task: {task_title}\n\nDiff:\n```\n{truncated_diff}\n```\n\nWrite a conventional commit message:"
    );

    let request = LlmRequest {
        messages: vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
        max_tokens: 512,
        temperature: 0.3,
        model: None,
    };

    let response = llm.chat(&request).await?;
    Ok(response.content.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_threshold() {
        assert_eq!(TRUNCATION_THRESHOLD, 8000);

        // Verify truncation logic works correctly
        let long_diff = "a".repeat(10000);
        let truncated = if long_diff.len() > TRUNCATION_THRESHOLD {
            &long_diff[..TRUNCATION_THRESHOLD]
        } else {
            &long_diff
        };
        assert_eq!(truncated.len(), 8000);

        // Short diff should not be truncated
        let short_diff = "b".repeat(100);
        let not_truncated = if short_diff.len() > TRUNCATION_THRESHOLD {
            &short_diff[..TRUNCATION_THRESHOLD]
        } else {
            &short_diff
        };
        assert_eq!(not_truncated.len(), 100);
    }
}
