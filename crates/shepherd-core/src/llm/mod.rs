pub mod providers;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Role of a chat message participant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A single chat message with a role and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Request to send to an LLM for chat completion.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub model: Option<String>,
}

impl LlmRequest {
    /// Create a new request with default settings.
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            max_tokens: 4096,
            temperature: 0.7,
            model: None,
        }
    }
}

/// Token usage information from an LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Response from an LLM chat completion.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
}

/// Request to generate images.
#[derive(Debug, Clone)]
pub struct ImageGenRequest {
    pub prompt: String,
    pub size: String,
    pub n: u32,
    pub model: Option<String>,
}

impl ImageGenRequest {
    /// Create a new image generation request with defaults.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            size: "1024x1024".to_string(),
            n: 4,
            model: None,
        }
    }
}

/// A single generated image.
#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub data: String,
    pub is_url: bool,
}

/// Response containing generated images.
#[derive(Debug, Clone)]
pub struct ImageGenResponse {
    pub images: Vec<GeneratedImage>,
}

/// Trait for LLM providers.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request.
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse>;

    /// Generate images. Default implementation returns an error.
    async fn generate_image(&self, _request: &ImageGenRequest) -> Result<ImageGenResponse> {
        anyhow::bail!("Image generation is not supported by this provider")
    }

    /// Return the provider name.
    fn name(&self) -> &str;
}

/// Configuration for creating an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
}

/// Create an LLM provider from configuration.
pub fn create_provider(config: ProviderConfig) -> Result<Box<dyn LlmProvider>> {
    match config.provider.as_str() {
        "openai" | "openai-compatible" => {
            let api_key = config
                .api_key
                .ok_or_else(|| anyhow::anyhow!("OpenAI provider requires an API key"))?;
            let base_url = config
                .base_url
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let default_model = config
                .default_model
                .unwrap_or_else(|| "gpt-4o".to_string());
            Ok(Box::new(providers::OpenAiProvider::new(
                api_key,
                base_url,
                default_model,
            )))
        }
        "anthropic" => {
            let api_key = config
                .api_key
                .ok_or_else(|| anyhow::anyhow!("Anthropic provider requires an API key"))?;
            let base_url = config
                .base_url
                .unwrap_or_else(|| "https://api.anthropic.com".to_string());
            let default_model = config
                .default_model
                .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());
            Ok(Box::new(providers::AnthropicProvider::new(
                api_key,
                base_url,
                default_model,
            )))
        }
        "ollama" => {
            let base_url = config
                .base_url
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let default_model = config
                .default_model
                .unwrap_or_else(|| "llama3".to_string());
            Ok(Box::new(providers::OllamaProvider::new(
                base_url,
                default_model,
            )))
        }
        other => anyhow::bail!("Unknown provider: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_constructors() {
        let sys = ChatMessage::system("You are helpful");
        assert_eq!(sys.role, Role::System);
        assert_eq!(sys.content, "You are helpful");

        let usr = ChatMessage::user("Hello");
        assert_eq!(usr.role, Role::User);
        assert_eq!(usr.content, "Hello");

        let asst = ChatMessage::assistant("Hi there");
        assert_eq!(asst.role, Role::Assistant);
        assert_eq!(asst.content, "Hi there");
    }

    #[test]
    fn test_llm_request_defaults() {
        let req = LlmRequest::new(vec![ChatMessage::user("test")]);
        assert_eq!(req.max_tokens, 4096);
        assert!((req.temperature - 0.7).abs() < f32::EPSILON);
        assert!(req.model.is_none());
        assert_eq!(req.messages.len(), 1);
    }

    #[test]
    fn test_create_provider_unknown() {
        let config = ProviderConfig {
            provider: "unknown-provider".to_string(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        let result = create_provider(config);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Unknown provider"));
    }

    #[test]
    fn test_create_provider_openai_missing_key() {
        let config = ProviderConfig {
            provider: "openai".to_string(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        let result = create_provider(config);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("API key"));
    }

    #[test]
    fn test_create_provider_openai_ok() {
        let config = ProviderConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test-key".to_string()),
            base_url: None,
            default_model: None,
        };
        let provider = create_provider(config).unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_create_provider_ollama_no_key_needed() {
        let config = ProviderConfig {
            provider: "ollama".to_string(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        let provider = create_provider(config).unwrap();
        assert_eq!(provider.name(), "ollama");
    }
}
