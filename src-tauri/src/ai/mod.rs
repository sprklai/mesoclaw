pub mod agents;
pub mod cache; // Database-specific caching - commented out in file
pub mod context;
pub mod prompts;
pub mod provider;
pub mod providers;
pub mod terminology; // Database-specific terminology - commented out in file
pub mod types;
pub mod utils;
pub mod verbosity;

// Agents can be imported here when needed
// pub use agents::CustomAgent;
// ExplanationCache is commented out in cache.rs - database-specific
// pub use cache::ExplanationCache;
pub use context::ContextManager;
pub use provider::{LLMProvider, ProviderFactory};
pub use providers::{OpenAICompatibleConfig, OpenAICompatibleProvider};
// Terminology is commented out in terminology.rs - database-specific
// pub use terminology::Terminology;
pub use types::{CompletionRequest, CompletionResponse, Message, MessageRole, StreamChunk};
pub use utils::extract_confidence_from_llm_response;
pub use verbosity::Verbosity;
