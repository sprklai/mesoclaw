use serde::{Deserialize, Serialize};

/// Role of a message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Request for LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The model to use (e.g., "openai/gpt-4", "anthropic/claude-sonnet-4.5")
    pub model: String,

    /// The messages in the conversation
    pub messages: Vec<Message>,

    /// Temperature for sampling (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl CompletionRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: None,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}

/// Response from LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// The generated content
    pub content: String,

    /// The model that generated the response
    pub model: String,

    /// Token usage information
    pub usage: Option<TokenUsage>,

    /// Finish reason (e.g., "stop", "length", "content_filter")
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A chunk of a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// The delta content for this chunk
    pub delta: String,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Finish reason if this is the final chunk
    pub finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let system_msg = Message::system("You are a helpful assistant");
        assert_eq!(system_msg.role, MessageRole::System);
        assert_eq!(system_msg.content, "You are a helpful assistant");

        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
        assert_eq!(assistant_msg.content, "Hi there!");
    }

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::new("openai/gpt-4", vec![Message::user("Test")])
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_top_p(0.9)
            .with_stream(true);

        assert_eq!(request.model, "openai/gpt-4");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.stream, Some(true));
    }
}
