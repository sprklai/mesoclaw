use super::types::Message;

/// Manages context window and token counting for LLM requests
pub struct ContextManager {
    /// Maximum context window size in tokens
    max_tokens: usize,
}

impl ContextManager {
    /// Create a new context manager with the given token limit
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Estimate the number of tokens in a message
    /// This is a rough approximation: ~4 characters per token for English text
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // Simple heuristic: divide character count by 4
        // This is a rough approximation and should be replaced with a proper tokenizer
        (text.len() + 3) / 4
    }

    /// Estimate total tokens for a list of messages
    pub fn estimate_message_tokens(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|msg| {
                // Add overhead for role and structure
                self.estimate_tokens(&msg.content) + 4
            })
            .sum()
    }

    /// Check if messages fit within the context window
    pub fn fits_in_context(&self, messages: &[Message], max_completion_tokens: usize) -> bool {
        let message_tokens = self.estimate_message_tokens(messages);
        message_tokens + max_completion_tokens <= self.max_tokens
    }

    /// Truncate messages to fit within context window
    /// Keeps system messages and most recent messages, dropping older user/assistant messages
    pub fn truncate_to_fit(
        &self,
        messages: Vec<Message>,
        max_completion_tokens: usize,
    ) -> Vec<Message> {
        let available_tokens = self.max_tokens.saturating_sub(max_completion_tokens);

        // Separate system messages from conversation messages
        let (system_msgs, mut conversation_msgs): (Vec<_>, Vec<_>) = messages
            .into_iter()
            .partition(|msg| matches!(msg.role, super::types::MessageRole::System));

        // Calculate system message tokens
        let system_tokens = self.estimate_message_tokens(&system_msgs);

        if system_tokens >= available_tokens {
            // If system messages alone exceed limit, just return them truncated
            return system_msgs;
        }

        let remaining_tokens = available_tokens - system_tokens;
        let mut current_tokens = 0;
        let mut result = Vec::new();

        // Add messages from most recent to oldest until we hit the limit
        conversation_msgs.reverse();
        for msg in conversation_msgs {
            let msg_tokens = self.estimate_tokens(&msg.content) + 4;
            if current_tokens + msg_tokens > remaining_tokens {
                break;
            }
            current_tokens += msg_tokens;
            result.push(msg);
        }

        // Reverse back to chronological order
        result.reverse();

        // Combine system messages with conversation messages
        let mut final_messages = system_msgs;
        final_messages.extend(result);
        final_messages
    }

    /// Get the maximum context window size
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::Message;

    #[test]
    fn test_estimate_tokens() {
        let manager = ContextManager::new(4096);

        // Rough estimate: ~4 chars per token
        let text = "Hello, world!"; // 13 chars -> ~3-4 tokens
        let tokens = manager.estimate_tokens(text);
        assert!(tokens >= 3 && tokens <= 4);
    }

    #[test]
    fn test_estimate_message_tokens() {
        let manager = ContextManager::new(4096);
        let messages = vec![Message::system("You are helpful"), Message::user("Hello")];

        let tokens = manager.estimate_message_tokens(&messages);
        // Should include content tokens + overhead
        assert!(tokens > 0);
    }

    #[test]
    fn test_fits_in_context() {
        let manager = ContextManager::new(100);

        // Small messages should fit
        let messages = vec![Message::user("Hi")];
        assert!(manager.fits_in_context(&messages, 50));

        // Large completion tokens should not fit
        assert!(!manager.fits_in_context(&messages, 99));
    }

    #[test]
    fn test_truncate_to_fit() {
        let manager = ContextManager::new(100);

        let messages = vec![
            Message::system("System prompt"),
            Message::user("Message 1"),
            Message::assistant("Response 1"),
            Message::user("Message 2"),
            Message::assistant("Response 2"),
            Message::user("Message 3"),
        ];

        let truncated = manager.truncate_to_fit(messages.clone(), 50);

        // Should keep system message
        assert!(
            truncated
                .iter()
                .any(|m| matches!(m.role, crate::ai::MessageRole::System))
        );

        // Should be shorter than original
        assert!(truncated.len() <= messages.len());

        // Should fit in context
        assert!(manager.fits_in_context(&truncated, 50));
    }

    #[test]
    fn test_truncate_preserves_system_messages() {
        let manager = ContextManager::new(50);

        let messages = vec![
            Message::system("Important system prompt"),
            Message::user("This is a longer user message that will be dropped"),
        ];

        // With only 5 tokens available for completion, we'll have ~45 for messages
        // System message: ~11 tokens, User message: ~17 tokens
        // Total would be ~28 tokens, but we want only system to fit
        // So we use a tighter constraint
        let truncated = manager.truncate_to_fit(messages, 35);

        // System message should always be preserved, user message should be dropped
        assert_eq!(truncated.len(), 1);
        assert!(matches!(truncated[0].role, crate::ai::MessageRole::System));
    }
}
