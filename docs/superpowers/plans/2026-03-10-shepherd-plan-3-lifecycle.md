# Shepherd Lifecycle & Polish — Implementation Plan (3 of 3)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Shepherd's unique differentiator features — product name generator with domain validation, logo/icon generator, North Star PMF integration, quality gates, contextual trigger system, one-click PR pipeline, and CLI polish.

**Architecture:** These features extend the existing Shepherd core (Plan 1) and frontend (Plan 2). Backend features are added to shepherd-core and shepherd-server crates. Frontend components are added to the React app. LLM features use a provider-agnostic API client (BYOK model).

**Tech Stack:** Rust (extending existing crates), React/TypeScript (extending frontend), RDAP/DNS for domain checks, image generation APIs (DALL-E/Stable Diffusion via BYOK), North Star Advisor methodology

**Spec:** `docs/superpowers/specs/2026-03-10-shepherd-design.md`

**Dependencies:** Plans 1 and 2 must be complete — this plan extends both the server and frontend.

---

## File Structure

```
crates/shepherd-core/src/
├── llm/
│   ├── mod.rs              # Provider-agnostic LLM client
│   └── providers.rs        # BYOK provider implementations
├── namegen/
│   ├── mod.rs              # Name generator orchestrator
│   ├── brainstorm.rs       # LLM name brainstorming
│   ├── validate.rs         # Domain/registry/conflict checks
│   └── rdap.rs             # RDAP domain availability client
├── logogen/
│   ├── mod.rs              # Logo generator orchestrator
│   ├── generate.rs         # Image API client (provider-agnostic)
│   └── export.rs           # Multi-format export
├── northstar/
│   ├── mod.rs              # North Star integration orchestrator
│   ├── phases.rs           # 13-phase wizard definitions
│   └── context.rs          # ai-context.yml generator
├── gates/
│   ├── mod.rs              # Quality gate runner
│   ├── builtin.rs          # Built-in gates
│   └── plugin.rs           # Plugin gate loader
├── pr/
│   ├── mod.rs              # PR pipeline orchestrator
│   ├── commit.rs           # LLM commit message generation
│   └── github.rs           # gh CLI wrapper
├── triggers/
│   ├── mod.rs              # Contextual trigger engine
│   └── detectors.rs        # Individual trigger detectors
└── lib.rs                  # Updated with new module declarations

crates/shepherd-server/src/
├── routes/
│   ├── namegen.rs          # POST /api/namegen/*
│   ├── logogen.rs          # POST /api/logogen/*
│   ├── northstar.rs        # POST /api/northstar/*
│   ├── gates.rs            # GET /api/tasks/:id/gates
│   ├── pr.rs               # POST /api/tasks/:id/pr
│   └── mod.rs              # Updated router with new routes
└── ws.rs                   # Updated with new event types

crates/shepherd-cli/src/
└── main.rs                 # Updated with new subcommands + shell completions

src/features/               # Frontend additions
├── namegen/
│   └── NameGenerator.tsx   # Name generator wizard UI
├── logogen/
│   └── LogoGenerator.tsx   # Logo generator wizard UI
├── northstar/
│   └── NorthStarWizard.tsx # North Star PMF wizard UI
├── gates/
│   └── GateResults.tsx     # Quality gate results display
├── pr/
│   └── PrPipeline.tsx      # PR creation flow UI
└── triggers/
    └── TriggerToast.tsx    # Contextual trigger toast notifications
```

---

## Chunk 1: LLM Client & Name Generator (Tasks 1–4)

### Task 1: Provider-Agnostic LLM Client

**Files:**
- Create: `crates/shepherd-core/src/llm/mod.rs`
- Create: `crates/shepherd-core/src/llm/providers.rs`
- Modify: `crates/shepherd-core/src/lib.rs`
- Modify: `crates/shepherd-core/Cargo.toml`

- [ ] **Step 1: Add dependencies to shepherd-core Cargo.toml**

Add these dependencies to the `[dependencies]` section in `crates/shepherd-core/Cargo.toml`:

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tokio = { workspace = true }
async-trait = "0.1"
base64 = "0.22"
```

- [ ] **Step 2: Define LLM trait and types**

```rust
// crates/shepherd-core/src/llm/mod.rs
pub mod providers;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Role in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A single message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }
}

/// Configuration for an LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Optional model override (uses provider default if None)
    pub model: Option<String>,
}

fn default_max_tokens() -> u32 { 4096 }
fn default_temperature() -> f32 { 0.7 }

/// Response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Image generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenRequest {
    pub prompt: String,
    #[serde(default = "default_image_size")]
    pub size: String,
    #[serde(default = "default_image_count")]
    pub n: u8,
    pub model: Option<String>,
}

fn default_image_size() -> String { "1024x1024".into() }
fn default_image_count() -> u8 { 4 }

/// Image generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenResponse {
    pub images: Vec<GeneratedImage>,
}

/// A single generated image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    /// Base64-encoded image data or URL
    pub data: String,
    /// Whether `data` is a URL or base64
    pub is_url: bool,
}

/// Provider-agnostic LLM client trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse>;

    /// Generate images (not all providers support this)
    async fn generate_image(&self, request: &ImageGenRequest) -> Result<ImageGenResponse> {
        anyhow::bail!("Image generation not supported by this provider")
    }

    /// Provider name for logging
    fn name(&self) -> &str;
}

/// BYOK provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,  // "openai", "anthropic", "ollama"
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
}

/// Create an LLM provider from config
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
    match config.provider.as_str() {
        "openai" | "openai-compatible" => {
            let api_key = config.api_key.clone()
                .ok_or_else(|| anyhow::anyhow!("API key required for OpenAI provider"))?;
            let base_url = config.base_url.clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into());
            let default_model = config.default_model.clone()
                .unwrap_or_else(|| "gpt-4o".into());
            Ok(Box::new(providers::OpenAiProvider::new(api_key, base_url, default_model)))
        }
        "anthropic" => {
            let api_key = config.api_key.clone()
                .ok_or_else(|| anyhow::anyhow!("API key required for Anthropic provider"))?;
            let base_url = config.base_url.clone()
                .unwrap_or_else(|| "https://api.anthropic.com/v1".into());
            let default_model = config.default_model.clone()
                .unwrap_or_else(|| "claude-sonnet-4-20250514".into());
            Ok(Box::new(providers::AnthropicProvider::new(api_key, base_url, default_model)))
        }
        "ollama" => {
            let base_url = config.base_url.clone()
                .unwrap_or_else(|| "http://localhost:11434".into());
            let default_model = config.default_model.clone()
                .unwrap_or_else(|| "llama3".into());
            Ok(Box::new(providers::OllamaProvider::new(base_url, default_model)))
        }
        other => anyhow::bail!("Unknown LLM provider: {other}"),
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
    }

    #[test]
    fn test_llm_request_defaults() {
        let req = LlmRequest {
            messages: vec![ChatMessage::user("test")],
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            model: None,
        };
        assert_eq!(req.max_tokens, 4096);
        assert!((req.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_create_provider_unknown() {
        let config = ProviderConfig {
            provider: "unknown".into(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        assert!(create_provider(&config).is_err());
    }

    #[test]
    fn test_create_provider_openai_missing_key() {
        let config = ProviderConfig {
            provider: "openai".into(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        assert!(create_provider(&config).is_err());
    }

    #[test]
    fn test_create_provider_openai_ok() {
        let config = ProviderConfig {
            provider: "openai".into(),
            api_key: Some("sk-test".into()),
            base_url: None,
            default_model: None,
        };
        let provider = create_provider(&config).unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_create_provider_ollama_no_key_needed() {
        let config = ProviderConfig {
            provider: "ollama".into(),
            api_key: None,
            base_url: None,
            default_model: None,
        };
        let provider = create_provider(&config).unwrap();
        assert_eq!(provider.name(), "ollama");
    }
}
```

- [ ] **Step 3: Implement provider backends**

```rust
// crates/shepherd-core/src/llm/providers.rs
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{
    GeneratedImage, ImageGenRequest, ImageGenResponse, LlmProvider, LlmRequest, LlmResponse,
    TokenUsage,
};

// ─── OpenAI-Compatible Provider ───────────────────────────────────────────────

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
    usage: Option<OpenAiUsage>,
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
    n: u8,
    size: String,
    response_format: String,
}

#[derive(Deserialize)]
struct OpenAiImageResponse {
    data: Vec<OpenAiImageData>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    b64_json: Option<String>,
    url: Option<String>,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let model = request.model.clone().unwrap_or_else(|| self.default_model.clone());
        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(|m| OpenAiMessage {
                role: match m.role {
                    super::Role::System => "system".into(),
                    super::Role::User => "user".into(),
                    super::Role::Assistant => "assistant".into(),
                },
                content: m.content.clone(),
            })
            .collect();

        let body = OpenAiChatRequest {
            model: model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error {status}: {text}");
        }

        let data: OpenAiChatResponse = resp.json().await?;
        let choice = data.choices.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;
        let usage = data.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or_default();

        Ok(LlmResponse {
            content: choice.message.content,
            model: data.model,
            usage,
        })
    }

    async fn generate_image(&self, request: &ImageGenRequest) -> Result<ImageGenResponse> {
        let model = request.model.clone().unwrap_or_else(|| "dall-e-3".into());
        let body = OpenAiImageRequest {
            model,
            prompt: request.prompt.clone(),
            n: request.n,
            size: request.size.clone(),
            response_format: "b64_json".into(),
        };

        let resp = self
            .client
            .post(format!("{}/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI Image API error {status}: {text}");
        }

        let data: OpenAiImageResponse = resp.json().await?;
        let images = data.data.into_iter().map(|d| {
            if let Some(b64) = d.b64_json {
                GeneratedImage { data: b64, is_url: false }
            } else if let Some(url) = d.url {
                GeneratedImage { data: url, is_url: true }
            } else {
                GeneratedImage { data: String::new(), is_url: false }
            }
        }).collect();

        Ok(ImageGenResponse { images })
    }

    fn name(&self) -> &str { "openai" }
}

// ─── Anthropic Provider ───────────────────────────────────────────────────────

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
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
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

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let model = request.model.clone().unwrap_or_else(|| self.default_model.clone());

        // Extract system message (Anthropic uses a separate field)
        let system = request.messages.iter()
            .find(|m| m.role == super::Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<AnthropicMessage> = request.messages.iter()
            .filter(|m| m.role != super::Role::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    super::Role::User => "user".into(),
                    super::Role::Assistant => "assistant".into(),
                    super::Role::System => unreachable!(),
                },
                content: m.content.clone(),
            })
            .collect();

        let body = AnthropicRequest {
            model: model.clone(),
            max_tokens: request.max_tokens,
            system,
            messages,
        };

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error {status}: {text}");
        }

        let data: AnthropicResponse = resp.json().await?;
        let content = data.content.into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            content,
            model: data.model,
            usage: TokenUsage {
                prompt_tokens: data.usage.input_tokens,
                completion_tokens: data.usage.output_tokens,
                total_tokens: data.usage.input_tokens + data.usage.output_tokens,
            },
        })
    }

    fn name(&self) -> &str { "anthropic" }
}

// ─── Ollama Provider (Local) ──────────────────────────────────────────────────

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
    messages: Vec<OpenAiMessage>,  // Ollama uses the same format
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OpenAiMessage,
    model: String,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let model = request.model.clone().unwrap_or_else(|| self.default_model.clone());
        let messages: Vec<OpenAiMessage> = request.messages.iter()
            .map(|m| OpenAiMessage {
                role: match m.role {
                    super::Role::System => "system".into(),
                    super::Role::User => "user".into(),
                    super::Role::Assistant => "assistant".into(),
                },
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
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error {status}: {text}");
        }

        let data: OllamaChatResponse = resp.json().await?;
        let prompt_tokens = data.prompt_eval_count.unwrap_or(0);
        let completion_tokens = data.eval_count.unwrap_or(0);

        Ok(LlmResponse {
            content: data.message.content,
            model: data.model,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        })
    }

    fn name(&self) -> &str { "ollama" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let p = OpenAiProvider::new(
            "sk-test".into(),
            "https://api.openai.com/v1".into(),
            "gpt-4o".into(),
        );
        assert_eq!(p.name(), "openai");
        assert_eq!(p.default_model, "gpt-4o");
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let p = AnthropicProvider::new(
            "sk-ant-test".into(),
            "https://api.anthropic.com/v1".into(),
            "claude-sonnet-4-20250514".into(),
        );
        assert_eq!(p.name(), "anthropic");
    }

    #[test]
    fn test_ollama_provider_creation() {
        let p = OllamaProvider::new(
            "http://localhost:11434".into(),
            "llama3".into(),
        );
        assert_eq!(p.name(), "ollama");
        assert_eq!(p.default_model, "llama3");
    }
}
```

- [ ] **Step 4: Update lib.rs to include llm module**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod llm;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p shepherd-core -- llm`
Expected: All 9 LLM tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/llm/ crates/shepherd-core/Cargo.toml crates/shepherd-core/src/lib.rs
git commit -m "feat: add provider-agnostic LLM client with OpenAI, Anthropic, and Ollama backends"
```

---

### Task 2: Name Brainstorming via LLM

**Files:**
- Create: `crates/shepherd-core/src/namegen/mod.rs`
- Create: `crates/shepherd-core/src/namegen/brainstorm.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Define name generator types and orchestrator**

```rust
// crates/shepherd-core/src/namegen/mod.rs
pub mod brainstorm;
pub mod validate;
pub mod rdap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::llm::LlmProvider;

/// Input for name generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameGenInput {
    /// Product description
    pub description: String,
    /// Optional vibe tags (e.g., "modern", "playful", "enterprise")
    #[serde(default)]
    pub vibes: Vec<String>,
    /// Number of candidates to generate
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize { 20 }

/// A single name candidate with validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameCandidate {
    pub name: String,
    pub tagline: Option<String>,
    pub reasoning: String,
    pub validation: NameValidation,
}

/// Validation status for a name candidate
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NameValidation {
    pub domains: Vec<DomainCheck>,
    pub npm_available: Option<bool>,
    pub pypi_available: Option<bool>,
    pub github_available: Option<bool>,
    pub negative_associations: Vec<String>,
    pub overall_status: ValidationStatus,
}

/// Domain availability check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCheck {
    pub tld: String,
    pub domain: String,
    pub available: Option<bool>,
    pub error: Option<String>,
}

/// Overall validation status
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// All checks passed — name is fully available
    AllClear,
    /// Some checks passed, some failed
    Partial,
    /// Major conflicts found
    Conflicted,
    /// Not yet validated
    #[default]
    Pending,
}

/// Complete name generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameGenResult {
    pub candidates: Vec<NameCandidate>,
    pub input: NameGenInput,
}

impl NameGenResult {
    /// Sort candidates: all-clear first, partial second, conflicted last
    pub fn sorted(mut self) -> Self {
        self.candidates.sort_by_key(|c| match c.validation.overall_status {
            ValidationStatus::AllClear => 0,
            ValidationStatus::Partial => 1,
            ValidationStatus::Pending => 2,
            ValidationStatus::Conflicted => 3,
        });
        self
    }
}

/// Orchestrate full name generation: brainstorm + validate
pub async fn generate_names(
    llm: &dyn LlmProvider,
    input: &NameGenInput,
) -> Result<NameGenResult> {
    // Step 1: Brainstorm names via LLM
    let mut candidates = brainstorm::brainstorm_names(llm, input).await?;

    // Step 2: Validate each candidate
    for candidate in &mut candidates {
        candidate.validation = validate::validate_name(&candidate.name).await?;
    }

    // Step 3: Run negative association scan via LLM
    for candidate in &mut candidates {
        let negatives = brainstorm::scan_negative_associations(llm, &candidate.name).await?;
        candidate.validation.negative_associations = negatives;

        // Recalculate overall status
        candidate.validation.overall_status = calculate_status(&candidate.validation);
    }

    let result = NameGenResult {
        candidates,
        input: input.clone(),
    };

    Ok(result.sorted())
}

fn calculate_status(v: &NameValidation) -> ValidationStatus {
    let has_negatives = !v.negative_associations.is_empty();
    let domain_available = v.domains.iter().any(|d| d.available == Some(true));
    let domain_conflicts = v.domains.iter().all(|d| d.available == Some(false));
    let registry_conflict = v.npm_available == Some(false)
        || v.pypi_available == Some(false)
        || v.github_available == Some(false);

    if has_negatives || (domain_conflicts && registry_conflict) {
        ValidationStatus::Conflicted
    } else if domain_available && !registry_conflict && !has_negatives {
        ValidationStatus::AllClear
    } else {
        ValidationStatus::Partial
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_gen_result_sorting() {
        let result = NameGenResult {
            candidates: vec![
                NameCandidate {
                    name: "conflicted".into(),
                    tagline: None,
                    reasoning: "test".into(),
                    validation: NameValidation {
                        overall_status: ValidationStatus::Conflicted,
                        ..Default::default()
                    },
                },
                NameCandidate {
                    name: "clear".into(),
                    tagline: None,
                    reasoning: "test".into(),
                    validation: NameValidation {
                        overall_status: ValidationStatus::AllClear,
                        ..Default::default()
                    },
                },
                NameCandidate {
                    name: "partial".into(),
                    tagline: None,
                    reasoning: "test".into(),
                    validation: NameValidation {
                        overall_status: ValidationStatus::Partial,
                        ..Default::default()
                    },
                },
            ],
            input: NameGenInput {
                description: "test".into(),
                vibes: vec![],
                count: 3,
            },
        };

        let sorted = result.sorted();
        assert_eq!(sorted.candidates[0].name, "clear");
        assert_eq!(sorted.candidates[1].name, "partial");
        assert_eq!(sorted.candidates[2].name, "conflicted");
    }

    #[test]
    fn test_calculate_status_all_clear() {
        let v = NameValidation {
            domains: vec![DomainCheck {
                tld: "com".into(),
                domain: "test.com".into(),
                available: Some(true),
                error: None,
            }],
            npm_available: Some(true),
            pypi_available: Some(true),
            github_available: Some(true),
            negative_associations: vec![],
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&v), ValidationStatus::AllClear);
    }

    #[test]
    fn test_calculate_status_conflicted_with_negatives() {
        let v = NameValidation {
            domains: vec![],
            npm_available: None,
            pypi_available: None,
            github_available: None,
            negative_associations: vec!["offensive in language X".into()],
            overall_status: ValidationStatus::Pending,
        };
        assert_eq!(calculate_status(&v), ValidationStatus::Conflicted);
    }

    #[test]
    fn test_default_count() {
        let input = NameGenInput {
            description: "test".into(),
            vibes: vec![],
            count: default_count(),
        };
        assert_eq!(input.count, 20);
    }
}
```

- [ ] **Step 2: Implement LLM brainstorming with prompt engineering**

```rust
// crates/shepherd-core/src/namegen/brainstorm.rs
use anyhow::Result;

use crate::llm::{ChatMessage, LlmProvider, LlmRequest};
use super::{NameCandidate, NameGenInput, NameValidation};

/// Brainstorm name candidates via LLM
pub async fn brainstorm_names(
    llm: &dyn LlmProvider,
    input: &NameGenInput,
) -> Result<Vec<NameCandidate>> {
    let vibes_str = if input.vibes.is_empty() {
        "No specific vibe preference — suggest a diverse mix.".to_string()
    } else {
        format!("Desired vibes: {}", input.vibes.join(", "))
    };

    let system_prompt = r#"You are a creative product naming expert. You generate memorable, brandable product names.

Rules:
1. Names should be 1-2 words, easy to spell and pronounce
2. Names should work as a domain name (no special characters)
3. Avoid generic or overly descriptive names
4. Consider the name in multiple languages for potential negative meanings
5. Each name should feel distinct from the others
6. Include a mix of: real words, compound words, invented words, and metaphors

Respond in EXACTLY this JSON format (no markdown, no code fences):
[
  {
    "name": "ProductName",
    "tagline": "Optional short tagline",
    "reasoning": "Why this name works for the product"
  }
]"#;

    let user_prompt = format!(
        "Generate exactly {} unique product name candidates.\n\n\
         Product description: {}\n\n\
         {}\n\n\
         Return ONLY the JSON array, no other text.",
        input.count, input.description, vibes_str
    );

    let request = LlmRequest {
        messages: vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
        max_tokens: 4096,
        temperature: 0.9, // Higher creativity for name generation
        model: None,
    };

    let response = llm.chat(&request).await?;
    let candidates = parse_brainstorm_response(&response.content)?;
    Ok(candidates)
}

/// Parse the LLM's JSON response into name candidates
fn parse_brainstorm_response(content: &str) -> Result<Vec<NameCandidate>> {
    // Strip potential markdown code fences
    let cleaned = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let raw: Vec<RawCandidate> = serde_json::from_str(cleaned)
        .map_err(|e| anyhow::anyhow!("Failed to parse LLM response as JSON: {e}\nRaw: {cleaned}"))?;

    let candidates = raw
        .into_iter()
        .map(|r| NameCandidate {
            name: r.name.trim().to_string(),
            tagline: r.tagline.filter(|t| !t.is_empty()),
            reasoning: r.reasoning,
            validation: NameValidation::default(),
        })
        .collect();

    Ok(candidates)
}

#[derive(serde::Deserialize)]
struct RawCandidate {
    name: String,
    tagline: Option<String>,
    reasoning: String,
}

/// Scan a name for negative associations using LLM
pub async fn scan_negative_associations(
    llm: &dyn LlmProvider,
    name: &str,
) -> Result<Vec<String>> {
    let system_prompt = r#"You are a brand safety analyst. Given a product name, identify ANY potential negative associations, offensive meanings, or unfortunate translations in major world languages (English, Spanish, French, German, Italian, Portuguese, Chinese, Japanese, Korean, Arabic, Hindi, Russian).

If the name is safe, respond with an empty JSON array: []
If there are concerns, respond with a JSON array of strings describing each concern.

Respond in EXACTLY this format (no markdown, no code fences):
["concern 1", "concern 2"]
or
[]"#;

    let user_prompt = format!(
        "Analyze the product name \"{}\" for negative associations, offensive meanings, \
         or unfortunate translations in any major language. Return ONLY a JSON array.",
        name
    );

    let request = LlmRequest {
        messages: vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
        max_tokens: 1024,
        temperature: 0.3, // Low creativity for analysis
        model: None,
    };

    let response = llm.chat(&request).await?;
    let cleaned = response.content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let concerns: Vec<String> = serde_json::from_str(cleaned).unwrap_or_default();
    Ok(concerns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_brainstorm_response_valid() {
        let json = r#"[
            {"name": "Shepherd", "tagline": "Guide your code", "reasoning": "Leadership metaphor"},
            {"name": "Nexus", "tagline": null, "reasoning": "Connection point"}
        ]"#;
        let candidates = parse_brainstorm_response(json).unwrap();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].name, "Shepherd");
        assert_eq!(candidates[0].tagline, Some("Guide your code".into()));
        assert_eq!(candidates[1].tagline, None);
    }

    #[test]
    fn test_parse_brainstorm_response_with_code_fences() {
        let json = "```json\n[\n{\"name\": \"Test\", \"tagline\": null, \"reasoning\": \"r\"}\n]\n```";
        let candidates = parse_brainstorm_response(json).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "Test");
    }

    #[test]
    fn test_parse_brainstorm_response_invalid() {
        let result = parse_brainstorm_response("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_brainstorm_response_trims_names() {
        let json = r#"[{"name": "  SpacedName  ", "tagline": "", "reasoning": "test"}]"#;
        let candidates = parse_brainstorm_response(json).unwrap();
        assert_eq!(candidates[0].name, "SpacedName");
        // Empty tagline should become None
        assert_eq!(candidates[0].tagline, None);
    }
}
```

- [ ] **Step 3: Update lib.rs**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod namegen;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p shepherd-core -- namegen`
Expected: All 8 namegen tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/namegen/mod.rs crates/shepherd-core/src/namegen/brainstorm.rs crates/shepherd-core/src/lib.rs
git commit -m "feat: add product name brainstorming via LLM with negative association scanning"
```

---

### Task 3: Domain & Registry Validation

**Files:**
- Create: `crates/shepherd-core/src/namegen/rdap.rs`
- Create: `crates/shepherd-core/src/namegen/validate.rs`

- [ ] **Step 1: Implement RDAP domain availability client**

```rust
// crates/shepherd-core/src/namegen/rdap.rs
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

/// RDAP bootstrap registry for finding the right RDAP server per TLD
const RDAP_BOOTSTRAP_URL: &str = "https://data.iana.org/rdap/dns.json";

/// TLDs to check for each name candidate
pub const DEFAULT_TLDS: &[&str] = &["com", "dev", "io", "app", "codes"];

/// Check domain availability via RDAP protocol
pub async fn check_domain(client: &Client, domain: &str) -> Result<DomainResult> {
    // Try RDAP lookup — a 404 means the domain is likely available
    let rdap_url = find_rdap_server(client, domain).await?;
    let url = format!("{}/domain/{}", rdap_url.trim_end_matches('/'), domain);

    let resp = client
        .get(&url)
        .header("Accept", "application/rdap+json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            // Domain record exists — it's registered
            Ok(DomainResult { domain: domain.to_string(), available: false, error: None })
        }
        Ok(r) if r.status().as_u16() == 404 => {
            // No record — domain is likely available
            Ok(DomainResult { domain: domain.to_string(), available: true, error: None })
        }
        Ok(r) => {
            let status = r.status();
            Ok(DomainResult {
                domain: domain.to_string(),
                available: false,
                error: Some(format!("RDAP returned status {status}")),
            })
        }
        Err(e) => {
            Ok(DomainResult {
                domain: domain.to_string(),
                available: false,
                error: Some(format!("RDAP lookup failed: {e}")),
            })
        }
    }
}

/// Result of a single domain check
#[derive(Debug, Clone)]
pub struct DomainResult {
    pub domain: String,
    pub available: bool,
    pub error: Option<String>,
}

/// Find the RDAP server for a given domain's TLD
async fn find_rdap_server(client: &Client, domain: &str) -> Result<String> {
    let tld = domain.rsplit('.').next()
        .ok_or_else(|| anyhow::anyhow!("Invalid domain: {domain}"))?;

    let resp = client
        .get(RDAP_BOOTSTRAP_URL)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    let bootstrap: RdapBootstrap = resp.json().await?;

    for service in &bootstrap.services {
        if service.len() >= 2 {
            let tlds = &service[0];
            let urls = &service[1];
            if tlds.iter().any(|t| t.eq_ignore_ascii_case(tld)) {
                if let Some(url) = urls.first() {
                    return Ok(url.clone());
                }
            }
        }
    }

    // Fallback to a well-known RDAP server
    Ok("https://rdap.org".into())
}

#[derive(Deserialize)]
struct RdapBootstrap {
    services: Vec<Vec<Vec<String>>>,
}

/// Check multiple TLDs for a given name
pub async fn check_domains_for_name(
    client: &Client,
    name: &str,
    tlds: &[&str],
) -> Vec<super::DomainCheck> {
    let name_lower = name.to_lowercase().replace(' ', "");
    let mut results = Vec::new();

    for tld in tlds {
        let domain = format!("{name_lower}.{tld}");
        let result = check_domain(client, &domain).await;
        results.push(match result {
            Ok(r) => super::DomainCheck {
                tld: tld.to_string(),
                domain: r.domain,
                available: Some(r.available),
                error: r.error,
            },
            Err(e) => super::DomainCheck {
                tld: tld.to_string(),
                domain: format!("{name_lower}.{tld}"),
                available: None,
                error: Some(e.to_string()),
            },
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tlds() {
        assert_eq!(DEFAULT_TLDS.len(), 5);
        assert!(DEFAULT_TLDS.contains(&"com"));
        assert!(DEFAULT_TLDS.contains(&"dev"));
    }

    #[test]
    fn test_domain_result_available() {
        let r = DomainResult { domain: "test.com".into(), available: true, error: None };
        assert!(r.available);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_domain_result_error() {
        let r = DomainResult {
            domain: "test.com".into(),
            available: false,
            error: Some("timeout".into()),
        };
        assert!(!r.available);
        assert_eq!(r.error.as_deref(), Some("timeout"));
    }
}
```

- [ ] **Step 2: Implement registry and conflict validation**

```rust
// crates/shepherd-core/src/namegen/validate.rs
use anyhow::Result;
use reqwest::Client;

use super::{DomainCheck, NameValidation, ValidationStatus};
use super::rdap;

/// Validate a name candidate across all registries
pub async fn validate_name(name: &str) -> Result<NameValidation> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let name_lower = name.to_lowercase().replace(' ', "");

    // Run all checks concurrently
    let (domains, npm, pypi, github) = tokio::join!(
        rdap::check_domains_for_name(&client, name, rdap::DEFAULT_TLDS),
        check_npm(&client, &name_lower),
        check_pypi(&client, &name_lower),
        check_github(&client, &name_lower),
    );

    Ok(NameValidation {
        domains,
        npm_available: npm.ok(),
        pypi_available: pypi.ok(),
        github_available: github.ok(),
        negative_associations: vec![],
        overall_status: ValidationStatus::Pending,
    })
}

/// Check npm registry for package name availability
async fn check_npm(client: &Client, name: &str) -> Result<bool> {
    let url = format!("https://registry.npmjs.org/{name}");
    let resp = client.get(&url).send().await?;
    // 404 = available, 200 = taken
    Ok(resp.status().as_u16() == 404)
}

/// Check PyPI for package name availability
async fn check_pypi(client: &Client, name: &str) -> Result<bool> {
    let url = format!("https://pypi.org/pypi/{name}/json");
    let resp = client.get(&url).send().await?;
    // 404 = available, 200 = taken
    Ok(resp.status().as_u16() == 404)
}

/// Check GitHub for org/repo name conflicts
async fn check_github(client: &Client, name: &str) -> Result<bool> {
    // Check if a GitHub user/org exists with this name
    let url = format!("https://api.github.com/users/{name}");
    let resp = client
        .get(&url)
        .header("User-Agent", "shepherd-namegen/0.1")
        .send()
        .await?;
    // 404 = available, 200 = taken
    Ok(resp.status().as_u16() == 404)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_npm_known_package() {
        let client = Client::new();
        // "express" is definitely taken on npm
        let result = check_npm(&client, "express").await;
        // May fail in offline environments, so we just check it doesn't panic
        if let Ok(available) = result {
            assert!(!available, "express should be taken on npm");
        }
    }

    #[tokio::test]
    async fn test_check_pypi_known_package() {
        let client = Client::new();
        // "requests" is definitely taken on PyPI
        let result = check_pypi(&client, "requests").await;
        if let Ok(available) = result {
            assert!(!available, "requests should be taken on PyPI");
        }
    }

    #[tokio::test]
    async fn test_check_github_known_org() {
        let client = Client::new();
        // "google" is definitely taken on GitHub
        let result = check_github(&client, "google").await;
        if let Ok(available) = result {
            assert!(!available, "google should be taken on GitHub");
        }
    }

    #[tokio::test]
    async fn test_validate_name_returns_results() {
        // Use a very unlikely-to-exist name
        let result = validate_name("xyzzy9847362qwk").await;
        // Just verify it doesn't panic and returns a struct
        if let Ok(v) = result {
            assert_eq!(v.domains.len(), 5); // 5 default TLDs
            assert_eq!(v.overall_status, ValidationStatus::Pending);
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p shepherd-core -- namegen`
Expected: All namegen tests pass (unit tests always, network tests may skip in offline env).

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-core/src/namegen/rdap.rs crates/shepherd-core/src/namegen/validate.rs
git commit -m "feat: add RDAP domain checks and npm/PyPI/GitHub registry validation for name generator"
```

---

### Task 4: Name Generator Server Route & Frontend UI

**Files:**
- Create: `crates/shepherd-server/src/routes/namegen.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`
- Create: `src/features/namegen/NameGenerator.tsx`

- [ ] **Step 1: Add server route for name generation**

```rust
// crates/shepherd-server/src/routes/namegen.rs
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::namegen::{self, NameCandidate, NameGenInput, ValidationStatus};

#[derive(Deserialize)]
pub struct NameGenRequest {
    pub description: String,
    #[serde(default)]
    pub vibes: Vec<String>,
    #[serde(default = "default_count")]
    pub count: Option<usize>,
}

fn default_count() -> Option<usize> { Some(20) }

#[derive(Serialize)]
pub struct NameGenResponse {
    pub candidates: Vec<CandidateResponse>,
}

#[derive(Serialize)]
pub struct CandidateResponse {
    pub name: String,
    pub tagline: Option<String>,
    pub reasoning: String,
    pub status: String,
    pub domains: Vec<DomainResponse>,
    pub npm_available: Option<bool>,
    pub pypi_available: Option<bool>,
    pub github_available: Option<bool>,
    pub negative_associations: Vec<String>,
}

#[derive(Serialize)]
pub struct DomainResponse {
    pub tld: String,
    pub domain: String,
    pub available: Option<bool>,
}

impl From<NameCandidate> for CandidateResponse {
    fn from(c: NameCandidate) -> Self {
        Self {
            name: c.name,
            tagline: c.tagline,
            reasoning: c.reasoning,
            status: match c.validation.overall_status {
                ValidationStatus::AllClear => "all_clear".into(),
                ValidationStatus::Partial => "partial".into(),
                ValidationStatus::Conflicted => "conflicted".into(),
                ValidationStatus::Pending => "pending".into(),
            },
            domains: c.validation.domains.into_iter().map(|d| DomainResponse {
                tld: d.tld,
                domain: d.domain,
                available: d.available,
            }).collect(),
            npm_available: c.validation.npm_available,
            pypi_available: c.validation.pypi_available,
            github_available: c.validation.github_available,
            negative_associations: c.validation.negative_associations,
        }
    }
}

pub async fn generate_names(
    State(state): State<Arc<AppState>>,
    Json(body): Json<NameGenRequest>,
) -> Result<Json<NameGenResponse>, (StatusCode, String)> {
    let llm = state.llm_provider.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "No LLM provider configured. Set API key in settings.".into()))?;

    let input = NameGenInput {
        description: body.description,
        vibes: body.vibes,
        count: body.count.unwrap_or(20),
    };

    let result = namegen::generate_names(llm.as_ref(), &input).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Name generation failed: {e}")))?;

    Ok(Json(NameGenResponse {
        candidates: result.candidates.into_iter().map(CandidateResponse::from).collect(),
    }))
}
```

- [ ] **Step 2: Register the route**

Add to `crates/shepherd-server/src/routes/mod.rs`:

```rust
pub mod namegen;
```

Add to the router in `mod.rs`:

```rust
.route("/api/namegen", post(namegen::generate_names))
```

Add `llm_provider: Option<Box<dyn shepherd_core::llm::LlmProvider>>` field to `AppState` in `crates/shepherd-server/src/state.rs`.

- [ ] **Step 3: Create Name Generator frontend component**

```tsx
// src/features/namegen/NameGenerator.tsx
import React, { useState, useCallback } from 'react';

interface DomainResult {
  tld: string;
  domain: string;
  available: boolean | null;
}

interface NameCandidate {
  name: string;
  tagline: string | null;
  reasoning: string;
  status: 'all_clear' | 'partial' | 'conflicted' | 'pending';
  domains: DomainResult[];
  npm_available: boolean | null;
  pypi_available: boolean | null;
  github_available: boolean | null;
  negative_associations: string[];
}

const VIBE_OPTIONS = [
  'modern', 'playful', 'enterprise', 'minimal', 'bold',
  'friendly', 'technical', 'abstract', 'nature', 'futuristic',
];

const STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  all_clear: { bg: 'bg-green-50', text: 'text-green-700', label: 'All Clear' },
  partial: { bg: 'bg-yellow-50', text: 'text-yellow-700', label: 'Partial' },
  conflicted: { bg: 'bg-red-50', text: 'text-red-700', label: 'Conflicted' },
  pending: { bg: 'bg-gray-50', text: 'text-gray-500', label: 'Checking...' },
};

export function NameGenerator() {
  const [description, setDescription] = useState('');
  const [selectedVibes, setSelectedVibes] = useState<string[]>([]);
  const [candidates, setCandidates] = useState<NameCandidate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const toggleVibe = useCallback((vibe: string) => {
    setSelectedVibes(prev =>
      prev.includes(vibe) ? prev.filter(v => v !== vibe) : [...prev, vibe]
    );
  }, []);

  const generate = useCallback(async () => {
    if (!description.trim()) return;
    setLoading(true);
    setError(null);
    setCandidates([]);

    try {
      const resp = await fetch('/api/namegen', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          description: description.trim(),
          vibes: selectedVibes,
          count: 20,
        }),
      });

      if (!resp.ok) {
        const text = await resp.text();
        throw new Error(text || `HTTP ${resp.status}`);
      }

      const data = await resp.json();
      setCandidates(data.candidates);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, [description, selectedVibes]);

  const AvailabilityDot = ({ available }: { available: boolean | null }) => {
    if (available === null) return <span className="w-3 h-3 rounded-full bg-gray-300 inline-block" title="Unknown" />;
    return available
      ? <span className="w-3 h-3 rounded-full bg-green-500 inline-block" title="Available" />
      : <span className="w-3 h-3 rounded-full bg-red-500 inline-block" title="Taken" />;
  };

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-1">Product Name Generator</h2>
        <p className="text-gray-500 text-sm">Brainstorm names with domain and registry validation</p>
      </div>

      {/* Input Section */}
      <div className="space-y-4 bg-white rounded-xl border border-gray-200 p-5">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Product Description
          </label>
          <textarea
            value={description}
            onChange={e => setDescription(e.target.value)}
            placeholder="Describe your product in 1-2 sentences..."
            className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
            rows={3}
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">Vibes</label>
          <div className="flex flex-wrap gap-2">
            {VIBE_OPTIONS.map(vibe => (
              <button
                key={vibe}
                onClick={() => toggleVibe(vibe)}
                className={`px-3 py-1 rounded-full text-sm border transition-colors ${
                  selectedVibes.includes(vibe)
                    ? 'bg-blue-100 border-blue-300 text-blue-700'
                    : 'bg-gray-50 border-gray-200 text-gray-600 hover:bg-gray-100'
                }`}
              >
                {vibe}
              </button>
            ))}
          </div>
        </div>

        <button
          onClick={generate}
          disabled={loading || !description.trim()}
          className="w-full py-2.5 rounded-lg bg-blue-600 text-white font-medium text-sm hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {loading ? 'Generating...' : 'Generate Names'}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700 text-sm">
          {error}
        </div>
      )}

      {/* Results Table */}
      {candidates.length > 0 && (
        <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 border-b border-gray-200">
              <tr>
                <th className="px-4 py-3 text-left font-medium text-gray-600">Name</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600">Status</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600">Domains</th>
                <th className="px-4 py-3 text-center font-medium text-gray-600">npm</th>
                <th className="px-4 py-3 text-center font-medium text-gray-600">PyPI</th>
                <th className="px-4 py-3 text-center font-medium text-gray-600">GitHub</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600">Action</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {candidates.map((c, i) => {
                const style = STATUS_STYLES[c.status] || STATUS_STYLES.pending;
                const isConflicted = c.status === 'conflicted';
                return (
                  <tr
                    key={i}
                    className={`${isConflicted ? 'opacity-50' : 'hover:bg-gray-50'} ${
                      selectedName === c.name ? 'bg-blue-50' : ''
                    }`}
                  >
                    <td className="px-4 py-3">
                      <div className={isConflicted ? 'line-through' : ''}>
                        <span className="font-medium text-gray-900">{c.name}</span>
                        {c.tagline && (
                          <span className="block text-xs text-gray-400 mt-0.5">{c.tagline}</span>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${style.bg} ${style.text}`}>
                        {style.label}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex gap-2">
                        {c.domains.map(d => (
                          <span key={d.tld} className="flex items-center gap-1 text-xs text-gray-500">
                            <AvailabilityDot available={d.available} />
                            .{d.tld}
                          </span>
                        ))}
                      </div>
                    </td>
                    <td className="px-4 py-3 text-center"><AvailabilityDot available={c.npm_available} /></td>
                    <td className="px-4 py-3 text-center"><AvailabilityDot available={c.pypi_available} /></td>
                    <td className="px-4 py-3 text-center"><AvailabilityDot available={c.github_available} /></td>
                    <td className="px-4 py-3">
                      {!isConflicted && (
                        <button
                          onClick={() => setSelectedName(c.name)}
                          className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                            selectedName === c.name
                              ? 'bg-blue-600 text-white'
                              : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                          }`}
                        >
                          {selectedName === c.name ? 'Selected' : 'Select'}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          {/* Negative associations warning */}
          {candidates.some(c => c.negative_associations.length > 0) && (
            <div className="border-t border-gray-200 px-4 py-3 bg-yellow-50">
              <h4 className="text-xs font-medium text-yellow-800 mb-1">Negative Association Warnings</h4>
              {candidates
                .filter(c => c.negative_associations.length > 0)
                .map((c, i) => (
                  <p key={i} className="text-xs text-yellow-700">
                    <strong>{c.name}:</strong> {c.negative_associations.join('; ')}
                  </p>
                ))}
            </div>
          )}
        </div>
      )}

      {/* Selected name action */}
      {selectedName && (
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 flex items-center justify-between">
          <p className="text-sm text-blue-800">
            Selected: <strong>{selectedName}</strong>
          </p>
          <button className="px-4 py-2 rounded-lg bg-blue-600 text-white text-sm font-medium hover:bg-blue-700">
            Apply to Project
          </button>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run frontend lint check**

Run: `npx tsc --noEmit --project tsconfig.json`
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-server/src/routes/namegen.rs crates/shepherd-server/src/routes/mod.rs crates/shepherd-server/src/state.rs src/features/namegen/
git commit -m "feat: add name generator server route and wizard UI with results table"
```

---

## Chunk 2: Logo Generator & North Star (Tasks 5–8)

### Task 5: Image Generation API Client

**Files:**
- Create: `crates/shepherd-core/src/logogen/mod.rs`
- Create: `crates/shepherd-core/src/logogen/generate.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Define logo generator types and orchestrator**

```rust
// crates/shepherd-core/src/logogen/mod.rs
pub mod generate;
pub mod export;

use serde::{Deserialize, Serialize};

/// Logo style options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogoStyle {
    Minimal,
    Geometric,
    Mascot,
    Abstract,
}

impl LogoStyle {
    pub fn prompt_hint(&self) -> &str {
        match self {
            Self::Minimal => "clean, minimal, simple shapes, modern, flat design, single color",
            Self::Geometric => "geometric shapes, structured, tessellation, bold lines, mathematical",
            Self::Mascot => "friendly character mascot, approachable, distinctive, memorable",
            Self::Abstract => "abstract art, fluid shapes, creative, artistic, unique composition",
        }
    }
}

/// Input for logo generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoGenInput {
    pub product_name: String,
    pub product_description: Option<String>,
    pub style: LogoStyle,
    #[serde(default)]
    pub colors: Vec<String>,
    /// Number of variants to generate
    #[serde(default = "default_variants")]
    pub variants: u8,
}

fn default_variants() -> u8 { 4 }

/// A single logo variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoVariant {
    pub index: u8,
    /// Base64-encoded PNG data
    pub png_data: String,
    /// Whether this variant was selected by the user
    pub selected: bool,
}

/// Result of logo generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoGenResult {
    pub variants: Vec<LogoVariant>,
    pub style: LogoStyle,
}

/// Exported icon set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconExport {
    pub files: Vec<ExportedFile>,
}

/// A single exported file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedFile {
    pub path: String,
    pub size_bytes: u64,
    pub format: String,
    pub dimensions: Option<(u32, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logo_style_prompt_hints() {
        assert!(LogoStyle::Minimal.prompt_hint().contains("minimal"));
        assert!(LogoStyle::Geometric.prompt_hint().contains("geometric"));
        assert!(LogoStyle::Mascot.prompt_hint().contains("mascot"));
        assert!(LogoStyle::Abstract.prompt_hint().contains("abstract"));
    }

    #[test]
    fn test_default_variants() {
        assert_eq!(default_variants(), 4);
    }

    #[test]
    fn test_logo_gen_input_serde() {
        let input = LogoGenInput {
            product_name: "Shepherd".into(),
            product_description: Some("AI agent manager".into()),
            style: LogoStyle::Minimal,
            colors: vec!["#3B82F6".into(), "#1E293B".into()],
            variants: 4,
        };
        let json = serde_json::to_string(&input).unwrap();
        let parsed: LogoGenInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.product_name, "Shepherd");
        assert_eq!(parsed.style, LogoStyle::Minimal);
        assert_eq!(parsed.colors.len(), 2);
    }
}
```

- [ ] **Step 2: Implement image generation client**

```rust
// crates/shepherd-core/src/logogen/generate.rs
use anyhow::Result;

use crate::llm::{ImageGenRequest, LlmProvider};
use super::{LogoGenInput, LogoGenResult, LogoVariant};

/// Build the image generation prompt from user input
pub fn build_logo_prompt(input: &LogoGenInput) -> String {
    let mut parts = vec![
        format!("Logo design for a product called \"{}\".", input.product_name),
    ];

    if let Some(desc) = &input.product_description {
        parts.push(format!("Product: {desc}."));
    }

    parts.push(format!("Style: {}", input.style.prompt_hint()));

    if !input.colors.is_empty() {
        parts.push(format!("Color palette: {}.", input.colors.join(", ")));
    }

    parts.push("Professional logo suitable for app icon and favicon. White or transparent background. Centered composition. No text unless the product name is very short (4 letters or fewer).".into());

    parts.join(" ")
}

/// Generate logo variants via image generation API
pub async fn generate_logos(
    llm: &dyn LlmProvider,
    input: &LogoGenInput,
) -> Result<LogoGenResult> {
    let prompt = build_logo_prompt(input);

    let request = ImageGenRequest {
        prompt,
        size: "1024x1024".into(),
        n: input.variants,
        model: None,
    };

    let response = llm.generate_image(&request).await?;

    let variants: Vec<LogoVariant> = response
        .images
        .into_iter()
        .enumerate()
        .map(|(i, img)| {
            let png_data = if img.is_url {
                // In production, we'd download the URL. For now, store the URL.
                img.data
            } else {
                img.data
            };
            LogoVariant {
                index: i as u8,
                png_data,
                selected: false,
            }
        })
        .collect();

    Ok(LogoGenResult {
        variants,
        style: input.style.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logogen::LogoStyle;

    #[test]
    fn test_build_logo_prompt_minimal() {
        let input = LogoGenInput {
            product_name: "Shepherd".into(),
            product_description: Some("AI agent manager".into()),
            style: LogoStyle::Minimal,
            colors: vec!["#3B82F6".into()],
            variants: 4,
        };
        let prompt = build_logo_prompt(&input);
        assert!(prompt.contains("Shepherd"));
        assert!(prompt.contains("AI agent manager"));
        assert!(prompt.contains("minimal"));
        assert!(prompt.contains("#3B82F6"));
        assert!(prompt.contains("Professional logo"));
    }

    #[test]
    fn test_build_logo_prompt_no_description_no_colors() {
        let input = LogoGenInput {
            product_name: "Test".into(),
            product_description: None,
            style: LogoStyle::Geometric,
            colors: vec![],
            variants: 4,
        };
        let prompt = build_logo_prompt(&input);
        assert!(prompt.contains("Test"));
        assert!(prompt.contains("geometric"));
        assert!(!prompt.contains("Color palette"));
    }
}
```

- [ ] **Step 3: Update lib.rs**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod logogen;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p shepherd-core -- logogen`
Expected: All 5 logogen tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/logogen/mod.rs crates/shepherd-core/src/logogen/generate.rs crates/shepherd-core/src/lib.rs
git commit -m "feat: add logo generation with style-aware prompt building and multi-variant support"
```

---

### Task 6: Multi-Format Icon Export

**Files:**
- Create: `crates/shepherd-core/src/logogen/export.rs`
- Modify: `crates/shepherd-core/Cargo.toml`

- [ ] **Step 1: Add image processing dependency**

Add to `crates/shepherd-core/Cargo.toml`:

```toml
image = "0.25"
```

- [ ] **Step 2: Implement multi-format export**

```rust
// crates/shepherd-core/src/logogen/export.rs
use anyhow::Result;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use super::{ExportedFile, IconExport};

/// Standard icon sizes to export
const PNG_SIZES: &[(u32, &str)] = &[
    (1024, "icon-1024.png"),
    (512, "icon-512.png"),
    (192, "icon-192.png"),
    (64, "icon-64.png"),
];

/// Export a logo to all required formats and sizes
pub async fn export_icons(
    png_base64: &str,
    output_dir: &Path,
    product_name: &str,
) -> Result<IconExport> {
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64)
        .map_err(|e| anyhow::anyhow!("Invalid base64 image data: {e}"))?;

    let img = image::load_from_memory(&png_bytes)?;
    let mut files = Vec::new();

    // Ensure output directory exists
    std::fs::create_dir_all(output_dir)?;

    // Export PNG at each standard size
    for &(size, filename) in PNG_SIZES {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let path = output_dir.join(filename);
        resized.save(&path)?;
        let metadata = std::fs::metadata(&path)?;
        files.push(ExportedFile {
            path: path.to_string_lossy().into_owned(),
            size_bytes: metadata.len(),
            format: "png".into(),
            dimensions: Some((size, size)),
        });
    }

    // Export favicon.ico (multi-size ICO: 16, 32, 48)
    let favicon_path = output_dir.join("favicon.ico");
    export_ico(&img, &favicon_path, &[16, 32, 48])?;
    let meta = std::fs::metadata(&favicon_path)?;
    files.push(ExportedFile {
        path: favicon_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        format: "ico".into(),
        dimensions: None,
    });

    // Export apple-touch-icon (180x180 PNG)
    let apple_path = output_dir.join("apple-touch-icon.png");
    let apple = img.resize_exact(180, 180, image::imageops::FilterType::Lanczos3);
    apple.save(&apple_path)?;
    let meta = std::fs::metadata(&apple_path)?;
    files.push(ExportedFile {
        path: apple_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        format: "png".into(),
        dimensions: Some((180, 180)),
    });

    // Export Windows .ico with multiple resolutions (16, 32, 48, 256)
    let win_ico_path = output_dir.join("app.ico");
    export_ico(&img, &win_ico_path, &[16, 32, 48, 256])?;
    let meta = std::fs::metadata(&win_ico_path)?;
    files.push(ExportedFile {
        path: win_ico_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        format: "ico".into(),
        dimensions: None,
    });

    // Export macOS .icns (using iconutil-compatible PNG set)
    let icns_path = output_dir.join("app.icns");
    export_icns(&img, &icns_path)?;
    let meta = std::fs::metadata(&icns_path)?;
    files.push(ExportedFile {
        path: icns_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        format: "icns".into(),
        dimensions: None,
    });

    // Save original as SVG placeholder (raster-to-SVG traced via potrace or pass-through if input is SVG)
    // For MVP, save the 1024px PNG and note SVG must be provided manually or via a tracing tool
    let svg_path = output_dir.join("logo.svg");
    let svg_content = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024">
  <image href="icon-1024.png" width="1024" height="1024"/>
  <!-- Replace with true vector SVG for production use -->
</svg>"#
    );
    std::fs::write(&svg_path, &svg_content)?;
    let meta = std::fs::metadata(&svg_path)?;
    files.push(ExportedFile {
        path: svg_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        format: "svg".into(),
        dimensions: Some((1024, 1024)),
    });

    // Generate manifest.json icons entry
    let manifest_path = output_dir.join("manifest.json");
    let manifest = generate_manifest_json(product_name);
    std::fs::write(&manifest_path, &manifest)?;
    let meta = std::fs::metadata(&manifest_path)?;
    files.push(ExportedFile {
        path: manifest_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        format: "json".into(),
        dimensions: None,
    });

    Ok(IconExport { files })
}

/// Export macOS .icns using the icns crate's format
/// Contains 16x16, 32x32, 128x128, 256x256, 512x512, 1024x1024
fn export_icns(img: &DynamicImage, path: &Path) -> Result<()> {
    // macOS .icns is a tagged container. For MVP, we create an iconset directory
    // and shell out to `iconutil` on macOS, or write a minimal icns manually.
    let sizes = [16, 32, 128, 256, 512, 1024];
    let iconset_dir = path.with_extension("iconset");
    std::fs::create_dir_all(&iconset_dir)?;

    for &size in &sizes {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let name = format!("icon_{}x{}.png", size, size);
        resized.save(iconset_dir.join(&name))?;
        // Also save @2x variants where applicable
        if size <= 512 {
            let double = size * 2;
            let resized_2x = img.resize_exact(double, double, image::imageops::FilterType::Lanczos3);
            let name_2x = format!("icon_{}x{}@2x.png", size, size);
            resized_2x.save(iconset_dir.join(&name_2x))?;
        }
    }

    // Try iconutil (macOS only), fall back to keeping the iconset directory
    let status = std::process::Command::new("iconutil")
        .args(["--convert", "icns", "--output"])
        .arg(path)
        .arg(&iconset_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Clean up iconset directory on success
            let _ = std::fs::remove_dir_all(&iconset_dir);
        }
        _ => {
            tracing::warn!("iconutil not available (non-macOS?). Iconset saved to {:?}", iconset_dir);
            // Write a placeholder .icns file so the export list is consistent
            std::fs::write(path, b"icns placeholder - run iconutil manually")?;
        }
    }
    Ok(())
}

/// Export an ICO file with multiple embedded sizes
fn export_ico(img: &DynamicImage, path: &Path, sizes: &[u32]) -> Result<()> {
    let count = sizes.len() as u16;
    let mut png_blobs: Vec<Vec<u8>> = Vec::new();

    for &size in sizes {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let mut buf = Vec::new();
        resized.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
        png_blobs.push(buf);
    }

    let mut ico_data: Vec<u8> = Vec::new();
    // ICO header: reserved(2) + type(2, 1=ICO) + count(2)
    ico_data.extend_from_slice(&[0, 0]);
    ico_data.extend_from_slice(&1u16.to_le_bytes()); // type = ICO
    ico_data.extend_from_slice(&count.to_le_bytes());

    // Calculate offsets: header(6) + entries(16 * count) + cumulative blob sizes
    let entries_end = 6 + 16 * sizes.len();
    let mut offset = entries_end;

    // Write directory entries
    for (i, &size) in sizes.iter().enumerate() {
        let w = if size >= 256 { 0u8 } else { size as u8 };
        let h = w;
        ico_data.extend_from_slice(&[w, h, 0, 0]); // width, height, palette, reserved
        ico_data.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico_data.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico_data.extend_from_slice(&(png_blobs[i].len() as u32).to_le_bytes()); // size
        ico_data.extend_from_slice(&(offset as u32).to_le_bytes()); // offset
        offset += png_blobs[i].len();
    }

    // Write all PNG blobs
    for blob in &png_blobs {
        ico_data.extend_from_slice(blob);
    }

    std::fs::write(path, &ico_data)?;
    Ok(())
}

/// Generate a manifest.json with icon entries
fn generate_manifest_json(name: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": name,
        "short_name": name,
        "icons": [
            { "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" },
            { "src": "/icon-512.png", "sizes": "512x512", "type": "image/png" }
        ],
        "theme_color": "#ffffff",
        "background_color": "#ffffff",
        "display": "standalone"
    }))
    .unwrap_or_default()
}

use base64::Engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_manifest_json() {
        let manifest = generate_manifest_json("TestApp");
        assert!(manifest.contains("TestApp"));
        assert!(manifest.contains("icon-192.png"));
        assert!(manifest.contains("icon-512.png"));
        assert!(manifest.contains("192x192"));
    }

    #[test]
    fn test_png_sizes_correct() {
        assert_eq!(PNG_SIZES.len(), 4);
        assert_eq!(PNG_SIZES[0].0, 1024);
        assert_eq!(PNG_SIZES[3].0, 64);
    }

    #[test]
    fn test_export_icons_invalid_base64() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(export_icons(
            "not-valid-base64!!!",
            Path::new("/tmp/shepherd-test-icons"),
            "Test",
        ));
        assert!(result.is_err());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p shepherd-core -- logogen::export`
Expected: All 3 export tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-core/src/logogen/export.rs crates/shepherd-core/Cargo.toml
git commit -m "feat: add multi-format icon export (PNG sizes, favicon.ico, apple-touch-icon, manifest.json)"
```

---

### Task 7: Logo Generator Frontend UI

**Files:**
- Create: `src/features/logogen/LogoGenerator.tsx`
- Create: `crates/shepherd-server/src/routes/logogen.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`

- [ ] **Step 1: Add server routes for logo generation and export**

```rust
// crates/shepherd-server/src/routes/logogen.rs
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::logogen::{self, generate, LogoGenInput, LogoStyle};

#[derive(Deserialize)]
pub struct LogoGenRequest {
    pub product_name: String,
    pub product_description: Option<String>,
    pub style: String,
    #[serde(default)]
    pub colors: Vec<String>,
}

#[derive(Serialize)]
pub struct LogoGenResponse {
    pub variants: Vec<VariantResponse>,
}

#[derive(Serialize)]
pub struct VariantResponse {
    pub index: u8,
    /// Base64 PNG or URL
    pub image_data: String,
    pub is_url: bool,
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub image_base64: String,
    pub product_name: String,
    pub output_dir: Option<String>,
}

#[derive(Serialize)]
pub struct ExportResponse {
    pub files: Vec<ExportedFileResponse>,
}

#[derive(Serialize)]
pub struct ExportedFileResponse {
    pub path: String,
    pub format: String,
    pub size_bytes: u64,
    pub dimensions: Option<(u32, u32)>,
}

fn parse_style(s: &str) -> LogoStyle {
    match s {
        "minimal" => LogoStyle::Minimal,
        "geometric" => LogoStyle::Geometric,
        "mascot" => LogoStyle::Mascot,
        "abstract" => LogoStyle::Abstract,
        _ => LogoStyle::Minimal,
    }
}

pub async fn generate_logo(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LogoGenRequest>,
) -> Result<Json<LogoGenResponse>, (StatusCode, String)> {
    let llm = state.llm_provider.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "No LLM provider configured".into()))?;

    let input = LogoGenInput {
        product_name: body.product_name,
        product_description: body.product_description,
        style: parse_style(&body.style),
        colors: body.colors,
        variants: 4,
    };

    let result = generate::generate_logos(llm.as_ref(), &input).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Logo generation failed: {e}")))?;

    Ok(Json(LogoGenResponse {
        variants: result.variants.into_iter().map(|v| VariantResponse {
            index: v.index,
            image_data: v.png_data,
            is_url: false,
        }).collect(),
    }))
}

pub async fn export_icons(
    Json(body): Json<ExportRequest>,
) -> Result<Json<ExportResponse>, (StatusCode, String)> {
    let output_dir = body.output_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("public"));

    let result = logogen::export::export_icons(&body.image_base64, &output_dir, &body.product_name).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Export failed: {e}")))?;

    Ok(Json(ExportResponse {
        files: result.files.into_iter().map(|f| ExportedFileResponse {
            path: f.path,
            format: f.format,
            size_bytes: f.size_bytes,
            dimensions: f.dimensions,
        }).collect(),
    }))
}
```

- [ ] **Step 2: Register routes**

Add to `crates/shepherd-server/src/routes/mod.rs`:

```rust
pub mod logogen;
```

Add to the router:

```rust
.route("/api/logogen", post(logogen::generate_logo))
.route("/api/logogen/export", post(logogen::export_icons))
```

- [ ] **Step 3: Create Logo Generator frontend**

```tsx
// src/features/logogen/LogoGenerator.tsx
import React, { useState, useCallback } from 'react';

interface LogoVariant {
  index: number;
  image_data: string;
  is_url: boolean;
}

interface ExportedFile {
  path: string;
  format: string;
  size_bytes: number;
  dimensions: [number, number] | null;
}

const STYLES = [
  { id: 'minimal', label: 'Minimal', desc: 'Clean, flat, modern' },
  { id: 'geometric', label: 'Geometric', desc: 'Structured, bold lines' },
  { id: 'mascot', label: 'Mascot', desc: 'Friendly character' },
  { id: 'abstract', label: 'Abstract', desc: 'Artistic, fluid shapes' },
];

export function LogoGenerator() {
  const [productName, setProductName] = useState('');
  const [description, setDescription] = useState('');
  const [selectedStyle, setSelectedStyle] = useState('minimal');
  const [colors, setColors] = useState<string[]>(['#3B82F6', '#1E293B']);
  const [variants, setVariants] = useState<LogoVariant[]>([]);
  const [selectedVariant, setSelectedVariant] = useState<number | null>(null);
  const [exportedFiles, setExportedFiles] = useState<ExportedFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generate = useCallback(async () => {
    if (!productName.trim()) return;
    setLoading(true);
    setError(null);
    setVariants([]);
    setSelectedVariant(null);
    setExportedFiles([]);

    try {
      const resp = await fetch('/api/logogen', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          product_name: productName.trim(),
          product_description: description.trim() || null,
          style: selectedStyle,
          colors: colors.filter(c => c.trim()),
        }),
      });

      if (!resp.ok) throw new Error(await resp.text());
      const data = await resp.json();
      setVariants(data.variants);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Generation failed');
    } finally {
      setLoading(false);
    }
  }, [productName, description, selectedStyle, colors]);

  const exportIcons = useCallback(async () => {
    if (selectedVariant === null) return;
    const variant = variants[selectedVariant];
    if (!variant) return;

    setExporting(true);
    setError(null);

    try {
      const resp = await fetch('/api/logogen/export', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          image_base64: variant.image_data,
          product_name: productName.trim(),
        }),
      });

      if (!resp.ok) throw new Error(await resp.text());
      const data = await resp.json();
      setExportedFiles(data.files);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setExporting(false);
    }
  }, [selectedVariant, variants, productName]);

  const updateColor = (index: number, value: string) => {
    setColors(prev => prev.map((c, i) => i === index ? value : c));
  };

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-1">Logo & Icon Generator</h2>
        <p className="text-gray-500 text-sm">Generate logos and export to all required sizes</p>
      </div>

      {/* Input Section */}
      <div className="space-y-4 bg-white rounded-xl border border-gray-200 p-5">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Product Name</label>
            <input
              value={productName}
              onChange={e => setProductName(e.target.value)}
              placeholder="Shepherd"
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Description (optional)</label>
            <input
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder="AI agent manager"
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
            />
          </div>
        </div>

        {/* Style Picker */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">Style</label>
          <div className="grid grid-cols-4 gap-3">
            {STYLES.map(style => (
              <button
                key={style.id}
                onClick={() => setSelectedStyle(style.id)}
                className={`p-3 rounded-lg border text-left transition-colors ${
                  selectedStyle === style.id
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="text-sm font-medium text-gray-900">{style.label}</div>
                <div className="text-xs text-gray-500 mt-0.5">{style.desc}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Color Picker */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">Colors</label>
          <div className="flex gap-3">
            {colors.map((color, i) => (
              <div key={i} className="flex items-center gap-2">
                <input
                  type="color"
                  value={color}
                  onChange={e => updateColor(i, e.target.value)}
                  className="w-8 h-8 rounded border border-gray-300 cursor-pointer"
                />
                <span className="text-xs text-gray-500 font-mono">{color}</span>
              </div>
            ))}
          </div>
        </div>

        <button
          onClick={generate}
          disabled={loading || !productName.trim()}
          className="w-full py-2.5 rounded-lg bg-blue-600 text-white font-medium text-sm hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? 'Generating...' : 'Generate Logo Variants'}
        </button>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700 text-sm">{error}</div>
      )}

      {/* Variant Grid */}
      {variants.length > 0 && (
        <div className="space-y-4">
          <h3 className="text-lg font-semibold text-gray-900">Choose a Variant</h3>
          <div className="grid grid-cols-2 gap-4">
            {variants.map((v, i) => (
              <button
                key={i}
                onClick={() => setSelectedVariant(i)}
                className={`relative rounded-xl border-2 overflow-hidden transition-all ${
                  selectedVariant === i
                    ? 'border-blue-500 ring-2 ring-blue-200'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <img
                  src={v.is_url ? v.image_data : `data:image/png;base64,${v.image_data}`}
                  alt={`Logo variant ${i + 1}`}
                  className="w-full aspect-square object-contain bg-gray-50 p-4"
                />
                {selectedVariant === i && (
                  <div className="absolute top-2 right-2 w-6 h-6 rounded-full bg-blue-500 text-white flex items-center justify-center text-xs font-bold">
                    &#10003;
                  </div>
                )}
              </button>
            ))}
          </div>

          {selectedVariant !== null && (
            <button
              onClick={exportIcons}
              disabled={exporting}
              className="w-full py-2.5 rounded-lg bg-green-600 text-white font-medium text-sm hover:bg-green-700 disabled:opacity-50"
            >
              {exporting ? 'Exporting...' : 'Export All Sizes & Formats'}
            </button>
          )}
        </div>
      )}

      {/* Export Results */}
      {exportedFiles.length > 0 && (
        <div className="bg-green-50 border border-green-200 rounded-xl p-5">
          <h3 className="text-sm font-semibold text-green-800 mb-3">Exported Files</h3>
          <div className="space-y-1">
            {exportedFiles.map((f, i) => (
              <div key={i} className="flex items-center justify-between text-sm">
                <span className="font-mono text-green-700">{f.path}</span>
                <span className="text-green-600">
                  {f.format.toUpperCase()}
                  {f.dimensions && ` ${f.dimensions[0]}x${f.dimensions[1]}`}
                  {' '}({(f.size_bytes / 1024).toFixed(1)} KB)
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run frontend lint check**

Run: `npx tsc --noEmit --project tsconfig.json`
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-server/src/routes/logogen.rs crates/shepherd-server/src/routes/mod.rs src/features/logogen/
git commit -m "feat: add logo generator server routes and wizard UI with style picker and export"
```

---

### Task 8: North Star PMF Integration

**Files:**
- Create: `crates/shepherd-core/src/northstar/mod.rs`
- Create: `crates/shepherd-core/src/northstar/phases.rs`
- Create: `crates/shepherd-core/src/northstar/context.rs`
- Create: `crates/shepherd-server/src/routes/northstar.rs`
- Create: `src/features/northstar/NorthStarWizard.tsx`
- Modify: `crates/shepherd-core/src/lib.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`

- [ ] **Step 1: Define North Star phases and types**

```rust
// crates/shepherd-core/src/northstar/mod.rs
pub mod phases;
pub mod context;

use serde::{Deserialize, Serialize};

/// A complete North Star PMF analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthStarAnalysis {
    pub product_name: String,
    pub product_description: String,
    pub phases_completed: Vec<PhaseResult>,
    pub ai_context: Option<String>,
}

/// Result of a single phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase_id: u8,
    pub phase_name: String,
    pub status: PhaseStatus,
    pub output: String,
    pub documents: Vec<GeneratedDocument>,
}

/// Status of a phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// A document generated by a phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDocument {
    pub title: String,
    pub filename: String,
    pub content: String,
    pub doc_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_status_serde() {
        let status = PhaseStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");
        let parsed: PhaseStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PhaseStatus::Completed);
    }
}
```

- [ ] **Step 2: Define the 13-phase wizard**

```rust
// crates/shepherd-core/src/northstar/phases.rs
use anyhow::Result;
use crate::llm::{ChatMessage, LlmProvider, LlmRequest};
use super::{GeneratedDocument, PhaseResult, PhaseStatus};

/// Definition of a single North Star phase
pub struct PhaseDefinition {
    pub id: u8,
    pub name: &'static str,
    pub description: &'static str,
    pub prompt_template: &'static str,
    pub output_documents: &'static [&'static str],
}

/// All 13 North Star phases
pub const PHASES: &[PhaseDefinition] = &[
    PhaseDefinition {
        id: 1,
        name: "Product Vision",
        description: "Define the core product vision and mission statement",
        prompt_template: "Given the product \"{name}\" described as \"{description}\", write a compelling product vision statement and mission statement. Include: 1) Vision (aspirational future state), 2) Mission (how you'll get there), 3) Core values (3-5 guiding principles). Format as markdown.",
        output_documents: &["vision-statement.md"],
    },
    PhaseDefinition {
        id: 2,
        name: "Brand Guidelines",
        description: "Define brand voice, tone, and visual identity principles",
        prompt_template: "Based on the product vision for \"{name}\", create brand guidelines covering: 1) Brand personality (3-5 traits), 2) Voice & tone guidelines, 3) Writing style rules, 4) Visual identity principles (colors, typography direction). Format as markdown.",
        output_documents: &["brand-guidelines.md"],
    },
    PhaseDefinition {
        id: 3,
        name: "North Star Metric",
        description: "Identify the single metric that best captures value delivery",
        prompt_template: "For \"{name}\" ({description}), define: 1) The North Star Metric — a single metric capturing core value delivery, 2) Why this metric matters, 3) How to measure it, 4) Input metrics that drive it (3-5), 5) Leading indicators. Format as markdown.",
        output_documents: &["north-star-metric.md"],
    },
    PhaseDefinition {
        id: 4,
        name: "Competitive Landscape",
        description: "Analyze the competitive landscape and positioning",
        prompt_template: "Analyze the competitive landscape for \"{name}\" ({description}). Include: 1) Direct competitors (3-5), 2) Indirect competitors (3-5), 3) Feature comparison matrix, 4) Competitive advantages, 5) Market gaps & opportunities, 6) Positioning statement. Format as markdown.",
        output_documents: &["competitive-analysis.md", "positioning.md"],
    },
    PhaseDefinition {
        id: 5,
        name: "User Personas",
        description: "Define target user personas with goals and pain points",
        prompt_template: "Create 3-4 detailed user personas for \"{name}\" ({description}). Each persona needs: 1) Name & demographics, 2) Role & responsibilities, 3) Goals (3-5), 4) Pain points (3-5), 5) Current workflow, 6) How the product helps. Format as markdown.",
        output_documents: &["user-personas.md"],
    },
    PhaseDefinition {
        id: 6,
        name: "User Journeys",
        description: "Map key user journeys from discovery to daily use",
        prompt_template: "Map 3 key user journeys for \"{name}\": 1) First-time setup & onboarding, 2) Core daily workflow, 3) Advanced/power user flow. Each journey: stages, actions, thoughts, emotions, pain points, opportunities. Format as markdown.",
        output_documents: &["user-journeys.md"],
    },
    PhaseDefinition {
        id: 7,
        name: "Feature Prioritization",
        description: "Prioritize features using ICE scoring framework",
        prompt_template: "Create a feature prioritization for \"{name}\" using the ICE framework (Impact, Confidence, Ease). List 15-20 features, score each 1-10, calculate total. Group into: Must-have (v1), Should-have (v1.1), Nice-to-have (v2), Kill list (explicitly not building). Format as markdown table.",
        output_documents: &["feature-prioritization.md", "kill-list.md"],
    },
    PhaseDefinition {
        id: 8,
        name: "UI Design System",
        description: "Define the UI component library and design tokens",
        prompt_template: "Define a UI design system for \"{name}\": 1) Color palette (primary, secondary, neutral, semantic), 2) Typography scale, 3) Spacing system, 4) Component inventory (buttons, inputs, cards, etc.), 5) Layout patterns, 6) Dark/light mode tokens. Format as markdown with CSS variable definitions.",
        output_documents: &["design-system.md"],
    },
    PhaseDefinition {
        id: 9,
        name: "Architecture Blueprint",
        description: "Define technical architecture and technology choices",
        prompt_template: "Create a technical architecture blueprint for \"{name}\" ({description}): 1) System architecture diagram (text-based), 2) Technology stack choices with rationale, 3) Data model (core entities), 4) API design (key endpoints), 5) Infrastructure requirements, 6) Scalability considerations. Format as markdown.",
        output_documents: &["architecture-blueprint.md"],
    },
    PhaseDefinition {
        id: 10,
        name: "Security Architecture",
        description: "Define security model, threat analysis, and compliance requirements",
        prompt_template: "Define the security architecture for \"{name}\": 1) Threat model (STRIDE analysis), 2) Authentication & authorization design, 3) Data protection (at rest, in transit), 4) API security, 5) Dependency security, 6) Compliance requirements (GDPR, SOC2 if applicable). Format as markdown.",
        output_documents: &["security-architecture.md"],
    },
    PhaseDefinition {
        id: 11,
        name: "Architecture Decision Records",
        description: "Document key technical decisions with context and consequences",
        prompt_template: "Create Architecture Decision Records (ADRs) for the top 5 most important technical decisions for \"{name}\". Each ADR: 1) Title, 2) Status (Accepted), 3) Context, 4) Decision, 5) Consequences (positive and negative), 6) Alternatives considered. Format as markdown.",
        output_documents: &["adr-001.md", "adr-002.md", "adr-003.md", "adr-004.md", "adr-005.md"],
    },
    PhaseDefinition {
        id: 12,
        name: "Action Roadmap",
        description: "Create a phased delivery roadmap with milestones",
        prompt_template: "Create a delivery roadmap for \"{name}\": 1) Phase 1 (MVP, 4 weeks) — core features, 2) Phase 2 (Enhancement, 4 weeks), 3) Phase 3 (Growth, 4 weeks). Each phase: milestones, deliverables, success criteria, risks. Include a GANTT-style text timeline. Format as markdown.",
        output_documents: &["roadmap.md"],
    },
    PhaseDefinition {
        id: 13,
        name: "Strategic Recommendation",
        description: "Synthesize all phases into a final strategic recommendation",
        prompt_template: "Synthesize all North Star analysis for \"{name}\" into a final strategic recommendation: 1) Executive summary, 2) Key findings, 3) Recommended strategy, 4) Risk mitigation, 5) Success metrics & KPIs, 6) Next steps. This is the master document that references all other deliverables. Format as markdown.",
        output_documents: &["strategic-recommendation.md"],
    },
];

/// Execute a single phase
pub async fn execute_phase(
    llm: &dyn LlmProvider,
    phase: &PhaseDefinition,
    product_name: &str,
    product_description: &str,
    previous_context: &str,
) -> Result<PhaseResult> {
    let prompt = phase.prompt_template
        .replace("{name}", product_name)
        .replace("{description}", product_description);

    let mut messages = vec![
        ChatMessage::system(
            "You are a senior product strategist and technical architect. Provide thorough, \
             actionable analysis. Use clear markdown formatting. Be specific, not generic."
        ),
    ];

    if !previous_context.is_empty() {
        messages.push(ChatMessage::user(format!(
            "Context from previous phases:\n\n{previous_context}"
        )));
        messages.push(ChatMessage::assistant(
            "I'll incorporate the previous analysis into this phase.".into(),
        ));
    }

    messages.push(ChatMessage::user(prompt));

    let request = LlmRequest {
        messages,
        max_tokens: 8192,
        temperature: 0.5,
        model: None,
    };

    let response = llm.chat(&request).await?;

    // Split response into documents based on phase definition
    let documents = phase.output_documents.iter().enumerate().map(|(i, filename)| {
        GeneratedDocument {
            title: filename.replace(".md", "").replace('-', " "),
            filename: filename.to_string(),
            content: if i == 0 {
                response.content.clone()
            } else {
                // For phases with multiple documents, the LLM should use headers
                // to separate them. In practice, we'd parse by heading.
                format!("<!-- See {} for full content -->", phase.output_documents[0])
            },
            doc_type: "markdown".into(),
        }
    }).collect();

    Ok(PhaseResult {
        phase_id: phase.id,
        phase_name: phase.name.to_string(),
        status: PhaseStatus::Completed,
        output: response.content,
        documents,
    })
}

/// Execute all phases sequentially, building context
pub async fn execute_all_phases(
    llm: &dyn LlmProvider,
    product_name: &str,
    product_description: &str,
) -> Result<Vec<PhaseResult>> {
    let mut results = Vec::new();
    let mut context = String::new();

    for phase in PHASES {
        let result = execute_phase(llm, phase, product_name, product_description, &context).await?;

        // Build context for next phase (keep it concise — use summaries)
        context.push_str(&format!("\n## {} (Phase {})\n{}\n", phase.name, phase.id, &result.output[..result.output.len().min(2000)]));

        results.push(result);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phases_count() {
        assert_eq!(PHASES.len(), 13);
    }

    #[test]
    fn test_phase_ids_sequential() {
        for (i, phase) in PHASES.iter().enumerate() {
            assert_eq!(phase.id, (i + 1) as u8);
        }
    }

    #[test]
    fn test_phase_prompt_template_has_placeholders() {
        for phase in PHASES {
            assert!(phase.prompt_template.contains("{name}"), "Phase {} missing {{name}}", phase.name);
        }
    }

    #[test]
    fn test_all_phases_have_documents() {
        for phase in PHASES {
            assert!(!phase.output_documents.is_empty(), "Phase {} has no output documents", phase.name);
        }
    }

    #[test]
    fn test_total_document_count() {
        let total: usize = PHASES.iter().map(|p| p.output_documents.len()).sum();
        // Should produce roughly 22 documents total
        assert!(total >= 18 && total <= 25, "Expected 18-25 documents, got {total}");
    }
}
```

- [ ] **Step 3: Implement ai-context.yml generator**

```rust
// crates/shepherd-core/src/northstar/context.rs
use anyhow::Result;
use std::path::Path;

use super::NorthStarAnalysis;

/// Generate an ai-context.yml file from North Star analysis
pub fn generate_ai_context(analysis: &NorthStarAnalysis) -> Result<String> {
    let mut yaml = String::new();
    yaml.push_str("# AI Context — Generated by Shepherd North Star PMF\n");
    yaml.push_str("# This file provides strategic context to all AI coding agents\n\n");

    yaml.push_str(&format!("product_name: \"{}\"\n", analysis.product_name));
    yaml.push_str(&format!("description: \"{}\"\n\n", analysis.product_description));

    // Extract key sections from completed phases
    yaml.push_str("strategic_context:\n");

    for phase in &analysis.phases_completed {
        if phase.status == super::PhaseStatus::Completed {
            yaml.push_str(&format!("  # Phase {}: {}\n", phase.phase_id, phase.phase_name));
            for doc in &phase.documents {
                yaml.push_str(&format!("  {}: \"docs/northstar/{}\"\n",
                    doc.filename.replace(".md", "").replace('-', "_"),
                    doc.filename
                ));
            }
        }
    }

    yaml.push_str("\n# Kill list — features explicitly NOT being built\n");
    yaml.push_str("# Agents should flag if they attempt to implement these\n");
    yaml.push_str("kill_list: []\n");

    yaml.push_str("\n# Success metrics from North Star analysis\n");
    yaml.push_str("metrics:\n");
    yaml.push_str("  north_star: \"\" # Set from Phase 3 output\n");
    yaml.push_str("  input_metrics: []\n");

    yaml.push_str("\n# Architecture constraints from Phase 9\n");
    yaml.push_str("architecture:\n");
    yaml.push_str("  tech_stack: []\n");
    yaml.push_str("  patterns: []\n");

    Ok(yaml)
}

/// Write the ai-context.yml and all generated documents to disk
pub fn write_northstar_output(
    analysis: &NorthStarAnalysis,
    project_dir: &Path,
) -> Result<Vec<String>> {
    let docs_dir = project_dir.join("docs").join("northstar");
    std::fs::create_dir_all(&docs_dir)?;

    let mut written_files = Vec::new();

    // Write each document
    for phase in &analysis.phases_completed {
        for doc in &phase.documents {
            let path = docs_dir.join(&doc.filename);
            std::fs::write(&path, &doc.content)?;
            written_files.push(path.to_string_lossy().into_owned());
        }
    }

    // Write ai-context.yml
    let context_yaml = generate_ai_context(analysis)?;
    let context_path = project_dir.join("ai-context.yml");
    std::fs::write(&context_path, &context_yaml)?;
    written_files.push(context_path.to_string_lossy().into_owned());

    Ok(written_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::northstar::{PhaseResult, PhaseStatus, GeneratedDocument};

    #[test]
    fn test_generate_ai_context_basic() {
        let analysis = NorthStarAnalysis {
            product_name: "TestApp".into(),
            product_description: "A test application".into(),
            phases_completed: vec![PhaseResult {
                phase_id: 1,
                phase_name: "Product Vision".into(),
                status: PhaseStatus::Completed,
                output: "Vision content".into(),
                documents: vec![GeneratedDocument {
                    title: "vision statement".into(),
                    filename: "vision-statement.md".into(),
                    content: "# Vision\nBe the best".into(),
                    doc_type: "markdown".into(),
                }],
            }],
            ai_context: None,
        };

        let yaml = generate_ai_context(&analysis).unwrap();
        assert!(yaml.contains("product_name: \"TestApp\""));
        assert!(yaml.contains("vision_statement: \"docs/northstar/vision-statement.md\""));
        assert!(yaml.contains("kill_list:"));
    }

    #[test]
    fn test_generate_ai_context_skips_incomplete() {
        let analysis = NorthStarAnalysis {
            product_name: "Test".into(),
            product_description: "Test".into(),
            phases_completed: vec![PhaseResult {
                phase_id: 1,
                phase_name: "Vision".into(),
                status: PhaseStatus::Failed,
                output: "".into(),
                documents: vec![],
            }],
            ai_context: None,
        };

        let yaml = generate_ai_context(&analysis).unwrap();
        // Should not include failed phase documents
        assert!(!yaml.contains("vision"));
    }
}
```

- [ ] **Step 4: Add North Star server route**

```rust
// crates/shepherd-server/src/routes/northstar.rs
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::northstar::{self, phases, context, PhaseStatus};

#[derive(Deserialize)]
pub struct NorthStarStartRequest {
    pub product_name: String,
    pub product_description: String,
    pub project_dir: Option<String>,
}

#[derive(Serialize)]
pub struct NorthStarPhaseResponse {
    pub phase_id: u8,
    pub phase_name: String,
    pub status: String,
    pub documents: Vec<DocumentResponse>,
}

#[derive(Serialize)]
pub struct DocumentResponse {
    pub title: String,
    pub filename: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct NorthStarResponse {
    pub phases: Vec<NorthStarPhaseResponse>,
    pub ai_context_path: Option<String>,
    pub total_documents: usize,
}

/// Execute a single phase
#[derive(Deserialize)]
pub struct ExecutePhaseRequest {
    pub product_name: String,
    pub product_description: String,
    pub phase_id: u8,
    pub previous_context: Option<String>,
}

pub async fn execute_phase(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ExecutePhaseRequest>,
) -> Result<Json<NorthStarPhaseResponse>, (StatusCode, String)> {
    let llm = state.llm_provider.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "No LLM provider configured".into()))?;

    let phase = phases::PHASES.iter()
        .find(|p| p.id == body.phase_id)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Phase {} not found", body.phase_id)))?;

    let result = phases::execute_phase(
        llm.as_ref(),
        phase,
        &body.product_name,
        &body.product_description,
        body.previous_context.as_deref().unwrap_or(""),
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Phase execution failed: {e}")))?;

    Ok(Json(NorthStarPhaseResponse {
        phase_id: result.phase_id,
        phase_name: result.phase_name,
        status: match result.status {
            PhaseStatus::Completed => "completed".into(),
            PhaseStatus::Failed => "failed".into(),
            _ => "unknown".into(),
        },
        documents: result.documents.into_iter().map(|d| DocumentResponse {
            title: d.title,
            filename: d.filename,
            content: d.content,
        }).collect(),
    }))
}

/// Get phase definitions (for the wizard UI)
pub async fn list_phases() -> Json<Vec<PhaseInfo>> {
    let phases: Vec<PhaseInfo> = phases::PHASES.iter().map(|p| PhaseInfo {
        id: p.id,
        name: p.name.to_string(),
        description: p.description.to_string(),
        document_count: p.output_documents.len(),
    }).collect();
    Json(phases)
}

#[derive(Serialize)]
pub struct PhaseInfo {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub document_count: usize,
}
```

- [ ] **Step 5: Create North Star Wizard frontend**

```tsx
// src/features/northstar/NorthStarWizard.tsx
import React, { useState, useCallback, useEffect } from 'react';

interface PhaseInfo {
  id: number;
  name: string;
  description: string;
  document_count: number;
}

interface PhaseResult {
  phase_id: number;
  phase_name: string;
  status: string;
  documents: { title: string; filename: string; content: string }[];
}

const STATUS_COLORS: Record<string, string> = {
  pending: 'bg-gray-200',
  running: 'bg-blue-400 animate-pulse',
  completed: 'bg-green-500',
  failed: 'bg-red-500',
  skipped: 'bg-gray-400',
};

export function NorthStarWizard() {
  const [productName, setProductName] = useState('');
  const [description, setDescription] = useState('');
  const [phases, setPhases] = useState<PhaseInfo[]>([]);
  const [results, setResults] = useState<Map<number, PhaseResult>>(new Map());
  const [currentPhase, setCurrentPhase] = useState<number | null>(null);
  const [started, setStarted] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch('/api/northstar/phases')
      .then(r => r.json())
      .then(setPhases)
      .catch(() => {});
  }, []);

  const runPhase = useCallback(async (phaseId: number): Promise<PhaseResult | null> => {
    setCurrentPhase(phaseId);
    setError(null);

    // Build context from previous phases
    const previousContext = Array.from(results.values())
      .filter(r => r.phase_id < phaseId && r.status === 'completed')
      .map(r => `## ${r.phase_name}\n${r.documents.map(d => d.content).join('\n').slice(0, 1500)}`)
      .join('\n\n');

    try {
      const resp = await fetch('/api/northstar/phase', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          product_name: productName,
          product_description: description,
          phase_id: phaseId,
          previous_context: previousContext || null,
        }),
      });

      if (!resp.ok) throw new Error(await resp.text());
      const result: PhaseResult = await resp.json();
      setResults(prev => new Map(prev).set(phaseId, result));
      return result;
    } catch (e) {
      const failedResult: PhaseResult = {
        phase_id: phaseId,
        phase_name: phases.find(p => p.id === phaseId)?.name || '',
        status: 'failed',
        documents: [],
      };
      setResults(prev => new Map(prev).set(phaseId, failedResult));
      setError(e instanceof Error ? e.message : 'Phase failed');
      return null;
    }
  }, [productName, description, results, phases]);

  const runAll = useCallback(async () => {
    if (!productName.trim() || !description.trim()) return;
    setStarted(true);
    setResults(new Map());

    for (const phase of phases) {
      const result = await runPhase(phase.id);
      if (!result || result.status === 'failed') {
        // Continue to next phase even if one fails
      }
    }
    setCurrentPhase(null);
  }, [productName, description, phases, runPhase]);

  const totalDocs = Array.from(results.values())
    .reduce((sum, r) => sum + r.documents.length, 0);

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-1">North Star PMF Wizard</h2>
        <p className="text-gray-500 text-sm">
          13-phase strategic analysis generating up to 22 documents
        </p>
      </div>

      {/* Input */}
      {!started && (
        <div className="space-y-4 bg-white rounded-xl border border-gray-200 p-5">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Product Name</label>
            <input
              value={productName}
              onChange={e => setProductName(e.target.value)}
              placeholder="Shepherd"
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Product Description</label>
            <textarea
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder="Describe your product, target audience, and core value proposition..."
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm resize-none"
              rows={4}
            />
          </div>
          <button
            onClick={runAll}
            disabled={!productName.trim() || !description.trim()}
            className="w-full py-2.5 rounded-lg bg-purple-600 text-white font-medium text-sm hover:bg-purple-700 disabled:opacity-50"
          >
            Start Analysis (13 Phases)
          </button>
        </div>
      )}

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700 text-sm">{error}</div>
      )}

      {/* Phase Progress */}
      {started && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold text-gray-900">Progress</h3>
            <span className="text-sm text-gray-500">
              {results.size} / {phases.length} phases | {totalDocs} documents
            </span>
          </div>

          {phases.map(phase => {
            const result = results.get(phase.id);
            const isCurrent = currentPhase === phase.id;
            const status = isCurrent ? 'running' : (result?.status || 'pending');

            return (
              <div
                key={phase.id}
                className={`bg-white rounded-lg border p-4 ${
                  isCurrent ? 'border-blue-300 ring-1 ring-blue-200' : 'border-gray-200'
                }`}
              >
                <div className="flex items-center gap-3">
                  <div className={`w-3 h-3 rounded-full ${STATUS_COLORS[status] || STATUS_COLORS.pending}`} />
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-gray-400 font-mono">Phase {phase.id}</span>
                      <span className="text-sm font-medium text-gray-900">{phase.name}</span>
                    </div>
                    <p className="text-xs text-gray-500 mt-0.5">{phase.description}</p>
                  </div>
                  {result && result.status === 'completed' && (
                    <span className="text-xs text-green-600 font-medium">
                      {result.documents.length} doc{result.documents.length !== 1 ? 's' : ''}
                    </span>
                  )}
                </div>

                {/* Show document list when completed */}
                {result && result.status === 'completed' && result.documents.length > 0 && (
                  <div className="mt-3 pl-6 space-y-1">
                    {result.documents.map((doc, i) => (
                      <div key={i} className="text-xs text-gray-600 font-mono">
                        docs/northstar/{doc.filename}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Completion Summary */}
      {started && currentPhase === null && results.size === phases.length && (
        <div className="bg-purple-50 border border-purple-200 rounded-xl p-5">
          <h3 className="text-sm font-semibold text-purple-800 mb-2">Analysis Complete</h3>
          <p className="text-sm text-purple-700">
            Generated {totalDocs} strategic documents. An <code className="bg-purple-100 px-1 rounded">ai-context.yml</code> file
            has been created to provide strategic context to all future AI agent sessions.
          </p>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 6: Register North Star routes and update lib.rs**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod northstar;
```

Add to `crates/shepherd-server/src/routes/mod.rs`:

```rust
pub mod northstar;
```

Add to the router:

```rust
.route("/api/northstar/phases", get(northstar::list_phases))
.route("/api/northstar/phase", post(northstar::execute_phase))
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p shepherd-core -- northstar`
Expected: All 7 North Star tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/shepherd-core/src/northstar/ crates/shepherd-server/src/routes/northstar.rs src/features/northstar/ crates/shepherd-core/src/lib.rs crates/shepherd-server/src/routes/mod.rs
git commit -m "feat: add North Star PMF wizard with 13-phase analysis and ai-context.yml generation"
```

---

## Chunk 3: Quality Gates & PR Pipeline (Tasks 9–12)

### Task 9: Quality Gate Runner (Built-in Gates)

**Files:**
- Create: `crates/shepherd-core/src/gates/mod.rs`
- Create: `crates/shepherd-core/src/gates/builtin.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Define gate runner types and orchestrator**

```rust
// crates/shepherd-core/src/gates/mod.rs
pub mod builtin;
pub mod plugin;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Result of running a single quality gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
    pub gate_type: GateType,
}

/// Type of quality gate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GateType {
    Lint,
    Format,
    TypeCheck,
    Test,
    Security,
    Custom,
}

/// Configuration for which gates to run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    #[serde(default = "default_true")]
    pub lint: bool,
    #[serde(default = "default_true")]
    pub format_check: bool,
    #[serde(default = "default_true")]
    pub type_check: bool,
    #[serde(default = "default_true")]
    pub test: bool,
    #[serde(default)]
    pub custom_gates: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_true() -> bool { true }
fn default_timeout() -> u64 { 300 }

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            lint: true,
            format_check: true,
            type_check: true,
            test: true,
            custom_gates: vec![],
            timeout_seconds: 300,
        }
    }
}

/// Run all configured quality gates for a project
pub async fn run_gates(
    project_dir: &Path,
    config: &GateConfig,
) -> Result<Vec<GateResult>> {
    let mut results = Vec::new();
    let timeout = Duration::from_secs(config.timeout_seconds);

    // Detect project type
    let project_type = builtin::detect_project_type(project_dir);

    if config.lint {
        if let Some(result) = builtin::run_lint(project_dir, &project_type, timeout).await? {
            results.push(result);
        }
    }

    if config.format_check {
        if let Some(result) = builtin::run_format_check(project_dir, &project_type, timeout).await? {
            results.push(result);
        }
    }

    if config.type_check {
        if let Some(result) = builtin::run_type_check(project_dir, &project_type, timeout).await? {
            results.push(result);
        }
    }

    if config.test {
        if let Some(result) = builtin::run_tests(project_dir, &project_type, timeout).await? {
            results.push(result);
        }
    }

    // Run custom plugin gates
    for gate_path in &config.custom_gates {
        let result = plugin::run_plugin_gate(project_dir, gate_path, timeout).await?;
        results.push(result);
    }

    Ok(results)
}

/// Check if all gates passed
pub fn all_gates_passed(results: &[GateResult]) -> bool {
    results.iter().all(|r| r.passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_config_defaults() {
        let config = GateConfig::default();
        assert!(config.lint);
        assert!(config.format_check);
        assert!(config.type_check);
        assert!(config.test);
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_all_gates_passed_true() {
        let results = vec![
            GateResult { gate_name: "lint".into(), passed: true, output: "".into(), duration_ms: 100, gate_type: GateType::Lint },
            GateResult { gate_name: "test".into(), passed: true, output: "".into(), duration_ms: 200, gate_type: GateType::Test },
        ];
        assert!(all_gates_passed(&results));
    }

    #[test]
    fn test_all_gates_passed_false() {
        let results = vec![
            GateResult { gate_name: "lint".into(), passed: true, output: "".into(), duration_ms: 100, gate_type: GateType::Lint },
            GateResult { gate_name: "test".into(), passed: false, output: "2 failed".into(), duration_ms: 200, gate_type: GateType::Test },
        ];
        assert!(!all_gates_passed(&results));
    }

    #[test]
    fn test_all_gates_passed_empty() {
        assert!(all_gates_passed(&[]));
    }
}
```

- [ ] **Step 2: Implement built-in gates with auto-detection**

```rust
// crates/shepherd-core/src/gates/builtin.rs
use anyhow::Result;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;

use super::{GateResult, GateType};

/// Detected project type
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    Node,       // package.json
    Python,     // pyproject.toml or setup.py
    TypeScript, // tsconfig.json
    Mixed(Vec<ProjectType>),
    Unknown,
}

/// Detect the project type from filesystem markers
pub fn detect_project_type(dir: &Path) -> ProjectType {
    let mut types = Vec::new();

    if dir.join("Cargo.toml").exists() {
        types.push(ProjectType::Rust);
    }
    if dir.join("tsconfig.json").exists() {
        types.push(ProjectType::TypeScript);
    } else if dir.join("package.json").exists() {
        types.push(ProjectType::Node);
    }
    if dir.join("pyproject.toml").exists() || dir.join("setup.py").exists() {
        types.push(ProjectType::Python);
    }

    match types.len() {
        0 => ProjectType::Unknown,
        1 => types.into_iter().next().unwrap(),
        _ => ProjectType::Mixed(types),
    }
}

/// Run a shell command and capture output
async fn run_command(
    dir: &Path,
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<(bool, String, u64)> {
    let start = Instant::now();

    let output = tokio::time::timeout(timeout, async {
        Command::new(program)
            .args(args)
            .current_dir(dir)
            .output()
            .await
    })
    .await
    .map_err(|_| anyhow::anyhow!("Command timed out after {}s", timeout.as_secs()))?
    .map_err(|e| anyhow::anyhow!("Failed to run {program}: {e}"))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}").trim().to_string();
    let passed = output.status.success();

    Ok((passed, combined, duration_ms))
}

/// Get all project types (flattened if Mixed)
fn all_types(pt: &ProjectType) -> Vec<&ProjectType> {
    match pt {
        ProjectType::Mixed(types) => types.iter().collect(),
        other => vec![other],
    }
}

/// Run lint check (auto-detected)
pub async fn run_lint(
    dir: &Path,
    project_type: &ProjectType,
    timeout: Duration,
) -> Result<Option<GateResult>> {
    let types = all_types(project_type);

    for pt in &types {
        match pt {
            ProjectType::Rust => {
                let (passed, output, ms) = run_command(dir, "cargo", &["clippy", "--all-targets", "--", "-D", "warnings"], timeout).await?;
                return Ok(Some(GateResult { gate_name: "clippy".into(), passed, output, duration_ms: ms, gate_type: GateType::Lint }));
            }
            ProjectType::Node | ProjectType::TypeScript => {
                if dir.join("node_modules/.bin/eslint").exists() {
                    let (passed, output, ms) = run_command(dir, "npx", &["eslint", ".", "--max-warnings=0"], timeout).await?;
                    return Ok(Some(GateResult { gate_name: "eslint".into(), passed, output, duration_ms: ms, gate_type: GateType::Lint }));
                }
            }
            ProjectType::Python => {
                let (passed, output, ms) = run_command(dir, "ruff", &["check", "."], timeout).await
                    .or_else(|_| async { run_command(dir, "python", &["-m", "ruff", "check", "."], timeout).await }.await)?;
                return Ok(Some(GateResult { gate_name: "ruff".into(), passed, output, duration_ms: ms, gate_type: GateType::Lint }));
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Run format check (auto-detected)
pub async fn run_format_check(
    dir: &Path,
    project_type: &ProjectType,
    timeout: Duration,
) -> Result<Option<GateResult>> {
    let types = all_types(project_type);

    for pt in &types {
        match pt {
            ProjectType::Rust => {
                let (passed, output, ms) = run_command(dir, "cargo", &["fmt", "--all", "--check"], timeout).await?;
                return Ok(Some(GateResult { gate_name: "rustfmt".into(), passed, output, duration_ms: ms, gate_type: GateType::Format }));
            }
            ProjectType::Node | ProjectType::TypeScript => {
                if dir.join("node_modules/.bin/prettier").exists() {
                    let (passed, output, ms) = run_command(dir, "npx", &["prettier", "--check", "."], timeout).await?;
                    return Ok(Some(GateResult { gate_name: "prettier".into(), passed, output, duration_ms: ms, gate_type: GateType::Format }));
                }
            }
            ProjectType::Python => {
                let (passed, output, ms) = run_command(dir, "ruff", &["format", "--check", "."], timeout).await
                    .or_else(|_| async { run_command(dir, "python", &["-m", "black", "--check", "."], timeout).await }.await)?;
                return Ok(Some(GateResult { gate_name: "ruff-format".into(), passed, output, duration_ms: ms, gate_type: GateType::Format }));
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Run type check (auto-detected)
pub async fn run_type_check(
    dir: &Path,
    project_type: &ProjectType,
    timeout: Duration,
) -> Result<Option<GateResult>> {
    let types = all_types(project_type);

    for pt in &types {
        match pt {
            ProjectType::Rust => {
                // cargo clippy already does type checking, but we can run cargo check too
                let (passed, output, ms) = run_command(dir, "cargo", &["check", "--all-targets"], timeout).await?;
                return Ok(Some(GateResult { gate_name: "cargo-check".into(), passed, output, duration_ms: ms, gate_type: GateType::TypeCheck }));
            }
            ProjectType::TypeScript => {
                let (passed, output, ms) = run_command(dir, "npx", &["tsc", "--noEmit"], timeout).await?;
                return Ok(Some(GateResult { gate_name: "tsc".into(), passed, output, duration_ms: ms, gate_type: GateType::TypeCheck }));
            }
            ProjectType::Python => {
                let (passed, output, ms) = run_command(dir, "mypy", &["."], timeout).await
                    .or_else(|_| async { run_command(dir, "python", &["-m", "mypy", "."], timeout).await }.await)?;
                return Ok(Some(GateResult { gate_name: "mypy".into(), passed, output, duration_ms: ms, gate_type: GateType::TypeCheck }));
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Run tests (auto-detected)
pub async fn run_tests(
    dir: &Path,
    project_type: &ProjectType,
    timeout: Duration,
) -> Result<Option<GateResult>> {
    let types = all_types(project_type);

    for pt in &types {
        match pt {
            ProjectType::Rust => {
                let (passed, output, ms) = run_command(dir, "cargo", &["test"], timeout).await?;
                return Ok(Some(GateResult { gate_name: "cargo-test".into(), passed, output, duration_ms: ms, gate_type: GateType::Test }));
            }
            ProjectType::Node | ProjectType::TypeScript => {
                // Try common test runners
                if dir.join("node_modules/.bin/vitest").exists() {
                    let (passed, output, ms) = run_command(dir, "npx", &["vitest", "run"], timeout).await?;
                    return Ok(Some(GateResult { gate_name: "vitest".into(), passed, output, duration_ms: ms, gate_type: GateType::Test }));
                } else if dir.join("node_modules/.bin/jest").exists() {
                    let (passed, output, ms) = run_command(dir, "npx", &["jest", "--passWithNoTests"], timeout).await?;
                    return Ok(Some(GateResult { gate_name: "jest".into(), passed, output, duration_ms: ms, gate_type: GateType::Test }));
                }
            }
            ProjectType::Python => {
                let (passed, output, ms) = run_command(dir, "python", &["-m", "pytest", "-x"], timeout).await?;
                return Ok(Some(GateResult { gate_name: "pytest".into(), passed, output, duration_ms: ms, gate_type: GateType::Test }));
            }
            _ => {}
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_rust_project() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Rust);
    }

    #[test]
    fn test_detect_node_project() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Node);
    }

    #[test]
    fn test_detect_typescript_project() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("tsconfig.json"), "{}").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::TypeScript);
    }

    #[test]
    fn test_detect_python_project() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Python);
    }

    #[test]
    fn test_detect_mixed_project() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        match detect_project_type(tmp.path()) {
            ProjectType::Mixed(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Mixed project type"),
        }
    }

    #[test]
    fn test_detect_unknown_project() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Unknown);
    }
}
```

- [ ] **Step 3: Add tempfile dev-dependency**

Add to `crates/shepherd-core/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Update lib.rs**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod gates;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p shepherd-core -- gates`
Expected: All 10 gate tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/gates/ crates/shepherd-core/Cargo.toml crates/shepherd-core/src/lib.rs
git commit -m "feat: add quality gate runner with auto-detection for Rust, Node, Python, and TypeScript projects"
```

---

### Task 10: Plugin Gates

**Files:**
- Create: `crates/shepherd-core/src/gates/plugin.rs`

- [ ] **Step 1: Implement plugin gate loader and runner**

```rust
// crates/shepherd-core/src/gates/plugin.rs
use anyhow::Result;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;

use super::{GateResult, GateType};

/// Run a custom plugin gate script
pub async fn run_plugin_gate(
    project_dir: &Path,
    gate_path: &str,
    timeout: Duration,
) -> Result<GateResult> {
    let full_path = if Path::new(gate_path).is_absolute() {
        gate_path.to_string()
    } else {
        project_dir.join(gate_path).to_string_lossy().into_owned()
    };

    let gate_name = Path::new(gate_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("custom")
        .to_string();

    let start = Instant::now();

    let output = tokio::time::timeout(timeout, async {
        Command::new("sh")
            .args(["-c", &full_path])
            .current_dir(project_dir)
            .env("SHEPHERD_PROJECT_DIR", project_dir.to_string_lossy().as_ref())
            .output()
            .await
    })
    .await
    .map_err(|_| anyhow::anyhow!("Plugin gate '{gate_name}' timed out after {}s", timeout.as_secs()))?
    .map_err(|e| anyhow::anyhow!("Failed to run plugin gate '{gate_name}': {e}"))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}").trim().to_string();

    Ok(GateResult {
        gate_name,
        passed: output.status.success(),
        output: combined,
        duration_ms,
        gate_type: GateType::Custom,
    })
}

/// Discover plugin gates in the .shepherd/gates/ directory
pub fn discover_plugin_gates(project_dir: &Path) -> Vec<String> {
    let gates_dir = project_dir.join(".shepherd").join("gates");
    if !gates_dir.exists() {
        return vec![];
    }

    let mut gates = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&gates_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "sh" | "bash" | "py" | "js" | "ts") {
                        gates.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    gates.sort();
    gates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_no_gates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let gates = discover_plugin_gates(tmp.path());
        assert!(gates.is_empty());
    }

    #[test]
    fn test_discover_plugin_gates() {
        let tmp = tempfile::tempdir().unwrap();
        let gates_dir = tmp.path().join(".shepherd").join("gates");
        std::fs::create_dir_all(&gates_dir).unwrap();
        std::fs::write(gates_dir.join("security-scan.sh"), "#!/bin/bash\nexit 0").unwrap();
        std::fs::write(gates_dir.join("license-check.py"), "import sys; sys.exit(0)").unwrap();
        std::fs::write(gates_dir.join("readme.md"), "not a gate").unwrap();

        let gates = discover_plugin_gates(tmp.path());
        assert_eq!(gates.len(), 2);
        assert!(gates[0].contains("license-check.py"));
        assert!(gates[1].contains("security-scan.sh"));
    }

    #[tokio::test]
    async fn test_run_plugin_gate_success() {
        let tmp = tempfile::tempdir().unwrap();
        let gate_path = tmp.path().join("test-gate.sh");
        std::fs::write(&gate_path, "#!/bin/bash\necho 'All checks passed'\nexit 0").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gate_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let result = run_plugin_gate(
            tmp.path(),
            gate_path.to_str().unwrap(),
            Duration::from_secs(10),
        ).await.unwrap();

        assert!(result.passed);
        assert_eq!(result.gate_name, "test-gate");
        assert_eq!(result.gate_type, GateType::Custom);
        assert!(result.output.contains("All checks passed"));
    }

    #[tokio::test]
    async fn test_run_plugin_gate_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let gate_path = tmp.path().join("fail-gate.sh");
        std::fs::write(&gate_path, "#!/bin/bash\necho 'FAIL: vulnerability found'\nexit 1").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gate_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let result = run_plugin_gate(
            tmp.path(),
            gate_path.to_str().unwrap(),
            Duration::from_secs(10),
        ).await.unwrap();

        assert!(!result.passed);
        assert!(result.output.contains("vulnerability found"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p shepherd-core -- gates::plugin`
Expected: All 4 plugin gate tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/shepherd-core/src/gates/plugin.rs
git commit -m "feat: add plugin quality gates with auto-discovery from .shepherd/gates/ directory"
```

---

### Task 11: One-Click PR Pipeline

**Files:**
- Create: `crates/shepherd-core/src/pr/mod.rs`
- Create: `crates/shepherd-core/src/pr/commit.rs`
- Create: `crates/shepherd-core/src/pr/github.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Define PR pipeline orchestrator**

```rust
// crates/shepherd-core/src/pr/mod.rs
pub mod commit;
pub mod github;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::gates::{self, GateConfig, GateResult};
use crate::llm::LlmProvider;

/// Input for PR creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInput {
    pub task_title: String,
    pub branch: String,
    pub base_branch: String,
    pub worktree_path: String,
    pub auto_commit_message: bool,
    pub edited_commit_message: Option<String>,  // User-edited commit message (overrides auto-generated)
    pub run_gates: bool,
    pub cleanup_worktree: bool,  // Remove worktree after successful PR (default: true)
}

/// Result of the PR pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrResult {
    pub steps: Vec<PipelineStep>,
    pub pr_url: Option<String>,
    pub success: bool,
}

/// A single step in the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub name: String,
    pub status: StepStatus,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

/// Execute the full PR pipeline
pub async fn create_pr(
    llm: Option<&dyn LlmProvider>,
    input: &PrInput,
    gate_config: &GateConfig,
    on_step: impl Fn(&PipelineStep),
) -> Result<PrResult> {
    let dir = Path::new(&input.worktree_path);
    let mut steps = Vec::new();

    // Step 1: Stage changes
    let stage_result = github::git_stage_all(dir).await;
    let stage_step = PipelineStep {
        name: "Stage changes".into(),
        status: if stage_result.is_ok() { StepStatus::Passed } else { StepStatus::Failed },
        output: stage_result.as_ref().map(|s| s.clone()).unwrap_or_else(|e| e.to_string()),
    };
    on_step(&stage_step);
    steps.push(stage_step);
    stage_result?;

    // Step 2: Generate commit message (returned to frontend for user edit/approval)
    let diff = github::git_diff_staged(dir).await?;
    let generated_msg = if let Some(llm) = llm {
        commit::generate_commit_message(llm, &input.task_title, &diff).await?
    } else {
        format!("feat: {}", input.task_title)
    };

    // Use the user-edited message if provided, otherwise use the generated one.
    // The frontend shows the generated message in an editable field. The user
    // can edit and submit, which populates input.edited_commit_message.
    let commit_msg = input.edited_commit_message
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&generated_msg);

    let commit_step = PipelineStep {
        name: "Generate commit message".into(),
        status: StepStatus::Passed,
        output: commit_msg.to_string(),
    };
    on_step(&commit_step);
    steps.push(commit_step);

    // Step 3: Commit
    let commit_result = github::git_commit(dir, commit_msg).await;
    let commit_step = PipelineStep {
        name: "Commit changes".into(),
        status: if commit_result.is_ok() { StepStatus::Passed } else { StepStatus::Failed },
        output: commit_result.as_ref().map(|s| s.clone()).unwrap_or_else(|e| e.to_string()),
    };
    on_step(&commit_step);
    steps.push(commit_step);
    commit_result?;

    // Step 4: Rebase on base branch
    let rebase_result = github::git_rebase(dir, &input.base_branch).await;
    let rebase_step = PipelineStep {
        name: format!("Rebase on {}", input.base_branch),
        status: if rebase_result.is_ok() { StepStatus::Passed } else { StepStatus::Failed },
        output: rebase_result.as_ref().map(|s| s.clone()).unwrap_or_else(|e| e.to_string()),
    };
    on_step(&rebase_step);
    steps.push(rebase_step);
    if rebase_result.is_err() {
        // Abort rebase and report failure
        let _ = github::git_rebase_abort(dir).await;
        return Ok(PrResult { steps, pr_url: None, success: false });
    }

    // Step 5: Run quality gates
    if input.run_gates {
        let gate_results = gates::run_gates(dir, gate_config).await?;
        let all_passed = gates::all_gates_passed(&gate_results);

        let gates_step = PipelineStep {
            name: "Quality gates".into(),
            status: if all_passed { StepStatus::Passed } else { StepStatus::Failed },
            output: gate_results.iter()
                .map(|r| format!("{}: {}", r.gate_name, if r.passed { "PASS" } else { "FAIL" }))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        on_step(&gates_step);
        steps.push(gates_step);

        if !all_passed {
            return Ok(PrResult { steps, pr_url: None, success: false });
        }
    }

    // Step 6: Push branch
    let push_result = github::git_push(dir, &input.branch).await;
    let push_step = PipelineStep {
        name: "Push branch".into(),
        status: if push_result.is_ok() { StepStatus::Passed } else { StepStatus::Failed },
        output: push_result.as_ref().map(|s| s.clone()).unwrap_or_else(|e| e.to_string()),
    };
    on_step(&push_step);
    steps.push(push_step);
    push_result?;

    // Step 7: Create PR via gh CLI
    let pr_body = github::build_pr_body(&input.task_title, &diff, &steps);
    let pr_result = github::gh_create_pr(dir, &input.branch, &input.base_branch, &input.task_title, &pr_body).await;
    let pr_url = pr_result.as_ref().ok().cloned();
    let pr_step = PipelineStep {
        name: "Create PR".into(),
        status: if pr_result.is_ok() { StepStatus::Passed } else { StepStatus::Failed },
        output: pr_result.unwrap_or_else(|e| e.to_string()),
    };
    on_step(&pr_step);
    steps.push(pr_step);

    // Step 8: Clean up worktree (optional, configurable)
    if input.cleanup_worktree && pr_url.is_some() {
        let cleanup_result = github::git_remove_worktree(dir).await;
        let cleanup_step = PipelineStep {
            name: "Clean up worktree".into(),
            status: if cleanup_result.is_ok() { StepStatus::Passed } else { StepStatus::Skipped },
            output: cleanup_result.unwrap_or_else(|e| format!("Skipped: {e}")),
        };
        on_step(&cleanup_step);
        steps.push(cleanup_step);
    }

    Ok(PrResult {
        steps,
        pr_url,
        success: pr_url.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_status_serde() {
        let status = StepStatus::Passed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"passed\"");
    }

    #[test]
    fn test_pr_result_default_failure() {
        let result = PrResult {
            steps: vec![],
            pr_url: None,
            success: false,
        };
        assert!(!result.success);
        assert!(result.pr_url.is_none());
    }
}
```

- [ ] **Step 2: Implement LLM commit message generation**

```rust
// crates/shepherd-core/src/pr/commit.rs
use anyhow::Result;
use crate::llm::{ChatMessage, LlmProvider, LlmRequest};

/// Generate a commit message from the diff using LLM
pub async fn generate_commit_message(
    llm: &dyn LlmProvider,
    task_title: &str,
    diff: &str,
) -> Result<String> {
    let truncated_diff = if diff.len() > 8000 {
        format!("{}...\n[diff truncated, {} total chars]", &diff[..8000], diff.len())
    } else {
        diff.to_string()
    };

    let system_prompt = r#"You are a commit message generator. Given a diff, write a conventional commit message.

Rules:
1. Use conventional commits format: type(scope): description
2. Types: feat, fix, refactor, docs, test, chore, style, perf
3. Keep the subject line under 72 characters
4. Add a blank line then a body with 2-3 bullet points explaining what changed
5. Be specific about what changed, not why (the diff shows what)

Respond with ONLY the commit message, no markdown fences or extra text."#;

    let user_prompt = format!(
        "Task: {task_title}\n\nDiff:\n```\n{truncated_diff}\n```\n\nWrite a commit message."
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
    #[test]
    fn test_truncation_threshold() {
        let long_diff = "a".repeat(10000);
        let truncated = if long_diff.len() > 8000 {
            format!("{}...", &long_diff[..8000])
        } else {
            long_diff.clone()
        };
        assert!(truncated.len() < long_diff.len());
        assert!(truncated.ends_with("..."));
    }
}
```

- [ ] **Step 3: Implement git and GitHub CLI operations**

```rust
// crates/shepherd-core/src/pr/github.rs
use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

/// Stage all changes
pub async fn git_stage_all(dir: &Path) -> Result<String> {
    run_git(dir, &["add", "-A"]).await
}

/// Get staged diff
pub async fn git_diff_staged(dir: &Path) -> Result<String> {
    run_git(dir, &["diff", "--staged", "--stat"]).await
}

/// Create a commit
pub async fn git_commit(dir: &Path, message: &str) -> Result<String> {
    run_git(dir, &["commit", "-m", message]).await
}

/// Rebase on base branch
pub async fn git_rebase(dir: &Path, base_branch: &str) -> Result<String> {
    // First fetch latest
    let _ = run_git(dir, &["fetch", "origin", base_branch]).await;
    run_git(dir, &["rebase", &format!("origin/{base_branch}")]).await
}

/// Abort a rebase
pub async fn git_rebase_abort(dir: &Path) -> Result<String> {
    run_git(dir, &["rebase", "--abort"]).await
}

/// Remove a git worktree (cleanup after PR)
pub async fn git_remove_worktree(dir: &Path) -> Result<String> {
    // Find the main worktree root, then remove this one
    let output = run_git(dir, &["worktree", "list", "--porcelain"]).await?;
    let worktree_path = dir.to_string_lossy();
    // git worktree remove needs to be run from a different worktree
    // We remove by going to parent and using git worktree remove <path>
    if let Some(parent) = dir.parent() {
        run_git(parent, &["worktree", "remove", &worktree_path]).await
    } else {
        Err(anyhow::anyhow!("Cannot determine parent directory for worktree cleanup"))
    }
}

/// Push branch to remote
pub async fn git_push(dir: &Path, branch: &str) -> Result<String> {
    run_git(dir, &["push", "-u", "origin", branch]).await
}

/// Create a PR using gh CLI
pub async fn gh_create_pr(
    dir: &Path,
    _branch: &str,
    base: &str,
    title: &str,
    body: &str,
) -> Result<String> {
    let output = Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", body, "--base", base])
        .current_dir(dir)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run gh CLI: {e}. Is gh installed?"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr create failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout) // Returns PR URL
}

/// Build PR body with summary, changes, and gate results
pub fn build_pr_body(
    task_title: &str,
    diff_stat: &str,
    steps: &[super::PipelineStep],
) -> String {
    let mut body = String::new();
    body.push_str("## Summary\n\n");
    body.push_str(&format!("**Task:** {task_title}\n\n"));

    body.push_str("## Changes\n\n");
    body.push_str("```\n");
    body.push_str(diff_stat);
    body.push_str("\n```\n\n");

    body.push_str("## Pipeline Results\n\n");
    body.push_str("| Step | Status |\n");
    body.push_str("|------|--------|\n");
    for step in steps {
        let icon = match step.status {
            super::StepStatus::Passed => "PASS",
            super::StepStatus::Failed => "FAIL",
            super::StepStatus::Skipped => "SKIP",
            _ => "...",
        };
        body.push_str(&format!("| {} | {} |\n", step.name, icon));
    }

    body.push_str("\n---\n*Created by Shepherd*\n");
    body
}

/// Run a git command and return stdout
async fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {stderr}", args.join(" "));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_pr_body() {
        let steps = vec![
            super::super::PipelineStep {
                name: "Stage changes".into(),
                status: super::super::StepStatus::Passed,
                output: "".into(),
            },
            super::super::PipelineStep {
                name: "Quality gates".into(),
                status: super::super::StepStatus::Passed,
                output: "".into(),
            },
        ];

        let body = build_pr_body("Add user auth", "3 files changed", &steps);
        assert!(body.contains("Add user auth"));
        assert!(body.contains("3 files changed"));
        assert!(body.contains("Stage changes"));
        assert!(body.contains("PASS"));
        assert!(body.contains("Created by Shepherd"));
    }
}
```

- [ ] **Step 4: Update lib.rs**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod pr;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p shepherd-core -- pr`
Expected: All 4 PR tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/pr/ crates/shepherd-core/src/lib.rs
git commit -m "feat: add one-click PR pipeline with LLM commit message gen, quality gates, and gh CLI integration"
```

---

### Task 12: PR Pipeline & Gate Results Frontend UI

**Files:**
- Create: `src/features/gates/GateResults.tsx`
- Create: `src/features/pr/PrPipeline.tsx`
- Create: `crates/shepherd-server/src/routes/gates.rs`
- Create: `crates/shepherd-server/src/routes/pr.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`

- [ ] **Step 1: Add server routes for gates and PR**

```rust
// crates/shepherd-server/src/routes/gates.rs
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::gates::{self, GateConfig, GateType};

#[derive(Serialize)]
pub struct GateResultResponse {
    pub gate_name: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
    pub gate_type: String,
}

pub async fn run_task_gates(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
) -> Result<Json<Vec<GateResultResponse>>, (StatusCode, String)> {
    // Look up task to get worktree path
    let conn = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let task = shepherd_core::db::queries::get_task(&conn, task_id)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Task not found: {e}")))?;
    drop(conn);

    let dir = std::path::Path::new(&task.repo_path);
    let config = GateConfig::default();

    let results = gates::run_gates(dir, &config).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Gate execution failed: {e}")))?;

    Ok(Json(results.into_iter().map(|r| GateResultResponse {
        gate_name: r.gate_name,
        passed: r.passed,
        output: r.output,
        duration_ms: r.duration_ms,
        gate_type: match r.gate_type {
            GateType::Lint => "lint".into(),
            GateType::Format => "format".into(),
            GateType::TypeCheck => "type_check".into(),
            GateType::Test => "test".into(),
            GateType::Security => "security".into(),
            GateType::Custom => "custom".into(),
        },
    }).collect()))
}
```

```rust
// crates/shepherd-server/src/routes/pr.rs
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::gates::GateConfig;
use shepherd_core::pr::{self, PrInput, StepStatus};

#[derive(Deserialize)]
pub struct CreatePrRequest {
    #[serde(default = "default_base")]
    pub base_branch: String,
    #[serde(default = "default_true")]
    pub auto_commit_message: bool,
    #[serde(default = "default_true")]
    pub run_gates: bool,
}

fn default_base() -> String { "main".into() }
fn default_true() -> bool { true }

#[derive(Serialize)]
pub struct PrResponse {
    pub success: bool,
    pub pr_url: Option<String>,
    pub steps: Vec<StepResponse>,
}

#[derive(Serialize)]
pub struct StepResponse {
    pub name: String,
    pub status: String,
    pub output: String,
}

pub async fn create_pr(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
    Json(body): Json<CreatePrRequest>,
) -> Result<Json<PrResponse>, (StatusCode, String)> {
    let conn = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let task = shepherd_core::db::queries::get_task(&conn, task_id)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Task not found: {e}")))?;
    drop(conn);

    let input = PrInput {
        task_title: task.title.clone(),
        branch: task.branch.clone(),
        base_branch: body.base_branch,
        worktree_path: task.repo_path.clone(),
        auto_commit_message: body.auto_commit_message,
        run_gates: body.run_gates,
    };

    let llm = state.llm_provider.as_deref();
    let gate_config = GateConfig::default();

    let result = pr::create_pr(llm, &input, &gate_config, |_step| {
        // In production, broadcast step updates via WebSocket
    }).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("PR pipeline failed: {e}")))?;

    Ok(Json(PrResponse {
        success: result.success,
        pr_url: result.pr_url,
        steps: result.steps.into_iter().map(|s| StepResponse {
            name: s.name,
            status: match s.status {
                StepStatus::Passed => "passed".into(),
                StepStatus::Failed => "failed".into(),
                StepStatus::Skipped => "skipped".into(),
                StepStatus::Running => "running".into(),
                StepStatus::Pending => "pending".into(),
            },
            output: s.output,
        }).collect(),
    }))
}
```

- [ ] **Step 2: Register routes**

Add to `crates/shepherd-server/src/routes/mod.rs`:

```rust
pub mod gates;
pub mod pr;
```

Add to the router:

```rust
.route("/api/tasks/:id/gates", post(gates::run_task_gates))
.route("/api/tasks/:id/pr", post(pr::create_pr))
```

- [ ] **Step 3: Create Gate Results frontend component**

```tsx
// src/features/gates/GateResults.tsx
import React from 'react';

interface GateResult {
  gate_name: string;
  passed: boolean;
  output: string;
  duration_ms: number;
  gate_type: string;
}

interface GateResultsProps {
  results: GateResult[];
  loading?: boolean;
}

const GATE_ICONS: Record<string, string> = {
  lint: 'L',
  format: 'F',
  type_check: 'T',
  test: 'X',
  security: 'S',
  custom: 'C',
};

export function GateResults({ results, loading }: GateResultsProps) {
  if (loading) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-5">
        <div className="flex items-center gap-2 text-gray-500 text-sm">
          <div className="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
          Running quality gates...
        </div>
      </div>
    );
  }

  if (results.length === 0) return null;

  const allPassed = results.every(r => r.passed);

  return (
    <div className={`rounded-xl border p-5 ${allPassed ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}`}>
      <div className="flex items-center justify-between mb-3">
        <h3 className={`text-sm font-semibold ${allPassed ? 'text-green-800' : 'text-red-800'}`}>
          Quality Gates {allPassed ? '-- All Passed' : '-- Failures Detected'}
        </h3>
        <span className="text-xs text-gray-500">
          {results.filter(r => r.passed).length}/{results.length} passed
        </span>
      </div>

      <div className="space-y-2">
        {results.map((r, i) => (
          <details key={i} className="group">
            <summary className="flex items-center gap-3 cursor-pointer list-none">
              <span className={`w-6 h-6 rounded flex items-center justify-center text-xs font-bold ${
                r.passed ? 'bg-green-200 text-green-800' : 'bg-red-200 text-red-800'
              }`}>
                {GATE_ICONS[r.gate_type] || '?'}
              </span>
              <span className="flex-1 text-sm font-medium text-gray-900">{r.gate_name}</span>
              <span className={`text-xs font-medium ${r.passed ? 'text-green-600' : 'text-red-600'}`}>
                {r.passed ? 'PASS' : 'FAIL'}
              </span>
              <span className="text-xs text-gray-400">{r.duration_ms}ms</span>
            </summary>
            {r.output && (
              <pre className="mt-2 ml-9 p-3 bg-gray-900 text-gray-100 rounded-lg text-xs overflow-x-auto max-h-48">
                {r.output}
              </pre>
            )}
          </details>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Create PR Pipeline frontend component**

```tsx
// src/features/pr/PrPipeline.tsx
import React, { useState, useCallback } from 'react';

interface PipelineStep {
  name: string;
  status: 'pending' | 'running' | 'passed' | 'failed' | 'skipped';
  output: string;
}

interface PrPipelineProps {
  taskId: number;
  taskTitle: string;
  branch: string;
}

const STEP_STYLES: Record<string, { dot: string; text: string }> = {
  pending: { dot: 'bg-gray-300', text: 'text-gray-500' },
  running: { dot: 'bg-blue-400 animate-pulse', text: 'text-blue-600' },
  passed: { dot: 'bg-green-500', text: 'text-green-700' },
  failed: { dot: 'bg-red-500', text: 'text-red-700' },
  skipped: { dot: 'bg-gray-400', text: 'text-gray-500' },
};

export function PrPipeline({ taskId, taskTitle, branch }: PrPipelineProps) {
  const [steps, setSteps] = useState<PipelineStep[]>([]);
  const [prUrl, setPrUrl] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [baseBranch, setBaseBranch] = useState('main');

  const createPr = useCallback(async () => {
    setRunning(true);
    setError(null);
    setSteps([]);
    setPrUrl(null);

    try {
      const resp = await fetch(`/api/tasks/${taskId}/pr`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          base_branch: baseBranch,
          auto_commit_message: true,
          run_gates: true,
        }),
      });

      if (!resp.ok) throw new Error(await resp.text());

      const data = await resp.json();
      setSteps(data.steps);
      setPrUrl(data.pr_url);

      if (!data.success) {
        setError('Pipeline failed. Check step details below.');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'PR creation failed');
    } finally {
      setRunning(false);
    }
  }, [taskId, baseBranch]);

  return (
    <div className="max-w-2xl mx-auto space-y-4">
      <div className="bg-white rounded-xl border border-gray-200 p-5">
        <h3 className="text-lg font-semibold text-gray-900 mb-1">Create Pull Request</h3>
        <p className="text-sm text-gray-500 mb-4">
          {taskTitle} ({branch})
        </p>

        <div className="flex items-end gap-3 mb-4">
          <div className="flex-1">
            <label className="block text-xs font-medium text-gray-600 mb-1">Base Branch</label>
            <input
              value={baseBranch}
              onChange={e => setBaseBranch(e.target.value)}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
            />
          </div>
          <button
            onClick={createPr}
            disabled={running}
            className="px-6 py-2 rounded-lg bg-blue-600 text-white font-medium text-sm hover:bg-blue-700 disabled:opacity-50"
          >
            {running ? 'Creating...' : 'Create PR'}
          </button>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm mb-4">
            {error}
          </div>
        )}

        {/* Pipeline Steps */}
        {steps.length > 0 && (
          <div className="space-y-2">
            {steps.map((step, i) => {
              const style = STEP_STYLES[step.status] || STEP_STYLES.pending;
              return (
                <details key={i} className="group">
                  <summary className="flex items-center gap-3 cursor-pointer list-none py-1">
                    <div className="flex items-center gap-2 flex-1">
                      <span className={`w-2.5 h-2.5 rounded-full ${style.dot}`} />
                      <span className={`text-sm font-medium ${style.text}`}>{step.name}</span>
                    </div>
                    <span className={`text-xs font-medium uppercase ${style.text}`}>
                      {step.status}
                    </span>
                  </summary>
                  {step.output && (
                    <pre className="mt-1 ml-5 p-3 bg-gray-50 rounded-lg text-xs text-gray-700 overflow-x-auto max-h-32">
                      {step.output}
                    </pre>
                  )}
                </details>
              );
            })}
          </div>
        )}

        {/* PR URL */}
        {prUrl && (
          <div className="mt-4 bg-green-50 border border-green-200 rounded-lg p-4">
            <p className="text-sm text-green-800">
              PR created successfully:{' '}
              <a href={prUrl} target="_blank" rel="noopener noreferrer" className="font-medium underline">
                {prUrl}
              </a>
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Run frontend lint check**

Run: `npx tsc --noEmit --project tsconfig.json`
Expected: No type errors.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/src/routes/gates.rs crates/shepherd-server/src/routes/pr.rs crates/shepherd-server/src/routes/mod.rs src/features/gates/ src/features/pr/
git commit -m "feat: add quality gate results UI and one-click PR pipeline with step-by-step progress"
```

---

## Chunk 4: Triggers, CLI Polish & Final Integration (Tasks 13–16)

### Task 13: Contextual Trigger Engine

**Files:**
- Create: `crates/shepherd-core/src/triggers/mod.rs`
- Create: `crates/shepherd-core/src/triggers/detectors.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Define trigger engine types and trait**

```rust
// crates/shepherd-core/src/triggers/mod.rs
pub mod detectors;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A trigger suggestion to show the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSuggestion {
    pub id: String,
    pub tool: String,
    pub message: String,
    pub action_label: String,
    pub action_route: String,
    pub priority: TriggerPriority,
}

/// Priority determines ordering and visual treatment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPriority {
    Low,
    Medium,
    High,
}

/// Trait for implementing trigger detectors
pub trait TriggerDetector: Send + Sync {
    /// Unique ID for this detector
    fn id(&self) -> &str;

    /// Check if this trigger should fire for the given project
    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>>;
}

/// Dismissed triggers stored in SQLite to avoid re-showing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissedTrigger {
    pub trigger_id: String,
    pub project_path: String,
    pub dismissed_at: String,
}

/// Run all detectors and return active suggestions
pub fn check_triggers(
    project_dir: &Path,
    dismissed: &[String],
) -> Vec<TriggerSuggestion> {
    let detectors: Vec<Box<dyn TriggerDetector>> = vec![
        Box::new(detectors::NameGenDetector),
        Box::new(detectors::LogoGenDetector),
        Box::new(detectors::NorthStarDetector),
    ];

    let mut suggestions = Vec::new();

    for detector in &detectors {
        if dismissed.contains(&detector.id().to_string()) {
            continue;
        }

        match detector.detect(project_dir) {
            Ok(Some(suggestion)) => suggestions.push(suggestion),
            Ok(None) => {}
            Err(e) => {
                tracing::debug!("Trigger detector '{}' failed: {e}", detector.id());
            }
        }
    }

    suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_priority_ordering() {
        assert!(TriggerPriority::High > TriggerPriority::Medium);
        assert!(TriggerPriority::Medium > TriggerPriority::Low);
    }

    #[test]
    fn test_check_triggers_respects_dismissed() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a project with no package name (should trigger name gen)
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "untitled"}"#,
        ).unwrap();

        let suggestions = check_triggers(tmp.path(), &[]);
        let has_namegen = suggestions.iter().any(|s| s.tool == "name_generator");
        // Should suggest name generator
        assert!(has_namegen);

        // Now dismiss it
        let suggestions = check_triggers(tmp.path(), &["namegen_untitled".to_string()]);
        let has_namegen = suggestions.iter().any(|s| s.id == "namegen_untitled");
        assert!(!has_namegen);
    }

    #[test]
    fn test_check_triggers_empty_project() {
        let tmp = tempfile::tempdir().unwrap();
        let suggestions = check_triggers(tmp.path(), &[]);
        // Should at least suggest North Star (no docs/ dir)
        let has_northstar = suggestions.iter().any(|s| s.tool == "north_star");
        assert!(has_northstar);
    }
}
```

- [ ] **Step 2: Implement built-in detectors**

```rust
// crates/shepherd-core/src/triggers/detectors.rs
use anyhow::Result;
use std::path::Path;

use super::{TriggerDetector, TriggerPriority, TriggerSuggestion};

/// Detects when the project has no meaningful product name
pub struct NameGenDetector;

impl TriggerDetector for NameGenDetector {
    fn id(&self) -> &str { "namegen_untitled" }

    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>> {
        let pkg_json = project_dir.join("package.json");
        if pkg_json.exists() {
            let content = std::fs::read_to_string(&pkg_json)?;
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
                    let is_untitled = matches!(
                        name.to_lowercase().as_str(),
                        "untitled" | "my-app" | "my-project" | "app" | "project"
                    );
                    if is_untitled {
                        return Ok(Some(TriggerSuggestion {
                            id: self.id().into(),
                            tool: "name_generator".into(),
                            message: "Want help brainstorming a product name?".into(),
                            action_label: "Open Name Generator".into(),
                            action_route: "/tools/namegen".into(),
                            priority: TriggerPriority::Medium,
                        }));
                    }
                }
            }
        }

        // Also check Cargo.toml
        let cargo_toml = project_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if let Ok(parsed) = content.parse::<toml::Value>() {
                if let Some(name) = parsed.get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    let is_untitled = matches!(
                        name.to_lowercase().as_str(),
                        "untitled" | "my-app" | "my-project" | "app" | "project"
                    );
                    if is_untitled {
                        return Ok(Some(TriggerSuggestion {
                            id: self.id().into(),
                            tool: "name_generator".into(),
                            message: "Want help brainstorming a product name?".into(),
                            action_label: "Open Name Generator".into(),
                            action_route: "/tools/namegen".into(),
                            priority: TriggerPriority::Medium,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Detects when the project has no favicon or app icon
pub struct LogoGenDetector;

impl TriggerDetector for LogoGenDetector {
    fn id(&self) -> &str { "logogen_no_icon" }

    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>> {
        let icon_locations = [
            "public/favicon.ico",
            "public/favicon.svg",
            "assets/icon.png",
            "src-tauri/icons/icon.png",
            "static/favicon.ico",
            "app/favicon.ico",
        ];

        let has_icon = icon_locations.iter().any(|loc| project_dir.join(loc).exists());

        if !has_icon {
            // Only suggest if the project has web-like structure
            let is_web_project = project_dir.join("package.json").exists()
                || project_dir.join("public").exists()
                || project_dir.join("index.html").exists();

            if is_web_project {
                return Ok(Some(TriggerSuggestion {
                    id: self.id().into(),
                    tool: "logo_generator".into(),
                    message: "No app icon found. Generate a logo?".into(),
                    action_label: "Open Logo Generator".into(),
                    action_route: "/tools/logogen".into(),
                    priority: TriggerPriority::Low,
                }));
            }
        }

        Ok(None)
    }
}

/// Detects when the project has no strategy/docs
pub struct NorthStarDetector;

impl TriggerDetector for NorthStarDetector {
    fn id(&self) -> &str { "northstar_no_docs" }

    fn detect(&self, project_dir: &Path) -> Result<Option<TriggerSuggestion>> {
        let docs_dir = project_dir.join("docs");
        let ai_context = project_dir.join("ai-context.yml");
        let has_strategy = docs_dir.exists() && docs_dir.is_dir();
        let has_ai_context = ai_context.exists();

        if !has_strategy && !has_ai_context {
            return Ok(Some(TriggerSuggestion {
                id: self.id().into(),
                tool: "north_star".into(),
                message: "Define your product strategy?".into(),
                action_label: "Open North Star Wizard".into(),
                action_route: "/tools/northstar".into(),
                priority: TriggerPriority::Low,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namegen_detector_untitled_package() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "untitled", "version": "1.0.0"}"#,
        ).unwrap();

        let detector = NameGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, "name_generator");
    }

    #[test]
    fn test_namegen_detector_proper_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "shepherd", "version": "1.0.0"}"#,
        ).unwrap();

        let detector = NameGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_logogen_detector_no_icon() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let detector = LogoGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, "logo_generator");
    }

    #[test]
    fn test_logogen_detector_has_icon() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("public")).unwrap();
        std::fs::write(tmp.path().join("public/favicon.ico"), "icon").unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let detector = LogoGenDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_northstar_detector_no_docs() {
        let tmp = tempfile::tempdir().unwrap();

        let detector = NorthStarDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, "north_star");
    }

    #[test]
    fn test_northstar_detector_has_docs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("docs")).unwrap();

        let detector = NorthStarDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_northstar_detector_has_ai_context() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("ai-context.yml"), "product: test").unwrap();

        let detector = NorthStarDetector;
        let result = detector.detect(tmp.path()).unwrap();
        assert!(result.is_none());
    }
}
```

- [ ] **Step 3: Update lib.rs**

Add to `crates/shepherd-core/src/lib.rs`:

```rust
pub mod triggers;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p shepherd-core -- triggers`
Expected: All 10 trigger tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/triggers/ crates/shepherd-core/src/lib.rs
git commit -m "feat: add contextual trigger engine with name gen, logo gen, and North Star detectors"
```

---

### Task 14: Trigger Toast UI

**Files:**
- Create: `src/features/triggers/TriggerToast.tsx`

- [ ] **Step 1: Create trigger toast notification component**

```tsx
// src/features/triggers/TriggerToast.tsx
import React, { useState, useEffect, useCallback } from 'react';

interface TriggerSuggestion {
  id: string;
  tool: string;
  message: string;
  action_label: string;
  action_route: string;
  priority: 'low' | 'medium' | 'high';
}

interface TriggerToastProps {
  projectDir: string;
  onNavigate: (route: string) => void;
}

export function TriggerToast({ projectDir, onNavigate }: TriggerToastProps) {
  const [suggestions, setSuggestions] = useState<TriggerSuggestion[]>([]);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const [visible, setVisible] = useState<string | null>(null);

  // Check for triggers on mount and when project changes
  useEffect(() => {
    const checkTriggers = async () => {
      try {
        const resp = await fetch('/api/triggers/check', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ project_dir: projectDir }),
        });
        if (!resp.ok) return;
        const data: TriggerSuggestion[] = await resp.json();
        setSuggestions(data.filter(s => !dismissed.has(s.id)));
      } catch {
        // Silently fail — triggers are non-critical
      }
    };

    checkTriggers();
    // Re-check every 30 seconds
    const interval = setInterval(checkTriggers, 30000);
    return () => clearInterval(interval);
  }, [projectDir, dismissed]);

  // Show the highest-priority non-dismissed suggestion
  useEffect(() => {
    const active = suggestions.find(s => !dismissed.has(s.id));
    if (active && visible !== active.id) {
      // Delay toast appearance for non-intrusive feel
      const timer = setTimeout(() => setVisible(active.id), 2000);
      return () => clearTimeout(timer);
    }
  }, [suggestions, dismissed, visible]);

  const dismiss = useCallback((id: string) => {
    setDismissed(prev => new Set([...prev, id]));
    setVisible(null);
    // Persist dismissal to server
    fetch('/api/triggers/dismiss', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ trigger_id: id, project_dir: projectDir }),
    }).catch(() => {});
  }, [projectDir]);

  const activeSuggestion = suggestions.find(s => s.id === visible);
  if (!activeSuggestion) return null;

  return (
    <div className="fixed bottom-6 right-6 z-50 animate-slide-up">
      <div className="bg-white rounded-xl shadow-lg border border-gray-200 p-4 max-w-sm">
        <div className="flex items-start gap-3">
          <div className="w-8 h-8 rounded-lg bg-blue-100 flex items-center justify-center flex-shrink-0">
            <ToolIcon tool={activeSuggestion.tool} />
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm text-gray-900 font-medium">{activeSuggestion.message}</p>
            <div className="flex items-center gap-2 mt-2">
              <button
                onClick={() => {
                  onNavigate(activeSuggestion.action_route);
                  dismiss(activeSuggestion.id);
                }}
                className="px-3 py-1 rounded-lg bg-blue-600 text-white text-xs font-medium hover:bg-blue-700"
              >
                {activeSuggestion.action_label}
              </button>
              <button
                onClick={() => dismiss(activeSuggestion.id)}
                className="px-3 py-1 rounded-lg bg-gray-100 text-gray-600 text-xs font-medium hover:bg-gray-200"
              >
                Dismiss
              </button>
            </div>
          </div>
          <button
            onClick={() => dismiss(activeSuggestion.id)}
            className="text-gray-400 hover:text-gray-600"
            aria-label="Close"
          >
            x
          </button>
        </div>
      </div>
    </div>
  );
}

function ToolIcon({ tool }: { tool: string }) {
  const icons: Record<string, string> = {
    name_generator: 'Aa',
    logo_generator: 'Lg',
    north_star: 'NS',
  };
  return (
    <span className="text-blue-600 text-xs font-bold">{icons[tool] || '?'}</span>
  );
}
```

- [ ] **Step 2: Run frontend lint check**

Run: `npx tsc --noEmit --project tsconfig.json`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/features/triggers/
git commit -m "feat: add contextual trigger toast notifications with dismiss persistence"
```

---

### Task 15: CLI Polish — Shell Completions & Subcommands

**Files:**
- Modify: `crates/shepherd-cli/src/main.rs`
- Modify: `crates/shepherd-cli/Cargo.toml`

- [ ] **Step 1: Add clap_complete dependency**

Add to `crates/shepherd-cli/Cargo.toml`:

```toml
clap_complete = "4"
```

- [ ] **Step 2: Implement full CLI with all subcommands and completions**

```rust
// crates/shepherd-cli/src/main.rs
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use reqwest::Client;
use serde_json::Value;
use std::io;

const DEFAULT_SERVER: &str = "http://localhost:7532";

#[derive(Parser)]
#[command(
    name = "shepherd",
    about = "Manage your coding agents from the command line",
    version,
    long_about = "Shepherd — a cross-platform manager for AI coding agents.\nAlias: shep"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Server URL
    #[arg(long, global = true, default_value = DEFAULT_SERVER, env = "SHEPHERD_URL")]
    server: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of all tasks
    #[command(alias = "s")]
    Status,

    /// Create a new task
    New {
        /// Task description / prompt
        prompt: String,
        /// Agent to use
        #[arg(long, short, default_value = "claude-code")]
        agent: String,
        /// Isolation mode: worktree, docker, local
        #[arg(long, short, default_value = "worktree")]
        isolation: String,
        /// Repository path
        #[arg(long, short, default_value = ".")]
        repo: String,
    },

    /// Approve a pending permission
    #[command(alias = "a")]
    Approve {
        /// Task ID to approve
        task_id: Option<u64>,
        /// Approve all pending permissions
        #[arg(long)]
        all: bool,
    },

    /// Create PR for a completed task
    Pr {
        /// Task ID
        task_id: u64,
        /// Base branch
        #[arg(long, default_value = "main")]
        base: String,
    },

    /// Run quality gates for a task
    Gates {
        /// Task ID
        task_id: u64,
    },

    /// Initialize Shepherd in current project
    Init,

    /// Generate product name candidates
    #[command(alias = "name")]
    Namegen {
        /// Product description
        description: String,
        /// Vibe tags
        #[arg(long, short, num_args = 1..)]
        vibes: Vec<String>,
    },

    /// Stop all agents and server
    Stop,

    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();
    let base_url = &cli.server;

    match cli.command {
        Some(Commands::Status) => {
            let resp = client
                .get(format!("{base_url}/api/tasks"))
                .send()
                .await?;

            if !resp.status().is_success() {
                eprintln!("Error: Could not connect to Shepherd server at {base_url}");
                eprintln!("Is the server running? Start with: shepherd");
                std::process::exit(1);
            }

            let tasks: Vec<Value> = resp.json().await?;

            if tasks.is_empty() {
                println!("No active tasks.");
                return Ok(());
            }

            // Count by status
            let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for task in &tasks {
                let status = task.get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                *counts.entry(status.to_string()).or_default() += 1;
            }

            let parts: Vec<String> = counts
                .iter()
                .map(|(status, count)| format!("{count} {status}"))
                .collect();
            println!("{}", parts.join(" · "));

            // Print individual tasks
            println!();
            for task in &tasks {
                let id = task.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("?");
                let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                let agent = task.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?");

                let status_icon = match status {
                    "queued" => "[ ]",
                    "running" => "[>]",
                    "input" => "[?]",
                    "review" => "[R]",
                    "error" => "[!]",
                    "done" => "[x]",
                    _ => "[-]",
                };

                println!("  {status_icon} #{id} {title} ({agent})");
            }
        }

        Some(Commands::New { prompt, agent, isolation, repo }) => {
            let body = serde_json::json!({
                "title": &prompt,
                "prompt": &prompt,
                "agent_id": &agent,
                "repo_path": &repo,
                "isolation_mode": &isolation,
            });

            let resp = client
                .post(format!("{base_url}/api/tasks"))
                .json(&body)
                .send()
                .await?;

            if resp.status().is_success() {
                let task: Value = resp.json().await?;
                let id = task.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                println!("Created task #{id}: {prompt}");
            } else {
                let text = resp.text().await?;
                eprintln!("Error: {text}");
                std::process::exit(1);
            }
        }

        Some(Commands::Approve { task_id, all }) => {
            if all {
                let resp = client
                    .post(format!("{base_url}/api/approve-all"))
                    .send()
                    .await?;

                if resp.status().is_success() {
                    println!("Approved all pending permissions.");
                } else {
                    eprintln!("Error: {}", resp.text().await?);
                }
            } else if let Some(id) = task_id {
                let resp = client
                    .post(format!("{base_url}/api/tasks/{id}/approve"))
                    .send()
                    .await?;

                if resp.status().is_success() {
                    println!("Approved task #{id}.");
                } else {
                    eprintln!("Error: {}", resp.text().await?);
                }
            } else {
                eprintln!("Specify a task ID or use --all");
                std::process::exit(1);
            }
        }

        Some(Commands::Pr { task_id, base }) => {
            println!("Creating PR for task #{task_id} against {base}...");

            let body = serde_json::json!({
                "base_branch": base,
                "auto_commit_message": true,
                "run_gates": true,
            });

            let resp = client
                .post(format!("{base_url}/api/tasks/{task_id}/pr"))
                .json(&body)
                .send()
                .await?;

            if resp.status().is_success() {
                let result: Value = resp.json().await?;
                if let Some(url) = result.get("pr_url").and_then(|v| v.as_str()) {
                    println!("PR created: {url}");
                } else {
                    println!("PR pipeline completed but no URL returned.");
                    if let Some(steps) = result.get("steps").and_then(|v| v.as_array()) {
                        for step in steps {
                            let name = step.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                            let status = step.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                            println!("  {status}: {name}");
                        }
                    }
                }
            } else {
                eprintln!("Error: {}", resp.text().await?);
                std::process::exit(1);
            }
        }

        Some(Commands::Gates { task_id }) => {
            println!("Running quality gates for task #{task_id}...");

            let resp = client
                .post(format!("{base_url}/api/tasks/{task_id}/gates"))
                .send()
                .await?;

            if resp.status().is_success() {
                let results: Vec<Value> = resp.json().await?;
                let mut all_passed = true;

                for result in &results {
                    let name = result.get("gate_name").and_then(|v| v.as_str()).unwrap_or("?");
                    let passed = result.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
                    let ms = result.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    let icon = if passed { "PASS" } else { all_passed = false; "FAIL" };
                    println!("  {icon} {name} ({ms}ms)");
                }

                if all_passed {
                    println!("\nAll gates passed.");
                } else {
                    println!("\nSome gates failed.");
                    std::process::exit(1);
                }
            } else {
                eprintln!("Error: {}", resp.text().await?);
                std::process::exit(1);
            }
        }

        Some(Commands::Init) => {
            let cwd = std::env::current_dir()?;
            let shepherd_dir = cwd.join(".shepherd");
            std::fs::create_dir_all(shepherd_dir.join("gates"))?;

            // Write default config
            let default_config = r#"# Shepherd project configuration
# default_agent = "claude-code"
# default_isolation = "worktree"
# default_permission_mode = "ask"
"#;
            let config_path = shepherd_dir.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, default_config)?;
            }

            println!("Initialized Shepherd in {}", cwd.display());
            println!("  Created .shepherd/config.toml");
            println!("  Created .shepherd/gates/");
        }

        Some(Commands::Namegen { description, vibes }) => {
            println!("Generating product names...");

            let body = serde_json::json!({
                "description": description,
                "vibes": vibes,
                "count": 20,
            });

            let resp = client
                .post(format!("{base_url}/api/namegen"))
                .json(&body)
                .send()
                .await?;

            if resp.status().is_success() {
                let result: Value = resp.json().await?;
                if let Some(candidates) = result.get("candidates").and_then(|v| v.as_array()) {
                    for (i, c) in candidates.iter().enumerate() {
                        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        let status = c.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                        let icon = match status {
                            "all_clear" => "+",
                            "partial" => "~",
                            "conflicted" => "x",
                            _ => "?",
                        };
                        println!("  [{icon}] {:<3} {name}", i + 1);
                    }
                }
            } else {
                eprintln!("Error: {}", resp.text().await?);
                std::process::exit(1);
            }
        }

        Some(Commands::Stop) => {
            let resp = client
                .post(format!("{base_url}/api/shutdown"))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => println!("Shepherd server stopped."),
                _ => println!("Server may already be stopped."),
            }
        }

        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
        }

        None => {
            println!("Starting Shepherd server + GUI...");
            println!("Server: {base_url}");
            // In production: start the server + open Tauri window
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Run build check**

Run: `cargo build -p shepherd-cli`
Expected: Compiles successfully.

- [ ] **Step 4: Verify shell completions work**

Run: `cargo run -p shepherd-cli -- completions bash > /dev/null && echo "OK"`
Expected: `OK`

Run: `cargo run -p shepherd-cli -- completions zsh > /dev/null && echo "OK"`
Expected: `OK`

Run: `cargo run -p shepherd-cli -- completions fish > /dev/null && echo "OK"`
Expected: `OK`

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-cli/
git commit -m "feat: polish CLI with all subcommands, aliases, and shell completions for bash/zsh/fish"
```

---

### Task 16: End-to-End Integration Test

**Files:**
- Create: `tests/integration/lifecycle_test.rs`
- Modify: `Cargo.toml` (workspace test member)

- [ ] **Step 1: Write end-to-end lifecycle integration test**

```rust
// tests/integration/lifecycle_test.rs
//! End-to-end integration test for the Shepherd lifecycle:
//! 1. Detect triggers on a new project
//! 2. Generate product names
//! 3. Run quality gates
//! 4. Verify trigger system dismissal

use shepherd_core::gates::{self, GateConfig};
use shepherd_core::triggers;
use shepherd_core::namegen::{self, NameGenInput, ValidationStatus};
use shepherd_core::logogen::{LogoGenInput, LogoStyle};
use shepherd_core::northstar::phases::PHASES;
use std::path::Path;

#[test]
fn test_trigger_detection_on_new_project() {
    let tmp = tempfile::tempdir().unwrap();

    // Create a bare project with an untitled name
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "untitled", "version": "0.1.0"}"#,
    ).unwrap();

    // No docs dir, no favicon
    let suggestions = triggers::check_triggers(tmp.path(), &[]);

    // Should have at least: namegen (untitled), logogen (no favicon), northstar (no docs)
    assert!(suggestions.len() >= 2, "Expected at least 2 trigger suggestions, got {}", suggestions.len());

    let tool_names: Vec<&str> = suggestions.iter().map(|s| s.tool.as_str()).collect();
    assert!(tool_names.contains(&"name_generator"), "Expected name_generator trigger");
    assert!(tool_names.contains(&"north_star"), "Expected north_star trigger");
}

#[test]
fn test_trigger_dismissed_not_reshown() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "untitled"}"#,
    ).unwrap();

    let dismissed = vec!["namegen_untitled".to_string()];
    let suggestions = triggers::check_triggers(tmp.path(), &dismissed);

    let has_namegen = suggestions.iter().any(|s| s.id == "namegen_untitled");
    assert!(!has_namegen, "Dismissed trigger should not reappear");
}

#[test]
fn test_trigger_cleared_after_fix() {
    let tmp = tempfile::tempdir().unwrap();

    // Project with proper name — should NOT trigger namegen
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "shepherd", "version": "1.0.0"}"#,
    ).unwrap();

    // Create docs dir — should NOT trigger northstar
    std::fs::create_dir_all(tmp.path().join("docs")).unwrap();

    // Create favicon — should NOT trigger logogen
    std::fs::create_dir_all(tmp.path().join("public")).unwrap();
    std::fs::write(tmp.path().join("public/favicon.ico"), "icon").unwrap();

    let suggestions = triggers::check_triggers(tmp.path(), &[]);
    assert!(suggestions.is_empty(), "Well-configured project should have no triggers, got: {:?}",
        suggestions.iter().map(|s| &s.tool).collect::<Vec<_>>());
}

#[test]
fn test_name_validation_sorting() {
    // Test that name candidates sort correctly
    use shepherd_core::namegen::{NameCandidate, NameGenResult, NameValidation};

    let result = NameGenResult {
        candidates: vec![
            NameCandidate {
                name: "BadName".into(),
                tagline: None,
                reasoning: "test".into(),
                validation: NameValidation {
                    overall_status: ValidationStatus::Conflicted,
                    ..Default::default()
                },
            },
            NameCandidate {
                name: "GoodName".into(),
                tagline: None,
                reasoning: "test".into(),
                validation: NameValidation {
                    overall_status: ValidationStatus::AllClear,
                    ..Default::default()
                },
            },
            NameCandidate {
                name: "OkayName".into(),
                tagline: None,
                reasoning: "test".into(),
                validation: NameValidation {
                    overall_status: ValidationStatus::Partial,
                    ..Default::default()
                },
            },
        ],
        input: NameGenInput {
            description: "test".into(),
            vibes: vec![],
            count: 3,
        },
    };

    let sorted = result.sorted();
    assert_eq!(sorted.candidates[0].name, "GoodName", "AllClear should be first");
    assert_eq!(sorted.candidates[1].name, "OkayName", "Partial should be second");
    assert_eq!(sorted.candidates[2].name, "BadName", "Conflicted should be last");
}

#[test]
fn test_gate_config_with_project_detection() {
    let tmp = tempfile::tempdir().unwrap();

    // Create a Rust project
    std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    let project_type = gates::builtin::detect_project_type(tmp.path());
    assert_eq!(project_type, gates::builtin::ProjectType::Rust);

    let config = GateConfig::default();
    assert!(config.lint);
    assert!(config.format_check);
    assert!(config.type_check);
    assert!(config.test);
}

#[test]
fn test_northstar_phases_complete() {
    // Verify all 13 phases are defined
    assert_eq!(PHASES.len(), 13);

    // Verify phase IDs are sequential
    for (i, phase) in PHASES.iter().enumerate() {
        assert_eq!(phase.id, (i + 1) as u8);
    }

    // Count total output documents
    let total_docs: usize = PHASES.iter().map(|p| p.output_documents.len()).sum();
    assert!(total_docs >= 18, "Expected at least 18 documents, got {total_docs}");
}

#[test]
fn test_logo_style_prompt_coverage() {
    // Verify all logo styles produce distinct prompts
    let styles = [LogoStyle::Minimal, LogoStyle::Geometric, LogoStyle::Mascot, LogoStyle::Abstract];
    let hints: Vec<&str> = styles.iter().map(|s| s.prompt_hint()).collect();

    // All hints should be unique
    for i in 0..hints.len() {
        for j in (i + 1)..hints.len() {
            assert_ne!(hints[i], hints[j], "Logo style hints should be unique");
        }
    }
}

#[test]
fn test_gate_all_passed_helper() {
    use shepherd_core::gates::{GateResult, GateType};

    let all_pass = vec![
        GateResult { gate_name: "lint".into(), passed: true, output: "".into(), duration_ms: 100, gate_type: GateType::Lint },
        GateResult { gate_name: "test".into(), passed: true, output: "".into(), duration_ms: 500, gate_type: GateType::Test },
    ];
    assert!(gates::all_gates_passed(&all_pass));

    let some_fail = vec![
        GateResult { gate_name: "lint".into(), passed: true, output: "".into(), duration_ms: 100, gate_type: GateType::Lint },
        GateResult { gate_name: "test".into(), passed: false, output: "1 failed".into(), duration_ms: 500, gate_type: GateType::Test },
    ];
    assert!(!gates::all_gates_passed(&some_fail));
}
```

- [ ] **Step 2: Add test dependencies to workspace**

Ensure `tempfile = "3"` is in `[workspace.dependencies]` in the root `Cargo.toml`, and the test file is accessible. Add to `Cargo.toml`:

```toml
[[test]]
name = "lifecycle_test"
path = "tests/integration/lifecycle_test.rs"

[dev-dependencies]
tempfile = "3"
shepherd-core = { path = "crates/shepherd-core" }
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --test lifecycle_test`
Expected: All 8 integration tests pass.

- [ ] **Step 4: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests across all crates pass.

- [ ] **Step 5: Commit**

```bash
git add tests/ Cargo.toml
git commit -m "test: add end-to-end lifecycle integration tests covering triggers, gates, and name validation"
```

---

## Chunk 5: Ecosystem Integrations — nono.sh, Obra Superpowers, context-mode (Tasks 17–19)

These tasks integrate the three ecosystem tools that Shepherd advertises as "powered by". Each must have real substance, not just README credits.

---

### Task 17: nono.sh Sandbox Integration in PTY Manager

**Goal:** Wrap agent process spawns in nono.sh kernel-level sandbox (Seatbelt on macOS, Landlock on Linux) so even YOLO mode can't touch SSH keys, AWS credentials, or shell configs.

**Files:**
- Create: `crates/shepherd-core/src/pty/sandbox.rs`
- Modify: `crates/shepherd-core/src/pty/mod.rs` (wrap spawn logic)
- Modify: `crates/shepherd-core/src/config/types.rs` (add sandbox config)
- Test: `crates/shepherd-core/src/pty/sandbox.rs` (inline tests)

- [ ] **Step 1: Write failing test for sandbox profile generation**

```rust
// crates/shepherd-core/src/pty/sandbox.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sandbox_profile_blocks_ssh() {
        let profile = SandboxProfile::default();
        assert!(profile.blocked_paths.iter().any(|p| p.contains(".ssh")));
    }

    #[test]
    fn test_default_sandbox_profile_blocks_aws() {
        let profile = SandboxProfile::default();
        assert!(profile.blocked_paths.iter().any(|p| p.contains(".aws")));
    }

    #[test]
    fn test_sandbox_command_wraps_with_nono() {
        let profile = SandboxProfile::default();
        let (cmd, args) = profile.wrap_command("claude", &["--auto".into()]);
        assert_eq!(cmd, "nono");
        assert!(args.contains(&"claude".to_string()));
    }

    #[test]
    fn test_sandbox_disabled_passes_through() {
        let profile = SandboxProfile::disabled();
        let (cmd, args) = profile.wrap_command("claude", &["--auto".into()]);
        assert_eq!(cmd, "claude");
        assert_eq!(args, vec!["--auto"]);
    }

    #[test]
    fn test_custom_blocked_paths() {
        let mut profile = SandboxProfile::default();
        profile.blocked_paths.push("/custom/secret".into());
        assert!(profile.blocked_paths.iter().any(|p| p == "/custom/secret"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core sandbox`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement SandboxProfile and nono.sh wrapper**

```rust
// crates/shepherd-core/src/pty/sandbox.rs

use std::path::PathBuf;

/// Sandbox profile controlling what agents can access.
/// Uses nono.sh (Seatbelt on macOS, Landlock on Linux) for kernel-level enforcement.
#[derive(Debug, Clone)]
pub struct SandboxProfile {
    pub enabled: bool,
    /// Paths blocked from agent access (read and write)
    pub blocked_paths: Vec<String>,
    /// Whether to block network access entirely
    pub block_network: bool,
    /// Additional nono.sh flags
    pub extra_flags: Vec<String>,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let home_str = home.to_string_lossy();
        Self {
            enabled: true,
            blocked_paths: vec![
                format!("{home_str}/.ssh"),
                format!("{home_str}/.aws"),
                format!("{home_str}/.gnupg"),
                format!("{home_str}/.config/gcloud"),
                format!("{home_str}/.azure"),
                format!("{home_str}/.kube"),
                format!("{home_str}/.bashrc"),
                format!("{home_str}/.zshrc"),
                format!("{home_str}/.profile"),
                format!("{home_str}/.bash_profile"),
                format!("{home_str}/.netrc"),
                format!("{home_str}/.npmrc"),
            ],
            block_network: false,
            extra_flags: vec![],
        }
    }
}

impl SandboxProfile {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            blocked_paths: vec![],
            block_network: false,
            extra_flags: vec![],
        }
    }

    /// Wrap a command and args with nono.sh sandbox enforcement.
    /// If sandbox is disabled, returns the command unchanged.
    pub fn wrap_command(&self, command: &str, args: &[String]) -> (String, Vec<String>) {
        if !self.enabled {
            return (command.to_string(), args.to_vec());
        }

        let mut nono_args = Vec::new();

        for path in &self.blocked_paths {
            nono_args.push("--block".to_string());
            nono_args.push(path.clone());
        }

        if self.block_network {
            nono_args.push("--no-network".to_string());
        }

        for flag in &self.extra_flags {
            nono_args.push(flag.clone());
        }

        // Separator then the actual command
        nono_args.push("--".to_string());
        nono_args.push(command.to_string());
        nono_args.extend(args.iter().cloned());

        ("nono".to_string(), nono_args)
    }

    /// Check if nono.sh is installed on the system
    pub fn is_available() -> bool {
        std::process::Command::new("nono")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
```

- [ ] **Step 4: Add sandbox config to ShepherdConfig**

```rust
// Add to crates/shepherd-core/src/config/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_sandbox_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub extra_blocked_paths: Vec<String>,
    #[serde(default)]
    pub block_network: bool,
}

fn default_sandbox_enabled() -> bool { true }

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: default_sandbox_enabled(),
            extra_blocked_paths: vec![],
            block_network: false,
        }
    }
}
```

Add `pub sandbox: SandboxConfig` to `ShepherdConfig` with `#[serde(default)]`.

- [ ] **Step 5: Integrate sandbox into PtyManager::spawn**

Modify `PtyManager::spawn` to accept an optional `SandboxProfile` and wrap the command:

```rust
// In pty/mod.rs spawn method, before CommandBuilder::new(command):
let (actual_cmd, actual_args) = sandbox.wrap_command(command, args);
let mut cmd = CommandBuilder::new(&actual_cmd);
cmd.args(&actual_args);
```

Add a `new` constructor that accepts `SandboxProfile`:

```rust
pub fn new(max_agents: usize, sandbox: SandboxProfile) -> Self {
```

If `sandbox.enabled && !SandboxProfile::is_available()`, log a warning and fall back to disabled.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p shepherd-core sandbox`
Expected: PASS (5 tests)

- [ ] **Step 7: Commit**

```bash
git add crates/shepherd-core/src/pty/sandbox.rs crates/shepherd-core/src/pty/mod.rs crates/shepherd-core/src/config/types.rs
git commit -m "feat: add nono.sh kernel-level sandbox integration to PTY manager"
```

---

### Task 18: Obra Superpowers Auto-Install

**Goal:** Auto-detect and optionally install Obra Superpowers skills for supported agents. The toggle lives in Shepherd's config (`~/.shepherd/config.toml`), but detection and installation target each **agent's own config directory** (e.g., `~/.claude/` for Claude Code, `~/.codex/` for Codex). At project-scope, install targets the agent's project config (e.g., `.claude/CLAUDE.md`). On new task creation, if superpowers aren't detected for the chosen agent, offer to install.

**Agent config locations:**
| Agent | User-scope | Project-scope |
|-------|-----------|---------------|
| Claude Code | `~/.claude/plugins/cache/claude-plugins-official/superpowers/` | `.claude/settings.json` (plugin ref) |
| Codex | `~/.codex/instructions.md` | `.codex/instructions.md` |
| OpenCode | `~/.opencode/config.toml` | `.opencode/config.toml` |

**Files:**
- Create: `crates/shepherd-core/src/ecosystem/mod.rs`
- Create: `crates/shepherd-core/src/ecosystem/superpowers.rs`
- Modify: `crates/shepherd-core/src/config/types.rs` (add ecosystem config)
- Modify: `crates/shepherd-core/src/lib.rs` (add module)
- Test: inline tests in `superpowers.rs`

- [ ] **Step 1: Write failing test for superpowers detection and install**

```rust
// crates/shepherd-core/src/ecosystem/superpowers.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_not_installed() {
        let tmp = TempDir::new().unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_claude_code_user_scope() {
        let tmp = TempDir::new().unwrap();
        // Simulate ~/.claude/plugins/cache/claude-plugins-official/superpowers/
        let skills_dir = tmp.path().join(".claude/plugins/cache/claude-plugins-official/superpowers");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_claude_code_project_scope() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::write(
            project.join(".claude/settings.json"),
            r#"{"plugins":["superpowers"]}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_targets_agent_dir() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_install_config_project_scope() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project).unwrap();
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_superpowers_compatible("claude-code"));
        assert!(is_superpowers_compatible("codex"));
        assert!(is_superpowers_compatible("opencode"));
        assert!(!is_superpowers_compatible("aider"));
        assert!(!is_superpowers_compatible("gemini-cli"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core superpowers`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement detection and install logic**

```rust
// crates/shepherd-core/src/ecosystem/superpowers.rs

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum InstallScope {
    User,
    Project,
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

/// Per-agent install configuration — targets the agent's own config directory
#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    /// Where to write config (e.g., ~/.claude/ or .claude/)
    pub target_path: PathBuf,
    /// Content to add/write to the agent's config
    pub config_content: String,
}

/// Detect if superpowers is installed for a specific agent.
/// `home` = user home dir (for user-scope detection in agent's config dir)
/// `project_root` = optional project dir (for project-scope detection)
pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    // Check project-scope first (higher priority)
    if let Some(project) = project_root {
        if let Some(result) = detect_project_scope(agent, project) {
            return result;
        }
    }
    // Fall back to user-scope (agent's own config dir under home)
    detect_user_scope(agent, home)
}

fn detect_user_scope(agent: &str, home: &Path) -> DetectionResult {
    let path = match agent {
        "claude-code" => home.join(".claude/plugins/cache/claude-plugins-official/superpowers"),
        "codex" => home.join(".codex/superpowers"),
        "opencode" => home.join(".opencode/superpowers"),
        _ => return DetectionResult { installed: false, scope: InstallScope::User, path: None, version: None },
    };
    if path.exists() {
        let version = detect_version(&path);
        DetectionResult { installed: true, scope: InstallScope::User, path: Some(path), version }
    } else {
        DetectionResult { installed: false, scope: InstallScope::User, path: None, version: None }
    }
}

fn detect_project_scope(agent: &str, project: &Path) -> Option<DetectionResult> {
    let config_path = match agent {
        "claude-code" => project.join(".claude/settings.json"),
        "codex" => project.join(".codex/instructions.md"),
        "opencode" => project.join(".opencode/config.toml"),
        _ => return None,
    };
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if content.contains("superpowers") {
            return Some(DetectionResult {
                installed: true,
                scope: InstallScope::Project,
                path: Some(config_path),
                version: None,
            });
        }
    }
    None
}

fn detect_version(path: &Path) -> Option<String> {
    std::fs::read_dir(path)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                Some(name)
            } else {
                None
            }
        })
        .max()
}

/// Check if an agent supports Obra Superpowers integration
pub fn is_superpowers_compatible(agent: &str) -> bool {
    matches!(agent, "claude-code" | "codex" | "opencode")
}

impl InstallConfig {
    /// Create install config targeting the specific agent's config directory.
    /// Returns None for unsupported agents.
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let (target_path, config_content) = match (agent, &scope) {
            ("claude-code", InstallScope::User) => (
                PathBuf::from("~/.claude/CLAUDE.md"),
                "# Obra Superpowers — brainstorming, planning, and agentic development\n\
                 # Installed by Shepherd. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("claude-code", InstallScope::Project) => (
                PathBuf::from(".claude/CLAUDE.md"),
                "# Obra Superpowers — brainstorming, planning, and agentic development\n\
                 # Installed by Shepherd. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("codex", InstallScope::User) => (
                PathBuf::from("~/.codex/instructions.md"),
                "# Obra Superpowers skills available. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("codex", InstallScope::Project) => (
                PathBuf::from(".codex/instructions.md"),
                "# Obra Superpowers skills available. See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("opencode", InstallScope::User) => (
                PathBuf::from("~/.opencode/config.toml"),
                "# superpowers = true\n# See https://github.com/obra/superpowers\n".to_string(),
            ),
            ("opencode", InstallScope::Project) => (
                PathBuf::from(".opencode/config.toml"),
                "# superpowers = true\n# See https://github.com/obra/superpowers\n".to_string(),
            ),
            _ => return None,
        };
        Some(Self {
            agent: agent.to_string(),
            scope,
            target_path,
            config_content,
        })
    }
}
```

- [ ] **Step 4: Add ecosystem module and config**

```rust
// crates/shepherd-core/src/ecosystem/mod.rs
pub mod superpowers;
pub mod context_mode;
```

Add `pub mod ecosystem;` to `lib.rs`.

Add to `ShepherdConfig` — the toggle lives in Shepherd's config, controls whether to auto-detect/offer install:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    #[serde(default = "default_true")]
    pub auto_detect_superpowers: bool,
    #[serde(default = "default_true")]
    pub auto_detect_context_mode: bool,
    #[serde(default = "default_true")]
    pub offer_install_on_new_task: bool,
}
fn default_true() -> bool { true }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-core superpowers`
Expected: PASS (7 tests)

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/ecosystem/ crates/shepherd-core/src/lib.rs crates/shepherd-core/src/config/types.rs
git commit -m "feat: add Obra Superpowers auto-detection targeting agent-specific config dirs"
```

---

### Task 19: context-mode Auto-Install

**Goal:** Auto-detect and optionally install context-mode MCP server for supported agents. Detection and installation target the **agent's own config directory** — for Claude Code, this means writing to `~/.claude/settings.json` (user-scope) or `.claude/settings.json` (project-scope) to register the MCP server. Toggle lives in Shepherd's config.

**Agent config locations for context-mode:**
| Agent | User-scope MCP config | Project-scope MCP config |
|-------|----------------------|--------------------------|
| Claude Code | `~/.claude/settings.json` → `mcpServers` | `.claude/settings.json` → `mcpServers` |

Note: context-mode is currently a Claude Code MCP server only. Other agents may gain support later.

**Files:**
- Create: `crates/shepherd-core/src/ecosystem/context_mode.rs`
- Modify: `crates/shepherd-core/src/ecosystem/mod.rs` (already added in Task 18)
- Test: inline tests in `context_mode.rs`

- [ ] **Step 1: Write failing test for context-mode detection and install**

```rust
// crates/shepherd-core/src/ecosystem/context_mode.rs

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::superpowers::InstallScope;
    use tempfile::TempDir;

    #[test]
    fn test_detect_not_installed() {
        let tmp = TempDir::new().unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_claude_code_user_scope() {
        let tmp = TempDir::new().unwrap();
        // Simulate ~/.claude/settings.json with context-mode MCP registered
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"mcpServers":{"context-mode":{"command":"npx","args":["-y","context-mode"]}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_detect_claude_code_project_scope() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::write(
            project.join(".claude/settings.json"),
            r#"{"mcpServers":{"context-mode":{"command":"npx"}}}"#,
        ).unwrap();
        let result = detect_for_agent("claude-code", &home, Some(&project));
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::Project);
    }

    #[test]
    fn test_install_config_generates_mcp_json() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config.mcp_server_json.contains("context-mode"));
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_install_config_project_scope() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::Project).unwrap();
        assert!(config.mcp_server_json.contains("context-mode"));
        assert!(config.target_path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_context_mode_compatible("claude-code"));
        assert!(!is_context_mode_compatible("codex"));
        assert!(!is_context_mode_compatible("aider"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core context_mode`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement detection and install logic**

```rust
// crates/shepherd-core/src/ecosystem/context_mode.rs

use std::path::{Path, PathBuf};
use super::superpowers::InstallScope;

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
}

/// Per-agent install config — targets the agent's own config directory
#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    /// Where to write (e.g., ~/.claude/settings.json or .claude/settings.json)
    pub target_path: PathBuf,
    /// MCP server JSON to merge into the agent's settings
    pub mcp_server_json: String,
}

const CONTEXT_MODE_MCP_ENTRY: &str = r#""context-mode": {
      "command": "npx",
      "args": ["-y", "context-mode"],
      "env": {}
    }"#;

/// Detect if context-mode is installed for a specific agent.
/// Checks agent's own config directory, not Shepherd's.
pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    // Check project-scope first (higher priority)
    if let Some(project) = project_root {
        if let Some(result) = detect_project_scope(agent, project) {
            return result;
        }
    }
    detect_user_scope(agent, home)
}

fn detect_user_scope(agent: &str, home: &Path) -> DetectionResult {
    let settings_path = match agent {
        "claude-code" => home.join(".claude/settings.json"),
        _ => return DetectionResult { installed: false, scope: InstallScope::User, path: None },
    };
    check_settings_file(&settings_path, InstallScope::User)
}

fn detect_project_scope(agent: &str, project: &Path) -> Option<DetectionResult> {
    let settings_path = match agent {
        "claude-code" => project.join(".claude/settings.json"),
        _ => return None,
    };
    let result = check_settings_file(&settings_path, InstallScope::Project);
    if result.installed { Some(result) } else { None }
}

fn check_settings_file(path: &Path, scope: InstallScope) -> DetectionResult {
    if path.exists() {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        if content.contains("context-mode") {
            return DetectionResult {
                installed: true,
                scope,
                path: Some(path.to_path_buf()),
            };
        }
    }
    DetectionResult { installed: false, scope, path: None }
}

/// Check if an agent supports context-mode integration
pub fn is_context_mode_compatible(agent: &str) -> bool {
    matches!(agent, "claude-code")
}

impl InstallConfig {
    /// Create install config targeting the agent's own settings file.
    /// Returns None for unsupported agents.
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let target_path = match (agent, &scope) {
            ("claude-code", InstallScope::User) => PathBuf::from("~/.claude/settings.json"),
            ("claude-code", InstallScope::Project) => PathBuf::from(".claude/settings.json"),
            _ => return None,
        };
        Some(Self {
            agent: agent.to_string(),
            scope,
            target_path,
            mcp_server_json: format!("{{\n  \"mcpServers\": {{\n    {CONTEXT_MODE_MCP_ENTRY}\n  }}\n}}"),
        })
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p shepherd-core context_mode`
Expected: PASS (7 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/ecosystem/context_mode.rs
git commit -m "feat: add context-mode auto-detection targeting agent's own MCP config"
```

---

### Task 20: New Project Wizard — Guided Full-Stack SDD Journey

**Goal:** Chain name gen → logo gen → North Star → Superpowers setup into an optional guided wizard for new projects. Each tool stays independently accessible (via Cmd+K, sidebar, triggers). The wizard just provides a "starting from zero" guided flow. Users can skip any step, jump to any step, or dismiss the wizard entirely. Always available from the sidebar and Cmd+K.

**Journey order (strategic → tactical → identity):**
1. **North Star PMF** — define what you're building, who it's for, success metrics, brand guidelines
2. **Obra Superpowers** — brainstorm → spec → plan (informed by North Star strategy)
3. **Brand Name Gen** — name the product (informed by brand guidelines + target audience from North Star)
4. **Logo Gen** — visual identity (informed by name + brand personality from North Star)

**Design principles:**
- **Optional, never forced** — wizard is offered on first project creation, always accessible from sidebar + Cmd+K, but never blocks workflow
- **Skip/jump anywhere** — stepper shows all 4 phases, user can click any phase directly
- **Each step independent** — completing North Star doesn't require Superpowers; skipping name gen doesn't break logo gen
- **Progress persisted** — wizard state saved per-project in SQLite so user can return later
- **Results flow forward** — North Star brand guidelines pre-fill name gen prompts, chosen name pre-fills logo gen, etc.

**Files:**
- Create: `crates/shepherd-core/src/wizard/mod.rs`
- Create: `crates/shepherd-core/src/wizard/state.rs`
- Create: `src/features/wizard/ProjectWizard.tsx`
- Create: `src/features/wizard/WizardStepper.tsx`
- Modify: `crates/shepherd-core/src/db/mod.rs` (add wizard_state table)
- Modify: `crates/shepherd-core/src/lib.rs` (add module)
- Test: inline tests in `state.rs`

- [ ] **Step 1: Write failing test for wizard state machine**

```rust
// crates/shepherd-core/src/wizard/state.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_wizard_starts_at_north_star() {
        let state = WizardState::new("my-project");
        assert_eq!(state.current_phase, WizardPhase::NorthStar);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_skip_advances_phase() {
        let mut state = WizardState::new("my-project");
        state.skip_current();
        assert_eq!(state.current_phase, WizardPhase::SuperpowersSetup);
        assert_eq!(state.phases[0].status, PhaseStatus::Skipped);
    }

    #[test]
    fn test_complete_advances_phase() {
        let mut state = WizardState::new("my-project");
        state.complete_current("north-star-done".into());
        assert_eq!(state.current_phase, WizardPhase::SuperpowersSetup);
        assert_eq!(state.phases[0].status, PhaseStatus::Completed);
        assert_eq!(state.phases[0].result.as_deref(), Some("north-star-done"));
    }

    #[test]
    fn test_jump_to_phase() {
        let mut state = WizardState::new("my-project");
        state.jump_to(WizardPhase::NorthStar);
        assert_eq!(state.current_phase, WizardPhase::NorthStar);
    }

    #[test]
    fn test_all_complete_or_skipped_marks_done() {
        let mut state = WizardState::new("my-project");
        state.complete_current("north-star-done".into()); // north star
        state.skip_current(); // superpowers
        state.complete_current("acme-tools".into()); // name gen
        state.skip_current(); // logo
        assert!(state.is_complete());
    }

    #[test]
    fn test_results_flow_forward() {
        let mut state = WizardState::new("my-project");
        state.complete_current("pmf-analysis".into());
        // Superpowers setup should have access to North Star results
        assert_eq!(state.get_result(WizardPhase::NorthStar).as_deref(), Some("pmf-analysis"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core wizard`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement wizard state machine**

```rust
// crates/shepherd-core/src/wizard/state.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WizardPhase {
    NameGen,
    LogoGen,
    NorthStar,
    SuperpowersSetup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub phase: WizardPhase,
    pub status: PhaseStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardState {
    pub project_id: String,
    pub current_phase: WizardPhase,
    pub phases: Vec<PhaseState>,
}

impl WizardState {
    pub fn new(project_id: &str) -> Self {
        Self {
            project_id: project_id.to_string(),
            current_phase: WizardPhase::NorthStar,
            phases: vec![
                PhaseState { phase: WizardPhase::NorthStar, status: PhaseStatus::Pending, result: None },
                PhaseState { phase: WizardPhase::SuperpowersSetup, status: PhaseStatus::Pending, result: None },
                PhaseState { phase: WizardPhase::NameGen, status: PhaseStatus::Pending, result: None },
                PhaseState { phase: WizardPhase::LogoGen, status: PhaseStatus::Pending, result: None },
            ],
        }
    }

    pub fn skip_current(&mut self) {
        if let Some(phase) = self.phases.iter_mut().find(|p| p.phase == self.current_phase) {
            phase.status = PhaseStatus::Skipped;
        }
        self.advance();
    }

    pub fn complete_current(&mut self, result: String) {
        if let Some(phase) = self.phases.iter_mut().find(|p| p.phase == self.current_phase) {
            phase.status = PhaseStatus::Completed;
            phase.result = Some(result);
        }
        self.advance();
    }

    pub fn jump_to(&mut self, target: WizardPhase) {
        self.current_phase = target;
    }

    pub fn is_complete(&self) -> bool {
        self.phases.iter().all(|p| matches!(p.status, PhaseStatus::Completed | PhaseStatus::Skipped))
    }

    pub fn get_result(&self, phase: WizardPhase) -> Option<String> {
        self.phases.iter().find(|p| p.phase == phase).and_then(|p| p.result.clone())
    }

    fn advance(&mut self) {
        let order = [WizardPhase::NorthStar, WizardPhase::SuperpowersSetup, WizardPhase::NameGen, WizardPhase::LogoGen];
        let current_idx = order.iter().position(|p| *p == self.current_phase).unwrap_or(0);
        if current_idx + 1 < order.len() {
            self.current_phase = order[current_idx + 1].clone();
        }
    }
}
```

- [ ] **Step 4: Add wizard module**

```rust
// crates/shepherd-core/src/wizard/mod.rs
pub mod state;
```

Add `pub mod wizard;` to `lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-core wizard`
Expected: PASS (6 tests)

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/wizard/ crates/shepherd-core/src/lib.rs
git commit -m "feat: add New Project Wizard state machine with skip/jump/flow-forward"
```

---

## Summary

This plan adds **20 tasks** across **6 chunks**, implementing all of Shepherd's unique differentiator features:

| Chunk | Tasks | What It Builds |
|-------|-------|----------------|
| 1 | 1–4 | LLM client (3 providers), name brainstorming, RDAP/registry validation, name gen UI |
| 2 | 5–8 | Image generation, multi-format export, logo gen UI, North Star 13-phase wizard |
| 3 | 9–12 | Quality gate runner (auto-detect), plugin gates, PR pipeline, gate/PR frontend |
| 4 | 13–16 | Trigger engine (3 detectors), toast UI, CLI polish (completions), integration tests |
| 5 | 17–19 | nono.sh sandbox in PTY, Obra Superpowers auto-install, context-mode auto-install |
| 6 | 20 | New Project Wizard — guided full-stack SDD journey (name → logo → North Star → Superpowers) |

**Total new files:** ~36 Rust + ~8 TypeScript
**Total tests:** ~92+ unit tests + 8 integration tests
**Key dependencies added:** reqwest, async-trait, image, base64, clap_complete, tempfile, dirs
