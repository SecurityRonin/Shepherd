use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{
    ChatMessage, GeneratedImage, ImageGenRequest, ImageGenResponse, LlmProvider, LlmRequest,
    LlmResponse, Role, TokenUsage,
};

// ─── OpenAI ──────────────────────────────────────────────────────────

/// OpenAI-compatible provider (works with OpenAI API and compatible endpoints).
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: String, default_model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            default_model,
        }
    }
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    model: String,
    usage: OpenAiUsage,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Serialize)]
struct OpenAiImageRequest {
    model: String,
    prompt: String,
    size: String,
    n: u32,
}

#[derive(Deserialize)]
struct OpenAiImageResponse {
    data: Vec<OpenAiImageData>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    url: Option<String>,
    b64_json: Option<String>,
}

fn role_to_string(role: &Role) -> String {
    match role {
        Role::System => "system".to_string(),
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
    }
}

fn chat_messages_to_openai(messages: &[ChatMessage]) -> Vec<OpenAiMessage> {
    messages
        .iter()
        .map(|m| OpenAiMessage {
            role: role_to_string(&m.role),
            content: m.content.clone(),
        })
        .collect()
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let body = OpenAiChatRequest {
            model,
            messages: chat_messages_to_openai(&request.messages),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<OpenAiChatResponse>()
            .await?;

        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in OpenAI response"))?;

        Ok(LlmResponse {
            content: choice.message.content,
            model: resp.model,
            usage: TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
        })
    }

    async fn generate_image(&self, request: &ImageGenRequest) -> Result<ImageGenResponse> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| "dall-e-3".to_string());

        let body = OpenAiImageRequest {
            model,
            prompt: request.prompt.clone(),
            size: request.size.clone(),
            n: request.n,
        };

        let resp = self
            .client
            .post(format!("{}/images/generations", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<OpenAiImageResponse>()
            .await?;

        let images = resp
            .data
            .into_iter()
            .map(|d| {
                if let Some(url) = d.url {
                    GeneratedImage {
                        data: url,
                        is_url: true,
                    }
                } else if let Some(b64) = d.b64_json {
                    GeneratedImage {
                        data: b64,
                        is_url: false,
                    }
                } else {
                    GeneratedImage {
                        data: String::new(),
                        is_url: false,
                    }
                }
            })
            .collect();

        Ok(ImageGenResponse { images })
    }

    fn name(&self) -> &str {
        "openai"
    }
}

// ─── Anthropic ───────────────────────────────────────────────────────

/// Anthropic Claude provider.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: String, default_model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            default_model,
        }
    }
}

#[derive(Serialize)]
struct AnthropicChatRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicChatResponse {
    content: Vec<AnthropicContent>,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        // Extract system message separately for Anthropic API
        let system_msg: Option<String> = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| AnthropicMessage {
                role: role_to_string(&m.role),
                content: m.content.clone(),
            })
            .collect();

        let body = AnthropicChatRequest {
            model,
            max_tokens: request.max_tokens,
            system: system_msg,
            messages,
            temperature: Some(request.temperature),
        };

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<AnthropicChatResponse>()
            .await?;

        let content = resp
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            content,
            model: resp.model,
            usage: TokenUsage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            },
        })
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}

// ─── Ollama ──────────────────────────────────────────────────────────

/// Ollama local LLM provider.
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    default_model: String,
}

impl OllamaProvider {
    pub fn new(base_url: String, default_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            default_model,
        }
    }
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    model: String,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let messages: Vec<OllamaMessage> = request
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: role_to_string(&m.role),
                content: m.content.clone(),
            })
            .collect();

        let body = OllamaChatRequest {
            model,
            messages,
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<OllamaChatResponse>()
            .await?;

        let prompt_tokens = resp.prompt_eval_count.unwrap_or(0);
        let completion_tokens = resp.eval_count.unwrap_or(0);

        Ok(LlmResponse {
            content: resp.message.content,
            model: resp.model,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        })
    }

    fn name(&self) -> &str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAiProvider::new(
            "sk-test".to_string(),
            "https://api.openai.com/v1".to_string(),
            "gpt-4o".to_string(),
        );
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.api_key, "sk-test");
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
        assert_eq!(provider.default_model, "gpt-4o");
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new(
            "sk-ant-test".to_string(),
            "https://api.anthropic.com".to_string(),
            "claude-sonnet-4-20250514".to_string(),
        );
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.api_key, "sk-ant-test");
        assert_eq!(provider.base_url, "https://api.anthropic.com");
        assert_eq!(provider.default_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "llama3".to_string(),
        );
        assert_eq!(provider.name(), "ollama");
        assert_eq!(provider.base_url, "http://localhost:11434");
        assert_eq!(provider.default_model, "llama3");
    }

    #[test]
    fn test_role_to_string() {
        assert_eq!(role_to_string(&Role::System), "system");
        assert_eq!(role_to_string(&Role::User), "user");
        assert_eq!(role_to_string(&Role::Assistant), "assistant");
    }

    #[test]
    fn test_chat_messages_to_openai() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there"),
        ];
        let openai_msgs = chat_messages_to_openai(&messages);
        assert_eq!(openai_msgs.len(), 3);
        assert_eq!(openai_msgs[0].role, "system");
        assert_eq!(openai_msgs[0].content, "You are helpful");
        assert_eq!(openai_msgs[1].role, "user");
        assert_eq!(openai_msgs[1].content, "Hello");
        assert_eq!(openai_msgs[2].role, "assistant");
        assert_eq!(openai_msgs[2].content, "Hi there");
    }

    #[test]
    fn test_chat_messages_to_openai_empty() {
        let messages: Vec<ChatMessage> = vec![];
        let openai_msgs = chat_messages_to_openai(&messages);
        assert!(openai_msgs.is_empty());
    }

    #[test]
    fn test_openai_provider_custom_config() {
        let provider = OpenAiProvider::new(
            "sk-custom".to_string(),
            "https://custom.api.com/v1".to_string(),
            "gpt-4o-mini".to_string(),
        );
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.api_key, "sk-custom");
        assert_eq!(provider.base_url, "https://custom.api.com/v1");
        assert_eq!(provider.default_model, "gpt-4o-mini");
    }

    #[test]
    fn test_anthropic_provider_custom_config() {
        let provider = AnthropicProvider::new(
            "sk-ant-custom".to_string(),
            "https://custom.anthropic.com".to_string(),
            "claude-opus-4".to_string(),
        );
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.api_key, "sk-ant-custom");
        assert_eq!(provider.base_url, "https://custom.anthropic.com");
        assert_eq!(provider.default_model, "claude-opus-4");
    }

    #[test]
    fn test_openai_message_serde() {
        let msg = OpenAiMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: OpenAiMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello");
    }

    #[test]
    fn test_openai_chat_request_serializes() {
        let req = OpenAiChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 1024,
            temperature: 0.7,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4o"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_openai_chat_response_deserializes() {
        let json = r#"{
            "choices": [{"message": {"role": "assistant", "content": "Hi there"}}],
            "model": "gpt-4o",
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let resp: OpenAiChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "gpt-4o");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hi there");
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn test_openai_image_response_deserializes() {
        let json = r#"{
            "data": [
                {"url": "https://example.com/img.png", "b64_json": null},
                {"url": null, "b64_json": "iVBOR..."},
                {"url": null, "b64_json": null}
            ]
        }"#;
        let resp: OpenAiImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 3);
        assert_eq!(resp.data[0].url.as_deref(), Some("https://example.com/img.png"));
        assert!(resp.data[0].b64_json.is_none());
        assert!(resp.data[1].url.is_none());
        assert_eq!(resp.data[1].b64_json.as_deref(), Some("iVBOR..."));
    }

    #[test]
    fn test_anthropic_message_serde() {
        let msg = AnthropicMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: AnthropicMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello");
    }

    #[test]
    fn test_anthropic_chat_response_deserializes() {
        let json = r#"{
            "content": [{"text": "Hello "}, {"text": "world"}],
            "model": "claude-sonnet-4-20250514",
            "usage": {"input_tokens": 20, "output_tokens": 10}
        }"#;
        let resp: AnthropicChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "claude-sonnet-4-20250514");
        assert_eq!(resp.content.len(), 2);
        assert_eq!(resp.content[0].text, "Hello ");
        assert_eq!(resp.content[1].text, "world");
        assert_eq!(resp.usage.input_tokens, 20);
        assert_eq!(resp.usage.output_tokens, 10);
    }

    #[test]
    fn test_ollama_chat_response_deserializes() {
        let json = r#"{
            "message": {"role": "assistant", "content": "Hi"},
            "model": "llama3",
            "prompt_eval_count": 50,
            "eval_count": 25
        }"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "llama3");
        assert_eq!(resp.message.content, "Hi");
        assert_eq!(resp.prompt_eval_count, Some(50));
        assert_eq!(resp.eval_count, Some(25));
    }

    #[test]
    fn test_ollama_chat_response_missing_counts() {
        let json = r#"{
            "message": {"role": "assistant", "content": "Hi"},
            "model": "llama3"
        }"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.prompt_eval_count, None);
        assert_eq!(resp.eval_count, None);
    }

    #[test]
    fn test_openai_image_request_serializes() {
        let req = OpenAiImageRequest {
            model: "dall-e-3".to_string(),
            prompt: "A blue cat".to_string(),
            size: "1024x1024".to_string(),
            n: 2,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("dall-e-3"));
        assert!(json.contains("A blue cat"));
        assert!(json.contains("1024x1024"));
    }

    #[test]
    fn test_anthropic_chat_request_serializes_with_system() {
        let req = AnthropicChatRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            system: Some("You are helpful".to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("system"));
        assert!(json.contains("You are helpful"));
    }

    #[test]
    fn test_anthropic_chat_request_serializes_without_system() {
        let req = AnthropicChatRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            system: None,
            messages: vec![],
            temperature: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        // system and temperature should be skipped when None
        assert!(!json.contains("\"system\""));
        assert!(!json.contains("\"temperature\""));
    }

    #[test]
    fn test_ollama_options_serializes() {
        let opts = OllamaOptions {
            temperature: 0.5,
            num_predict: 2048,
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("0.5"));
        assert!(json.contains("2048"));
    }

    // ── httpmock-based async tests ────────────────────────────────────────

    use httpmock::prelude::*;
    use crate::llm::{ChatMessage, LlmRequest, Role, ImageGenRequest};

    fn basic_llm_request() -> LlmRequest {
        LlmRequest {
            messages: vec![ChatMessage::user("Hello")],
            model: None,
            max_tokens: 256,
            temperature: 0.7,
        }
    }

    // ── OpenAiProvider::chat ───────────────────────────────────────────────

    #[tokio::test]
    async fn openai_chat_200_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "choices": [{"message": {"role": "assistant", "content": "Hello there!"}}],
                    "model": "gpt-4o",
                    "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
                }));
        });
        let provider = OpenAiProvider::new(
            "sk-test".to_string(),
            server.base_url(),
            "gpt-4o".to_string(),
        );
        let result = provider.chat(&basic_llm_request()).await.unwrap();
        assert_eq!(result.content, "Hello there!");
        assert_eq!(result.model, "gpt-4o");
        assert_eq!(result.usage.prompt_tokens, 10);
        assert_eq!(result.usage.completion_tokens, 5);
        assert_eq!(result.usage.total_tokens, 15);
    }

    #[tokio::test]
    async fn openai_chat_500_returns_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(500).body("Internal Server Error");
        });
        let provider = OpenAiProvider::new(
            "sk-test".to_string(),
            server.base_url(),
            "gpt-4o".to_string(),
        );
        let result = provider.chat(&basic_llm_request()).await;
        assert!(result.is_err(), "expected error on 500 response");
    }

    // ── OpenAiProvider::generate_image ────────────────────────────────────

    #[tokio::test]
    async fn openai_generate_image_url_variant() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/images/generations");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "data": [{"url": "https://cdn.openai.com/img.png", "b64_json": null}]
                }));
        });
        let provider = OpenAiProvider::new(
            "sk-test".to_string(),
            server.base_url(),
            "dall-e-3".to_string(),
        );
        let req = ImageGenRequest {
            prompt: "A blue cat".to_string(),
            model: None,
            size: "1024x1024".to_string(),
            n: 1,
        };
        let result = provider.generate_image(&req).await.unwrap();
        assert_eq!(result.images.len(), 1);
        assert!(result.images[0].is_url);
        assert_eq!(result.images[0].data, "https://cdn.openai.com/img.png");
    }

    #[tokio::test]
    async fn openai_generate_image_b64_variant() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/images/generations");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "data": [{"url": null, "b64_json": "iVBORw0KGgo="}]
                }));
        });
        let provider = OpenAiProvider::new(
            "sk-test".to_string(),
            server.base_url(),
            "dall-e-3".to_string(),
        );
        let req = ImageGenRequest {
            prompt: "A red dog".to_string(),
            model: None,
            size: "512x512".to_string(),
            n: 1,
        };
        let result = provider.generate_image(&req).await.unwrap();
        assert_eq!(result.images.len(), 1);
        assert!(!result.images[0].is_url);
        assert_eq!(result.images[0].data, "iVBORw0KGgo=");
    }

    // ── AnthropicProvider::chat ────────────────────────────────────────────

    #[tokio::test]
    async fn anthropic_chat_200_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "content": [{"text": "Hello "}, {"text": "world"}],
                    "model": "claude-sonnet-4-20250514",
                    "usage": {"input_tokens": 20, "output_tokens": 10}
                }));
        });
        let provider = AnthropicProvider::new(
            "sk-ant-test".to_string(),
            server.base_url(),
            "claude-sonnet-4-20250514".to_string(),
        );
        // Include a system message to verify it is filtered out of messages[]
        let req = LlmRequest {
            messages: vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
            model: None,
            max_tokens: 1024,
            temperature: 0.5,
        };
        let result = provider.chat(&req).await.unwrap();
        // Multi-content blocks should be joined
        assert_eq!(result.content, "Hello world");
        assert_eq!(result.model, "claude-sonnet-4-20250514");
        assert_eq!(result.usage.prompt_tokens, 20);
        assert_eq!(result.usage.completion_tokens, 10);
        assert_eq!(result.usage.total_tokens, 30);
    }

    #[tokio::test]
    async fn anthropic_chat_multi_content_joined() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "content": [{"text": "Part1"}, {"text": "Part2"}, {"text": "Part3"}],
                    "model": "claude-opus-4",
                    "usage": {"input_tokens": 5, "output_tokens": 15}
                }));
        });
        let provider = AnthropicProvider::new(
            "sk-ant-test".to_string(),
            server.base_url(),
            "claude-opus-4".to_string(),
        );
        let result = provider.chat(&basic_llm_request()).await.unwrap();
        assert_eq!(result.content, "Part1Part2Part3");
        assert_eq!(result.usage.total_tokens, 20);
    }

    // ── OllamaProvider::chat ───────────────────────────────────────────────

    #[tokio::test]
    async fn ollama_chat_200_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "message": {"role": "assistant", "content": "Hi from Ollama!"},
                    "model": "llama3",
                    "prompt_eval_count": 30,
                    "eval_count": 12
                }));
        });
        let provider = OllamaProvider::new(server.base_url(), "llama3".to_string());
        let result = provider.chat(&basic_llm_request()).await.unwrap();
        assert_eq!(result.content, "Hi from Ollama!");
        assert_eq!(result.model, "llama3");
        assert_eq!(result.usage.prompt_tokens, 30);
        assert_eq!(result.usage.completion_tokens, 12);
        assert_eq!(result.usage.total_tokens, 42);
    }

    #[tokio::test]
    async fn ollama_chat_missing_token_counts() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "message": {"role": "assistant", "content": "No counts"},
                    "model": "llama3"
                }));
        });
        let provider = OllamaProvider::new(server.base_url(), "llama3".to_string());
        let result = provider.chat(&basic_llm_request()).await.unwrap();
        assert_eq!(result.content, "No counts");
        // Missing counts default to 0
        assert_eq!(result.usage.prompt_tokens, 0);
        assert_eq!(result.usage.completion_tokens, 0);
        assert_eq!(result.usage.total_tokens, 0);
    }
}
