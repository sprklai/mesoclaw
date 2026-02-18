use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::config::app_identity::{OPENROUTER_HTTP_REFERER, OPENROUTER_TITLE};

use crate::ai::provider::{LLMProvider, Result, StreamResponse};
use crate::ai::types::{
    CompletionRequest, CompletionResponse, Message as AppMessage, MessageRole, StreamChunk,
    TokenUsage,
};

/// Default timeout for HTTP requests (30 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum retry attempts for failed requests
const MAX_RETRIES: u32 = 3;

/// OpenAI-compatible message format for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Legacy max_tokens parameter (used by older models and non-OpenAI providers)
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// New max_completion_tokens parameter (used by newer OpenAI models)
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionResponse {
    id: String,
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
    model: String,
}

/// Choice in the completion response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Choice {
    message: ApiMessage,
    finish_reason: Option<String>,
    index: u32,
}

/// Token usage information
#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Streaming response delta
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
    index: u32,
}

/// Content delta in streaming response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Delta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

/// Streaming chunk response from API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiStreamResponse {
    id: String,
    choices: Vec<StreamChoice>,
    model: String,
}

/// Configuration for OpenAI-compatible Provider
///
/// This provider can work with any OpenAI-compatible API including:
/// - OpenAI (https://api.openai.com/v1)
/// - Anthropic (https://api.anthropic.com/v1)
/// - Google Gemini (https://generativelanguage.googleapis.com/v1beta/openai)
/// - Ollama (http://localhost:11434/v1)
/// - OpenRouter (https://openrouter.ai/api/v1)
/// - Vercel AI Gateway (https://ai-gateway.vercel.sh/v1)
#[derive(Debug, Clone)]
pub struct OpenAICompatibleConfig {
    /// API key for authentication (empty for local providers like Ollama)
    pub api_key: String,
    /// Base URL for the API endpoint
    pub base_url: String,
    /// Default model to use
    pub default_model: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Additional HTTP headers (e.g., for Anthropic-specific headers)
    pub extra_headers: HashMap<String, String>,
}

impl Default for OpenAICompatibleConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            default_model: String::new(),
            timeout: DEFAULT_TIMEOUT,
            max_retries: MAX_RETRIES,
            extra_headers: HashMap::new(),
        }
    }
}

impl OpenAICompatibleConfig {
    /// Create a new configuration with the given API key and base URL
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Create a new configuration with API key, base URL, and default model
    pub fn with_model(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: model.into(),
            ..Default::default()
        }
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum retry attempts
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Add an extra HTTP header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple extra HTTP headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.extra_headers = headers;
        self
    }

    /// Create configuration for OpenAI
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, "https://api.openai.com/v1", "gpt-4.1")
    }

    /// Create configuration for Anthropic
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, "https://api.anthropic.com/v1", "claude-sonnet-4.5")
            .with_header("anthropic-version", "2023-06-01")
    }

    /// Create configuration for Google Gemini
    pub fn gemini(api_key: impl Into<String>) -> Self {
        Self::with_model(
            api_key,
            "https://generativelanguage.googleapis.com/v1beta/openai",
            "gemini-2.5-flash",
        )
    }

    /// Create configuration for Ollama (local)
    /// Uses 127.0.0.1 instead of localhost to avoid IPv6 resolution issues
    pub fn ollama() -> Self {
        Self::with_model("", "http://localhost:11434/v1", "llama3")
    }

    /// Create configuration for OpenRouter
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self::with_model(
            api_key,
            "https://openrouter.ai/api/v1",
            "anthropic/claude-sonnet-4.5",
        )
        .with_header("HTTP-Referer", OPENROUTER_HTTP_REFERER)
        .with_header("X-Title", OPENROUTER_TITLE)
    }

    /// Create configuration for Vercel AI Gateway
    pub fn vercel_gateway(api_key: impl Into<String>) -> Self {
        Self::with_model(
            api_key,
            "https://ai-gateway.vercel.sh/v1",
            "google/gemini-3-flash",
        )
    }
}

/// Generic OpenAI-Compatible Provider
///
/// This provider works with any API that follows the OpenAI chat completion protocol,
/// including direct providers, gateways, and local servers.
///
/// The provider-agnostic design means all providers are treated identically
/// at the protocol level. The only differences are:
/// - Base URL (endpoint)
/// - API key requirement (some providers like Ollama don't require it)
/// - Model ID format (provider-specific convention)
pub struct OpenAICompatibleProvider {
    client: Client,
    config: OpenAICompatibleConfig,
    provider_name: String,
}

impl OpenAICompatibleProvider {
    /// Create a new OpenAI-compatible provider
    pub fn new(config: OpenAICompatibleConfig, provider_name: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            config,
            provider_name: provider_name.into(),
        })
    }

    /// Convert application message to API message format
    fn convert_message(msg: &AppMessage) -> ApiMessage {
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };

        ApiMessage {
            role: role.to_string(),
            content: msg.content.clone(),
        }
    }

    /// Build chat completion request from application request
    fn build_request(&self, request: &CompletionRequest) -> ChatCompletionRequest {
        // Handle provider-specific parameter requirements
        let is_openai = self.provider_name == "openai";
        let is_o1_model = request.model.starts_with("o1-") || request.model.starts_with("o1");

        // OpenAI reasoning models (o1-preview, o1-mini) have strict requirements:
        // - Don't support temperature parameter
        // - Don't support max_tokens/max_completion_tokens
        // - Don't support top_p
        let temperature = if is_openai && is_o1_model {
            None
        } else {
            request.temperature
        };

        // OpenAI now uses max_completion_tokens for newer models (gpt-4o and later)
        // But o1 models don't support token limits at all
        let (max_tokens, max_completion_tokens) = if is_openai && is_o1_model {
            (None, None)
        } else if is_openai {
            (None, request.max_tokens)
        } else {
            (request.max_tokens, None)
        };

        let top_p = if is_openai && is_o1_model {
            None
        } else {
            request.top_p
        };

        ChatCompletionRequest {
            model: request.model.clone(),
            messages: request.messages.iter().map(Self::convert_message).collect(),
            temperature,
            max_tokens,
            max_completion_tokens,
            top_p,
            stream: request.stream,
        }
    }

    /// Get authorization header value
    fn get_auth_header(&self) -> Option<String> {
        if self.config.api_key.is_empty() {
            None // No auth for local providers like Ollama
        } else {
            Some(format!("Bearer {}", self.config.api_key))
        }
    }

    /// Execute HTTP request with retry logic
    async fn execute_with_retry(
        &self,
        request_fn: impl Fn() -> Result<reqwest::RequestBuilder>,
    ) -> Result<Response> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s, ...
                let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }

            let builder = request_fn()?;
            match builder.send().await {
                Ok(response) => {
                    let status = response.status();

                    // Success
                    if status.is_success() {
                        return Ok(response);
                    }

                    // Don't retry client errors (except rate limits)
                    if status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS {
                        let error_body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        return Err(format!(
                            "API request failed with status {}: {}",
                            status, error_body
                        ));
                    }

                    // Retry server errors and rate limits
                    last_error = Some(format!("API request failed with status {}", status));
                }
                Err(e) => {
                    last_error = Some(format!("HTTP request failed: {}", e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Request failed".to_string()))
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatibleProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.build_request(&request);

        let mut request_builder = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Content-Type", "application/json");

        // Add auth header if API key is present
        if let Some(auth) = self.get_auth_header() {
            request_builder = request_builder.header("Authorization", auth);
        }

        // Add extra headers
        for (key, value) in &self.config.extra_headers {
            request_builder = request_builder.header(key, value);
        }

        let response = self
            .execute_with_retry(|| {
                request_builder
                    .try_clone()
                    .ok_or_else(|| "Failed to clone request builder".to_string())
                    .map(|b| b.json(&api_request))
            })
            .await?;

        let api_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        // Extract the first choice
        let choice = api_response
            .choices
            .first()
            .ok_or_else(|| "No choices in API response".to_string())?;

        Ok(CompletionResponse {
            content: choice.message.content.clone(),
            model: api_response.model,
            usage: api_response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn stream(&self, request: CompletionRequest) -> Result<StreamResponse> {
        // Set stream to true
        let mut api_request = self.build_request(&request);
        api_request.stream = Some(true);

        let url = format!("{}/chat/completions", self.config.base_url);
        log::info!(
            "[LLM Stream] Provider: {}, URL: {}, Model: {}, Extra headers: {:?}",
            self.provider_name,
            url,
            api_request.model,
            self.config.extra_headers.keys().collect::<Vec<_>>()
        );

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        // Add auth header if API key is present
        if let Some(auth) = self.get_auth_header() {
            request_builder = request_builder.header("Authorization", auth);
        }

        // Add extra headers
        for (key, value) in &self.config.extra_headers {
            request_builder = request_builder.header(key, value);
        }

        let response = self
            .execute_with_retry(|| {
                request_builder
                    .try_clone()
                    .ok_or_else(|| "Failed to clone request builder".to_string())
                    .map(|b| b.json(&api_request))
            })
            .await
            .map_err(|e| {
                log::error!("[LLM Stream] Request failed for {}: {}", url, e);
                e
            })?;

        // Convert to SSE stream
        let byte_stream = response.bytes_stream();
        let event_stream = byte_stream.eventsource();

        let stream = event_stream.filter_map(|event| async move {
            match event {
                Ok(event) => {
                    // Check for [DONE] message
                    if event.data == "[DONE]" {
                        return Some(Ok(StreamChunk {
                            delta: String::new(),
                            is_final: true,
                            finish_reason: Some("stop".to_string()),
                        }));
                    }

                    // Parse the SSE data
                    match serde_json::from_str::<ApiStreamResponse>(&event.data) {
                        Ok(stream_response) => {
                            if let Some(choice) = stream_response.choices.first() {
                                let delta = choice.delta.content.clone().unwrap_or_default();
                                let is_final = choice.finish_reason.is_some();

                                Some(Ok(StreamChunk {
                                    delta,
                                    is_final,
                                    finish_reason: choice.finish_reason.clone(),
                                }))
                            } else {
                                None
                            }
                        }
                        Err(e) => Some(Err(format!("Failed to parse stream chunk: {}", e))),
                    }
                }
                Err(e) => Some(Err(format!("Stream error: {}", e))),
            }
        });

        Ok(Box::pin(stream))
    }

    fn context_limit(&self) -> usize {
        // Use a default context limit
        // In production, this would be loaded from the database model table
        128_000
    }

    fn supports_tools(&self) -> bool {
        // Most OpenAI-compatible providers support function calling
        true
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_openai() {
        let config = OpenAICompatibleConfig::openai("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.default_model, "gpt-4.1");
    }

    #[test]
    fn test_config_anthropic() {
        let config = OpenAICompatibleConfig::anthropic("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.anthropic.com/v1");
        assert_eq!(config.default_model, "claude-sonnet-4.5");
        // Anthropic MUST have the anthropic-version header
        assert_eq!(
            config.extra_headers.get("anthropic-version"),
            Some(&"2023-06-01".to_string())
        );
    }

    #[test]
    fn test_config_gemini() {
        let config = OpenAICompatibleConfig::gemini("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(
            config.base_url,
            "https://generativelanguage.googleapis.com/v1beta/openai"
        );
        assert_eq!(config.default_model, "gemini-2.5-flash");
    }

    #[test]
    fn test_config_ollama() {
        let config = OpenAICompatibleConfig::ollama();
        assert_eq!(config.api_key, "");
        assert_eq!(config.base_url, "http://localhost:11434/v1");
        assert_eq!(config.default_model, "llama3");
        // Ollama should NOT have Anthropic headers
        assert!(config.extra_headers.is_empty());
    }

    #[test]
    fn test_config_openrouter() {
        let config = OpenAICompatibleConfig::openrouter("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(config.default_model, "anthropic/claude-sonnet-4.5");
    }

    #[test]
    fn test_config_vercel_gateway() {
        let config = OpenAICompatibleConfig::vercel_gateway("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://ai-gateway.vercel.sh/v1");
        assert_eq!(config.default_model, "google/gemini-3-flash");
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAICompatibleConfig::new("key", "https://api.test.com")
            .default_model("test-model")
            .timeout(Duration::from_secs(60))
            .max_retries(5)
            .with_header("X-Custom-Header", "value");

        assert_eq!(config.api_key, "key");
        assert_eq!(config.base_url, "https://api.test.com");
        assert_eq!(config.default_model, "test-model");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert_eq!(
            config.extra_headers.get("X-Custom-Header"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_message_conversion() {
        let system_msg = AppMessage::system("You are helpful");
        let api_msg = OpenAICompatibleProvider::convert_message(&system_msg);
        assert_eq!(api_msg.role, "system");
        assert_eq!(api_msg.content, "You are helpful");

        let user_msg = AppMessage::user("Hello");
        let api_msg = OpenAICompatibleProvider::convert_message(&user_msg);
        assert_eq!(api_msg.role, "user");
        assert_eq!(api_msg.content, "Hello");

        let assistant_msg = AppMessage::assistant("Hi there");
        let api_msg = OpenAICompatibleProvider::convert_message(&assistant_msg);
        assert_eq!(api_msg.role, "assistant");
        assert_eq!(api_msg.content, "Hi there");
    }

    #[test]
    fn test_provider_creation() {
        let config = OpenAICompatibleConfig::openai("test-key");
        let provider = OpenAICompatibleProvider::new(config, "openai");
        assert!(provider.is_ok());
    }

    #[test]
    fn test_provider_traits() {
        let config = OpenAICompatibleConfig::anthropic("test-key");
        let provider = OpenAICompatibleProvider::new(config, "anthropic").unwrap();

        assert_eq!(provider.provider_name(), "anthropic");
        assert!(provider.supports_tools());
        assert_eq!(provider.context_limit(), 128_000);
    }

    #[test]
    fn test_build_request() {
        let config = OpenAICompatibleConfig::gemini("test-key");
        let provider = OpenAICompatibleProvider::new(config, "gemini").unwrap();

        let request = CompletionRequest::new(
            "gemini-2.5-flash",
            vec![
                AppMessage::system("You are helpful"),
                AppMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(1000);

        let api_request = provider.build_request(&request);

        assert_eq!(api_request.model, "gemini-2.5-flash");
        assert_eq!(api_request.messages.len(), 2);
        assert_eq!(api_request.messages[0].role, "system");
        assert_eq!(api_request.messages[1].role, "user");
        assert_eq!(api_request.temperature, Some(0.7));
        assert_eq!(api_request.max_tokens, Some(1000));
    }
}
