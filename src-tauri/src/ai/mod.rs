pub mod agents;
pub mod context;
pub mod discovery;
pub mod prompts;
pub mod provider;
pub mod providers;
pub mod types;
pub mod utils;
pub mod verbosity;

pub use context::ContextManager;
pub use provider::{LLMProvider, ProviderFactory};
pub use providers::{OpenAICompatibleConfig, OpenAICompatibleProvider};
pub use types::{CompletionRequest, CompletionResponse, Message, MessageRole, StreamChunk};
pub use utils::extract_confidence_from_llm_response;
pub use verbosity::Verbosity;
