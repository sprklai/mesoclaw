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
//! - [`commands::list_agents_command`]
//! - [`commands::create_agent_command`]
//! - [`commands::get_agent_command`]
//! - [`commands::delete_agent_command`]
//! - [`commands::run_agent_command`]
//! - [`commands::list_sessions_command`]
//! - [`commands::get_session_command`]
//! - [`commands::get_workspace_file_command`]
//! - [`commands::update_workspace_file_command`]

pub mod agent_commands;
pub mod commands;
pub mod loop_;
pub mod session_router;
pub mod skills;
pub mod tool_parser;

pub use loop_::{AgentConfig, AgentLoop, AgentMessage};
pub use session_router::{Session, SessionKey, SessionMessage, SessionRouter};
pub use skills::{
    Skill, SkillMetadata, SkillRegistry, SkillRequirements, SkillSnapshot, SkillSource,
    TemplateParameter, ToolSchema,
};
pub use tool_parser::ParsedToolCall;

pub use agent_commands::{cancel_agent_session_command, start_agent_session_command};
pub use commands::{
    create_agent_command, delete_agent_command, get_agent_command, get_session_command,
    get_workspace_file_command, list_agents_command, list_sessions_command,
    list_workspace_files_command, run_agent_command, update_agent_command,
    update_workspace_file_command,
};
