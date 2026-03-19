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
            let default_model = config.default_model.unwrap_or_else(|| "gpt-4o".to_string());
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
            let default_model = config.default_model.unwrap_or_else(|| "llama3".to_string());
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

    #[test]
    fn test_create_provider_anthropic_missing_key() {
        let config = ProviderConfig {
            provider: "anthropic".to_string(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        let result = create_provider(config);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("API key"));
    }

    #[test]
    fn test_create_provider_anthropic_ok() {
        let config = ProviderConfig {
            provider: "anthropic".to_string(),
            api_key: Some("sk-ant-test".to_string()),
            base_url: None,
            default_model: None,
        };
        let provider = create_provider(config).unwrap();
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_create_provider_openai_compatible() {
        let config = ProviderConfig {
            provider: "openai-compatible".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: Some("https://custom.api.com/v1".to_string()),
            default_model: Some("custom-model".to_string()),
        };
        let provider = create_provider(config).unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_create_provider_openai_with_defaults() {
        let config = ProviderConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: None,
            default_model: None,
        };
        let provider = create_provider(config).unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_create_provider_ollama_with_custom_url() {
        let config = ProviderConfig {
            provider: "ollama".to_string(),
            api_key: None,
            base_url: Some("http://remote:11434".to_string()),
            default_model: Some("mistral".to_string()),
        };
        let provider = create_provider(config).unwrap();
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_image_gen_request_defaults() {
        let req = ImageGenRequest::new("A cute cat");
        assert_eq!(req.prompt, "A cute cat");
        assert_eq!(req.size, "1024x1024");
        assert_eq!(req.n, 4);
        assert!(req.model.is_none());
    }

    #[test]
    fn test_role_serde() {
        let json = serde_json::to_string(&Role::System).unwrap();
        assert_eq!(json, "\"system\"");
        let json = serde_json::to_string(&Role::User).unwrap();
        assert_eq!(json, "\"user\"");
        let json = serde_json::to_string(&Role::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");

        let parsed: Role = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(parsed, Role::System);
    }

    #[test]
    fn test_chat_message_serde_roundtrip() {
        let msg = ChatMessage::user("Hello world");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::User);
        assert_eq!(parsed.content, "Hello world");
    }

    #[test]
    fn test_token_usage_serde() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let parsed: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.prompt_tokens, 100);
        assert_eq!(parsed.completion_tokens, 50);
        assert_eq!(parsed.total_tokens, 150);
    }

    #[test]
    fn test_provider_config_serde() {
        let config = ProviderConfig {
            provider: "openai".to_string(),
            api_key: Some("key".to_string()),
            base_url: None,
            default_model: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider, "openai");
        assert_eq!(parsed.api_key, Some("key".to_string()));
        assert!(parsed.base_url.is_none());
    }

    #[tokio::test]
    async fn test_default_generate_image_returns_error() {
        struct ChatOnlyProvider;

        #[async_trait::async_trait]
        impl LlmProvider for ChatOnlyProvider {
            async fn chat(&self, _request: &LlmRequest) -> Result<LlmResponse> {
                Ok(LlmResponse {
                    content: "hi".to_string(),
                    model: "test".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                })
            }
            fn name(&self) -> &str {
                "chat-only"
            }
        }

        let provider = ChatOnlyProvider;
        let req = ImageGenRequest::new("test");
        let result = provider.generate_image(&req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }

    #[test]
    fn test_generated_image_fields() {
        let img_url = GeneratedImage {
            data: "https://example.com/img.png".to_string(),
            is_url: true,
        };
        assert!(img_url.is_url);

        let img_b64 = GeneratedImage {
            data: "iVBOR...".to_string(),
            is_url: false,
        };
        assert!(!img_b64.is_url);
    }

    #[test]
    fn test_llm_request_custom_fields() {
        let mut req = LlmRequest::new(vec![ChatMessage::system("sys"), ChatMessage::user("usr")]);
        req.max_tokens = 8192;
        req.temperature = 0.5;
        req.model = Some("gpt-4o-mini".to_string());
        assert_eq!(req.max_tokens, 8192);
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.model.as_deref(), Some("gpt-4o-mini"));
    }
}
