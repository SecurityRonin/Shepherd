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

    #[tokio::test]
    async fn generate_commit_message_with_mock() {
        use crate::llm::{LlmResponse, TokenUsage};

        struct MockProvider;

        #[async_trait::async_trait]
        impl LlmProvider for MockProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                // Verify the request structure
                assert_eq!(request.messages.len(), 2);
                assert!(request.messages[0].content.contains("commit message"));
                assert!(request.messages[1].content.contains("Fix auth"));
                Ok(LlmResponse {
                    content: "  fix: resolve authentication race condition  ".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 100,
                        completion_tokens: 10,
                        total_tokens: 110,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        let provider = MockProvider;
        let msg = generate_commit_message(&provider, "diff --git a/auth.rs", "Fix auth")
            .await
            .unwrap();
        assert_eq!(msg, "fix: resolve authentication race condition");
    }

    #[tokio::test]
    async fn generate_commit_message_truncates_long_diff() {
        use crate::llm::{LlmResponse, TokenUsage};

        struct DiffCheckProvider;

        #[async_trait::async_trait]
        impl LlmProvider for DiffCheckProvider {
            async fn chat(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                // The user message contains the diff, it should be truncated
                let user_msg = &request.messages[1].content;
                assert!(user_msg.len() < 10000, "Diff should be truncated");
                Ok(LlmResponse {
                    content: "feat: add feature".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        let long_diff = "x".repeat(10000);
        let msg = generate_commit_message(&DiffCheckProvider, &long_diff, "Task")
            .await
            .unwrap();
        assert_eq!(msg, "feat: add feature");
    }

    #[tokio::test]
    async fn generate_commit_message_llm_error() {
        use crate::llm::LlmResponse;

        struct FailProvider;

        #[async_trait::async_trait]
        impl LlmProvider for FailProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                anyhow::bail!("API rate limited")
            }
            fn name(&self) -> &str {
                "fail"
            }
        }

        let result = generate_commit_message(&FailProvider, "diff", "Task").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate limited"));
    }
}
