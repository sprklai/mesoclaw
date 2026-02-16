// Integration tests for AI multi-provider architecture
// Run with: cargo test --test integration_ai_providers

use local_ts_lib::ai::providers::openai_compatible::OpenAICompatibleConfig;

// Helper trait to check auth requirement
trait OpenAICompatibleConfigExt {
    fn requires_auth(&self) -> bool;
}

impl OpenAICompatibleConfigExt for OpenAICompatibleConfig {
    fn requires_auth(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use local_ts_lib::ai::provider::LLMProvider;
    use super::{OpenAICompatibleConfig, OpenAICompatibleConfigExt};
    use local_ts_lib::ai::providers::OpenAICompatibleProvider;
    use local_ts_lib::ai::types::{CompletionRequest, Message as AppMessage};
    use std::time::Duration;

    // =========================================================================
    // Provider Configuration Tests
    // =========================================================================

    #[test]
    fn test_openai_configuration() {
        let config = OpenAICompatibleConfig::openai("test-key-12345");
        assert_eq!(config.api_key, "test-key-12345");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.default_model, "gpt-4.1");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_anthropic_configuration() {
        let config = OpenAICompatibleConfig::anthropic("test-key-67890");
        assert_eq!(config.api_key, "test-key-67890");
        assert_eq!(config.base_url, "https://api.anthropic.com/v1");
        assert_eq!(config.default_model, "claude-sonnet-4.5");

        // Verify Anthropic-specific header is present
        assert!(config.extra_headers.contains_key("anthropic-version"));
        assert_eq!(
            config.extra_headers.get("anthropic-version"),
            Some(&"2023-06-01".to_string())
        );
    }

    #[test]
    fn test_gemini_configuration() {
        let config = OpenAICompatibleConfig::gemini("test-key-gemini");
        assert_eq!(config.api_key, "test-key-gemini");
        assert_eq!(
            config.base_url,
            "https://generativelanguage.googleapis.com/v1beta/openai"
        );
        assert_eq!(config.default_model, "gemini-2.5-flash");
    }

    #[test]
    fn test_ollama_configuration_no_api_key() {
        let config = OpenAICompatibleConfig::ollama();
        assert_eq!(config.api_key, "");
        assert_eq!(config.base_url, "http://localhost:11434/v1");
        assert_eq!(config.default_model, "llama3");

        // Ollama doesn't require auth
        assert!(!config.requires_auth());
    }

    #[test]
    fn test_openrouter_configuration() {
        let config = OpenAICompatibleConfig::openrouter("test-key-or");
        assert_eq!(config.api_key, "test-key-or");
        assert_eq!(config.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(config.default_model, "anthropic/claude-sonnet-4.5");
    }

    #[test]
    fn test_vercel_gateway_configuration() {
        let config = OpenAICompatibleConfig::vercel_gateway("test-key-vercel");
        assert_eq!(config.api_key, "test-key-vercel");
        assert_eq!(config.base_url, "https://ai-gateway.vercel.sh/v1");
        assert_eq!(config.default_model, "google/gemini-3-flash");
    }

    #[test]
    fn test_custom_configuration_builder() {
        let config = OpenAICompatibleConfig::new("custom-key", "https://custom.api.com")
            .default_model("custom-model")
            .timeout(Duration::from_secs(60))
            .max_retries(5)
            .with_header("X-Custom-Header", "custom-value")
            .with_header("X-Another-Header", "another-value");

        assert_eq!(config.api_key, "custom-key");
        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.default_model, "custom-model");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert_eq!(
            config.extra_headers.get("X-Custom-Header"),
            Some(&"custom-value".to_string())
        );
        assert_eq!(
            config.extra_headers.get("X-Another-Header"),
            Some(&"another-value".to_string())
        );
    }

    #[test]
    fn test_configuration_with_headers_map() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Header-1".to_string(), "value-1".to_string());
        headers.insert("X-Header-2".to_string(), "value-2".to_string());

        let config =
            OpenAICompatibleConfig::new("key", "https://api.com").with_headers(headers);

        assert_eq!(config.extra_headers.len(), 2);
        assert_eq!(config.extra_headers.get("X-Header-1"), Some(&"value-1".to_string()));
        assert_eq!(config.extra_headers.get("X-Header-2"), Some(&"value-2".to_string()));
    }

    // =========================================================================
    // Provider Creation Tests
    // =========================================================================

    #[test]
    fn test_provider_creation_openai() {
        let config = OpenAICompatibleConfig::openai("test-key");
        let result = OpenAICompatibleProvider::new(config, "openai");
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn test_provider_creation_anthropic() {
        let config = OpenAICompatibleConfig::anthropic("test-key");
        let result = OpenAICompatibleProvider::new(config, "anthropic");
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "anthropic");
    }

    #[test]
    fn test_provider_creation_ollama() {
        let config = OpenAICompatibleConfig::ollama();
        let result = OpenAICompatibleProvider::new(config, "ollama");
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "ollama");
    }

    // =========================================================================
    // Provider Trait Implementation Tests
    // =========================================================================

    #[test]
    fn test_provider_name() {
        let config = OpenAICompatibleConfig::openai("test-key");
        let provider = OpenAICompatibleProvider::new(config, "openai").unwrap();
        assert_eq!(provider.provider_name(), "openai");

        let config = OpenAICompatibleConfig::anthropic("test-key");
        let provider = OpenAICompatibleProvider::new(config, "anthropic").unwrap();
        assert_eq!(provider.provider_name(), "anthropic");

        let config = OpenAICompatibleConfig::ollama();
        let provider = OpenAICompatibleProvider::new(config, "ollama").unwrap();
        assert_eq!(provider.provider_name(), "ollama");
    }

    #[test]
    fn test_provider_supports_tools() {
        let config = OpenAICompatibleConfig::openai("test-key");
        let provider = OpenAICompatibleProvider::new(config, "openai").unwrap();
        assert!(provider.supports_tools());

        let config = OpenAICompatibleConfig::anthropic("test-key");
        let provider = OpenAICompatibleProvider::new(config, "anthropic").unwrap();
        assert!(provider.supports_tools());

        let config = OpenAICompatibleConfig::ollama();
        let provider = OpenAICompatibleProvider::new(config, "ollama").unwrap();
        assert!(provider.supports_tools());
    }

    #[test]
    fn test_provider_context_limit() {
        let config = OpenAICompatibleConfig::openai("test-key");
        let provider = OpenAICompatibleProvider::new(config, "openai").unwrap();
        assert_eq!(provider.context_limit(), 128_000);

        let config = OpenAICompatibleConfig::anthropic("test-key");
        let provider = OpenAICompatibleProvider::new(config, "anthropic").unwrap();
        assert_eq!(provider.context_limit(), 128_000);

        let config = OpenAICompatibleConfig::ollama();
        let provider = OpenAICompatibleProvider::new(config, "ollama").unwrap();
        assert_eq!(provider.context_limit(), 128_000);
    }

    // =========================================================================
    // Default Configuration Tests
    // =========================================================================

    #[test]
    fn test_default_configuration_values() {
        let config = OpenAICompatibleConfig::default();
        assert_eq!(config.api_key, "");
        assert_eq!(config.base_url, "");
        assert_eq!(config.default_model, "");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert!(config.extra_headers.contains_key("anthropic-version"));
    }

    #[test]
    fn test_timeout_configuration() {
        let config = OpenAICompatibleConfig::openai("test-key").timeout(Duration::from_secs(120));
        assert_eq!(config.timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_retry_configuration() {
        let config =
            OpenAICompatibleConfig::openai("test-key").max_retries(10);
        assert_eq!(config.max_retries, 10);
    }

    // =========================================================================
    // Provider-Specific Model Tests
    // =========================================================================

    #[test]
    fn test_openai_default_model() {
        let config = OpenAICompatibleConfig::openai("test-key");
        assert_eq!(config.default_model, "gpt-4.1");
    }

    #[test]
    fn test_anthropic_default_model() {
        let config = OpenAICompatibleConfig::anthropic("test-key");
        assert_eq!(config.default_model, "claude-sonnet-4.5");
    }

    #[test]
    fn test_gemini_default_model() {
        let config = OpenAICompatibleConfig::gemini("test-key");
        assert_eq!(config.default_model, "gemini-2.5-flash");
    }

    #[test]
    fn test_ollama_default_model() {
        let config = OpenAICompatibleConfig::ollama();
        assert_eq!(config.default_model, "llama3");
    }

    #[test]
    fn test_openrouter_default_model() {
        let config = OpenAICompatibleConfig::openrouter("test-key");
        assert_eq!(config.default_model, "anthropic/claude-sonnet-4.5");
    }

    #[test]
    fn test_vercel_gateway_default_model() {
        let config = OpenAICompatibleConfig::vercel_gateway("test-key");
        assert_eq!(config.default_model, "google/gemini-3-flash");
    }

    // =========================================================================
    // Provider URL Tests
    // =========================================================================

    #[test]
    fn test_openai_base_url() {
        let config = OpenAICompatibleConfig::openai("test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_anthropic_base_url() {
        let config = OpenAICompatibleConfig::anthropic("test-key");
        assert_eq!(config.base_url, "https://api.anthropic.com/v1");
    }

    #[test]
    fn test_gemini_base_url() {
        let config = OpenAICompatibleConfig::gemini("test-key");
        assert_eq!(
            config.base_url,
            "https://generativelanguage.googleapis.com/v1beta/openai"
        );
    }

    #[test]
    fn test_ollama_base_url() {
        let config = OpenAICompatibleConfig::ollama();
        assert_eq!(config.base_url, "http://localhost:11434/v1");
    }

    #[test]
    fn test_openrouter_base_url() {
        let config = OpenAICompatibleConfig::openrouter("test-key");
        assert_eq!(config.base_url, "https://openrouter.ai/api/v1");
    }

    #[test]
    fn test_vercel_gateway_base_url() {
        let config = OpenAICompatibleConfig::vercel_gateway("test-key");
        assert_eq!(config.base_url, "https://ai-gateway.vercel.sh/v1");
    }

    // =========================================================================
    // Extra Headers Tests
    // =========================================================================

    #[test]
    fn test_anthropic_has_version_header() {
        let config = OpenAICompatibleConfig::anthropic("test-key");
        assert!(config.extra_headers.contains_key("anthropic-version"));
        assert_eq!(
            config.extra_headers.get("anthropic-version"),
            Some(&"2023-06-01".to_string())
        );
    }

    #[test]
    fn test_custom_headers_preserved() {
        let config = OpenAICompatibleConfig::new("key", "https://api.com")
            .with_header("X-Custom-1", "value-1")
            .with_header("X-Custom-2", "value-2");

        assert_eq!(config.extra_headers.len(), 3); // 2 custom + anthropic-version
        assert_eq!(config.extra_headers.get("X-Custom-1"), Some(&"value-1".to_string()));
        assert_eq!(config.extra_headers.get("X-Custom-2"), Some(&"value-2".to_string()));
    }

    // =========================================================================
    // CompletionRequest Builder Tests
    // =========================================================================

    #[test]
    fn test_completion_request_basic() {
        let request = CompletionRequest::new(
            "gpt-4.1",
            vec![AppMessage::user("Explain Rust")],
        );

        assert_eq!(request.model, "gpt-4.1");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, local_ts_lib::ai::types::MessageRole::User);
        assert_eq!(request.messages[0].content, "Explain Rust");
        assert_eq!(request.temperature, None);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.top_p, None);
        assert_eq!(request.stream, None);
    }

    #[test]
    fn test_completion_request_with_parameters() {
        let request = CompletionRequest::new(
            "gpt-4.1",
            vec![
                AppMessage::system("You are helpful"),
                AppMessage::user("Explain Rust"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_top_p(0.9);

        assert_eq!(request.model, "gpt-4.1");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.stream, None);
    }

    #[test]
    fn test_completion_request_with_stream() {
        let request = CompletionRequest::new(
            "gpt-4.1",
            vec![AppMessage::user("Hello")],
        )
        .with_stream(true);

        assert_eq!(request.stream, Some(true));
    }

    #[test]
    fn test_completion_request_multi_turn_conversation() {
        let request = CompletionRequest::new(
            "gpt-4.1",
            vec![
                AppMessage::user("What is Rust?"),
                AppMessage::assistant("Rust is a systems programming language"),
                AppMessage::user("Why is it safe?"),
            ],
        );

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, local_ts_lib::ai::types::MessageRole::User);
        assert_eq!(request.messages[1].role, local_ts_lib::ai::types::MessageRole::Assistant);
        assert_eq!(request.messages[2].role, local_ts_lib::ai::types::MessageRole::User);
    }

    #[test]
    fn test_completion_request_with_all_parameters() {
        let request = CompletionRequest::new(
            "gpt-4.1",
            vec![AppMessage::user("Test")],
        )
        .with_temperature(0.5)
        .with_max_tokens(2000)
        .with_top_p(0.8)
        .with_stream(false);

        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.max_tokens, Some(2000));
        assert_eq!(request.top_p, Some(0.8));
        assert_eq!(request.stream, Some(false));
    }

    #[test]
    fn test_completion_request_empty_messages() {
        let request = CompletionRequest::new("gpt-4.1", vec![]);
        assert_eq!(request.messages.len(), 0);
    }

    // =========================================================================
    // Message Constructor Tests
    // =========================================================================

    #[test]
    fn test_message_constructors() {
        let system_msg = AppMessage::system("You are helpful");
        assert_eq!(system_msg.role, local_ts_lib::ai::types::MessageRole::System);
        assert_eq!(system_msg.content, "You are helpful");

        let user_msg = AppMessage::user("Hello");
        assert_eq!(user_msg.role, local_ts_lib::ai::types::MessageRole::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = AppMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, local_ts_lib::ai::types::MessageRole::Assistant);
        assert_eq!(assistant_msg.content, "Hi there!");
    }

    #[test]
    fn test_message_multibyte_characters() {
        let msg = AppMessage::user("Hello 世界 🌍");
        assert_eq!(msg.content, "Hello 世界 🌍");
    }

    #[test]
    fn test_message_empty_content() {
        let msg = AppMessage::user("");
        assert_eq!(msg.content, "");
    }

    #[test]
    fn test_message_special_characters() {
        let special_content = "Test: \n\t\r\"'\\&<>";
        let msg = AppMessage::user(special_content);
        assert_eq!(msg.content, special_content);
    }

    #[test]
    fn test_message_very_long_content() {
        let long_content = "A".repeat(10000);
        let msg = AppMessage::user(&long_content);
        assert_eq!(msg.content.len(), 10000);
    }

    // =========================================================================
    // Edge Cases and Boundary Tests
    // =========================================================================

    #[test]
    fn test_temperature_boundary_values() {
        // Minimum temperature
        let request1 = CompletionRequest::new("gpt-4.1", vec![]).with_temperature(0.0);
        assert_eq!(request1.temperature, Some(0.0));

        // Maximum temperature
        let request2 = CompletionRequest::new("gpt-4.1", vec![]).with_temperature(2.0);
        assert_eq!(request2.temperature, Some(2.0));
    }

    #[test]
    fn test_max_tokens_boundary_values() {
        // Minimum tokens
        let request1 = CompletionRequest::new("gpt-4.1", vec![]).with_max_tokens(1);
        assert_eq!(request1.max_tokens, Some(1));

        // Large token count
        let request2 = CompletionRequest::new("gpt-4.1", vec![]).with_max_tokens(128000);
        assert_eq!(request2.max_tokens, Some(128000));
    }

    #[test]
    fn test_model_id_with_provider_prefix() {
        let request = CompletionRequest::new(
            "anthropic/claude-sonnet-4.5",
            vec![AppMessage::user("Test")],
        );

        // Model ID should preserve the provider/model format
        assert_eq!(request.model, "anthropic/claude-sonnet-4.5");
    }

    // =========================================================================
    // Provider Factory Module Tests
    // =========================================================================

    #[test]
    fn test_provider_type_from_id() {
        use local_ts_lib::ai::providers::ProviderType;

        assert_eq!(
            ProviderType::from_id("vercel-ai-gateway"),
            Some(local_ts_lib::ai::providers::ProviderType::VercelAIGateway)
        );
        assert_eq!(
            ProviderType::from_id("openrouter"),
            Some(local_ts_lib::ai::providers::ProviderType::OpenRouter)
        );
        assert_eq!(ProviderType::from_id("unknown"), None);
    }

    #[test]
    fn test_provider_type_as_id() {
        assert_eq!(
            local_ts_lib::ai::providers::ProviderType::VercelAIGateway.as_id(),
            "vercel-ai-gateway"
        );
        assert_eq!(
            local_ts_lib::ai::providers::ProviderType::OpenRouter.as_id(),
            "openrouter"
        );
    }
}
