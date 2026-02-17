use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::ai::provider::{LLMProvider, Result, StreamResponse};
use crate::ai::types::{
    CompletionRequest, CompletionResponse, Message as AppMessage, MessageRole, StreamChunk,
    TokenUsage,
};
use crate::config::app_identity::{OPENROUTER_HTTP_REFERER, OPENROUTER_TITLE};

/// Default base URL for OpenRouter
const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

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
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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

/// Configuration for OpenRouter Provider
#[derive(Debug, Clone)]
pub struct OpenRouterConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (defaults to OpenRouter)
    pub base_url: String,
    /// Default model to use
    pub default_model: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            default_model: "anthropic/claude-3.5-sonnet".to_string(),
            timeout: DEFAULT_TIMEOUT,
            max_retries: MAX_RETRIES,
        }
    }
}

impl OpenRouterConfig {
    /// Create a new configuration with the given API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Set the base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// OpenRouter Provider
///
/// Provides access to multiple LLM providers through OpenRouter's unified API:
/// - OpenAI (gpt-4, gpt-4o, gpt-3.5-turbo, etc.)
/// - Anthropic (claude-opus, claude-sonnet, claude-haiku)
/// - Google (gemini-pro, gemini-flash, etc.)
/// - Meta (llama-3, llama-3.1, etc.)
/// - Mistral AI (mistral-large, mixtral, etc.)
/// - And many more
///
/// Model format: "provider/model-name" (e.g., "anthropic/claude-3.5-sonnet")
/// Unlike Vercel AI Gateway, OpenRouter models are specified without a gateway prefix.
pub struct OpenRouterProvider {
    client: Client,
    config: OpenRouterConfig,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider
    pub fn new(config: OpenRouterConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { client, config })
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
        ChatCompletionRequest {
            model: request.model.clone(),
            messages: request.messages.iter().map(Self::convert_message).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stream: request.stream,
        }
    }

    /// Execute HTTP request with retry logic
    async fn execute_with_retry(
        &self,
        request_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<Response> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s, ...
                let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }

            match request_fn().send().await {
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
                    last_error = Some(format!(
                        "API request failed with status {}",
                        status
                    ));
                }
                Err(e) => {
                    last_error = Some(format!("HTTP request failed: {}", e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Request failed".to_string()))
    }

    /// Get the context limit for a given model
    fn get_context_limit(model: &str) -> usize {
        // Extract provider and model name
        let parts: Vec<&str> = model.split('/').collect();
        if parts.len() != 2 {
            return 4096; // Default fallback
        }

        let (provider, model_name) = (parts[0], parts[1]);

        match provider {
            "anthropic" => {
                if model_name.contains("claude-3.5")
                    || model_name.contains("claude-sonnet-4")
                    || model_name.contains("claude-opus-4")
                    || model_name.contains("claude-3")
                    || model_name.contains("claude-opus")
                {
                    200_000
                } else {
                    100_000 // Claude 2 and earlier
                }
            }
            "openai" => {
                if model_name.contains("gpt-4-turbo") || model_name.contains("gpt-4o") {
                    128_000
                } else if model_name.contains("gpt-4") {
                    8_192
                } else if model_name.contains("gpt-3.5-turbo-16k") {
                    16_384
                } else if model_name.contains("gpt-3.5") {
                    4_096
                } else {
                    8_192
                }
            }
            "google" => {
                if model_name.contains("gemini-1.5")
                    || model_name.contains("gemini-2")
                    || model_name.contains("gemini-exp")
                {
                    1_000_000 // Gemini 1.5+ has massive context
                } else {
                    32_768
                }
            }
            "meta-llama" | "mistralai" | "deepseek" => {
                // Most open-source models have 32k-128k context
                if model_name.contains("70b") || model_name.contains("large") {
                    32_768
                } else {
                    8_192
                }
            }
            _ => 4_096, // Default fallback
        }
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.build_request(&request);

        let response = self
            .execute_with_retry(|| {
                self.client
                    .post(format!("{}/chat/completions", self.config.base_url))
                    .header("Authorization", format!("Bearer {}", self.config.api_key))
                    .header("HTTP-Referer", OPENROUTER_HTTP_REFERER)
                    .header("X-Title", OPENROUTER_TITLE)
                    .header("Content-Type", "application/json")
                    .json(&api_request)
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

        let response = self
            .execute_with_retry(|| {
                self.client
                    .post(format!("{}/chat/completions", self.config.base_url))
                    .header("Authorization", format!("Bearer {}", self.config.api_key))
                    .header("HTTP-Referer", OPENROUTER_HTTP_REFERER)
                    .header("X-Title", OPENROUTER_TITLE)
                    .header("Content-Type", "application/json")
                    .json(&api_request)
            })
            .await?;

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
                        Err(e) => Some(Err(format!(
                            "Failed to parse stream chunk: {}",
                            e
                        ))),
                    }
                }
                Err(e) => Some(Err(format!("Stream error: {}", e))),
            }
        });

        Ok(Box::pin(stream))
    }

    fn context_limit(&self) -> usize {
        Self::get_context_limit(&self.config.default_model)
    }

    fn supports_tools(&self) -> bool {
        // OpenRouter supports function calling for compatible models
        true
    }

    fn provider_name(&self) -> &str {
        "openrouter"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = OpenRouterConfig::new("test-key")
            .with_base_url("https://custom.api.com")
            .with_default_model("anthropic/claude-3.5-sonnet")
            .with_timeout(Duration::from_secs(60))
            .with_max_retries(5);

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.default_model, "anthropic/claude-3.5-sonnet");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_message_conversion() {
        let system_msg = AppMessage::system("You are helpful");
        let api_msg = OpenRouterProvider::convert_message(&system_msg);
        assert_eq!(api_msg.role, "system");
        assert_eq!(api_msg.content, "You are helpful");

        let user_msg = AppMessage::user("Hello");
        let api_msg = OpenRouterProvider::convert_message(&user_msg);
        assert_eq!(api_msg.role, "user");
        assert_eq!(api_msg.content, "Hello");

        let assistant_msg = AppMessage::assistant("Hi there");
        let api_msg = OpenRouterProvider::convert_message(&assistant_msg);
        assert_eq!(api_msg.role, "assistant");
        assert_eq!(api_msg.content, "Hi there");
    }

    #[test]
    fn test_context_limits() {
        // Anthropic models
        assert_eq!(
            OpenRouterProvider::get_context_limit("anthropic/claude-3.5-sonnet"),
            200_000
        );
        assert_eq!(
            OpenRouterProvider::get_context_limit("anthropic/claude-opus-4"),
            200_000
        );

        // OpenAI models
        assert_eq!(
            OpenRouterProvider::get_context_limit("openai/gpt-4o"),
            128_000
        );
        assert_eq!(
            OpenRouterProvider::get_context_limit("openai/gpt-4"),
            8_192
        );
        assert_eq!(
            OpenRouterProvider::get_context_limit("openai/gpt-3.5-turbo"),
            4_096
        );

        // Google models
        assert_eq!(
            OpenRouterProvider::get_context_limit("google/gemini-2-flash-exp"),
            1_000_000
        );

        // Meta models
        assert_eq!(
            OpenRouterProvider::get_context_limit("meta-llama/llama-3.1-70b"),
            32_768
        );

        // Unknown models
        assert_eq!(
            OpenRouterProvider::get_context_limit("unknown/model"),
            4_096
        );
    }

    #[test]
    fn test_provider_creation() {
        let config = OpenRouterConfig::new("test-key");
        let provider = OpenRouterProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_provider_traits() {
        let config = OpenRouterConfig::new("test-key");
        let provider = OpenRouterProvider::new(config).unwrap();

        assert_eq!(provider.provider_name(), "openrouter");
        assert!(provider.supports_tools());
        assert_eq!(provider.context_limit(), 200_000); // Default model is claude-3.5-sonnet
    }

    #[test]
    fn test_build_request() {
        let config = OpenRouterConfig::new("test-key");
        let provider = OpenRouterProvider::new(config).unwrap();

        let request = CompletionRequest::new(
            "anthropic/claude-3.5-sonnet",
            vec![
                AppMessage::system("You are helpful"),
                AppMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(1000);

        let api_request = provider.build_request(&request);

        assert_eq!(api_request.model, "anthropic/claude-3.5-sonnet");
        assert_eq!(api_request.messages.len(), 2);
        assert_eq!(api_request.messages[0].role, "system");
        assert_eq!(api_request.messages[1].role, "user");
        assert_eq!(api_request.temperature, Some(0.7));
        assert_eq!(api_request.max_tokens, Some(1000));
    }
}
