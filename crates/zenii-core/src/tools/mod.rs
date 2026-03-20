pub mod agent_self_tool;
pub mod config_tool;
pub mod file_ops;
pub mod file_search;
pub mod learn;
pub mod memory_tool;
pub mod patch;
pub mod process;
pub mod registry;
pub mod shell;
pub mod skill_proposal;
pub mod system_info;
pub mod traits;
pub mod web_search;

#[cfg(feature = "channels")]
pub mod channel_tool;
#[cfg(feature = "scheduler")]
pub mod scheduler_tool;
#[cfg(feature = "workflows")]
pub mod workflow_tool;

pub use registry::ToolRegistry;
pub use traits::*;
