//! Application adapters for the skill system.
//!
//! Adapters provide application-specific context and tools to the
//! domain-agnostic skill engine.

mod traits;

// Database-specific adapter module removed - legacy project-specific code no longer needed.
// Re-implement if you need database-specific AI skills integration
pub use traits::{
    AdapterError, ApplicationAdapter, ContextBag, ContextType, ToolCall, ToolDefinition, ToolResult,
};
