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
}
