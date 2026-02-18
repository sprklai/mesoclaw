//! Agent Intelligence Layer — the core reasoning loop and related utilities.
//!
//! # Key types
//! - [`AgentLoop`] — drives the tool-call iteration cycle
//! - [`AgentConfig`] — parameters for the loop (model, max iterations, etc.)
//! - [`AgentMessage`] — a message in the agent's conversation history
//! - [`ParsedToolCall`] — a tool invocation extracted from an LLM response
//!
//! # Tauri commands
//! - [`agent_commands::start_agent_session_command`]
//! - [`agent_commands::cancel_agent_session_command`]

pub mod agent_commands;
pub mod loop_;
pub mod session_router;
pub mod tool_parser;

pub use loop_::{AgentConfig, AgentLoop, AgentMessage};
pub use session_router::{Session, SessionKey, SessionMessage, SessionRouter};
pub use tool_parser::ParsedToolCall;

pub use agent_commands::{cancel_agent_session_command, start_agent_session_command};
