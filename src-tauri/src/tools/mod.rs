pub mod cron;
pub mod file_ops;
pub mod patch;
pub mod process;
pub mod profiles;
pub mod registry;
pub mod session_spawn;
pub mod shell;
pub mod traits;

use std::sync::Arc;

use crate::agent::SessionRouter;
use crate::scheduler::Scheduler;
use crate::security::SecurityPolicy;

pub use cron::CronTool;
pub use file_ops::{FileListTool, FileReadTool, FileWriteTool};
pub use patch::PatchTool;
pub use process::ProcessTool;
pub use profiles::{ToolGroup, ToolProfile};
pub use registry::ToolRegistry;
pub use session_spawn::SessionSpawnTool;
pub use shell::ShellTool;
pub use traits::{Tool, ToolInfo, ToolResult};

/// Context needed to register all built-in tools.
pub struct ToolContext {
    pub policy: Arc<SecurityPolicy>,
    pub scheduler: Option<Arc<dyn Scheduler>>,
    pub session_router: Option<Arc<SessionRouter>>,
}

/// Register all built-in tools into `registry`.
///
/// # Arguments
/// * `registry` - The tool registry to populate.
/// * `ctx` - Context containing policy, scheduler, and session router.
///
/// Some tools (cron, session_spawn) require additional dependencies
/// (scheduler, session_router). If these are `None`, those tools
/// will not be registered.
pub fn register_builtin_tools(registry: &mut ToolRegistry, ctx: ToolContext) {
    // Core tools that only need security policy.
    registry.register(Arc::new(ShellTool::new(ctx.policy.clone())));
    registry.register(Arc::new(FileReadTool::new(ctx.policy.clone())));
    registry.register(Arc::new(FileWriteTool::new(ctx.policy.clone())));
    registry.register(Arc::new(FileListTool::new(ctx.policy.clone())));
    registry.register(Arc::new(PatchTool::new(ctx.policy.clone())));
    registry.register(Arc::new(ProcessTool::new(ctx.policy.clone())));

    // Tools that require scheduler.
    if let Some(scheduler) = ctx.scheduler {
        registry.register(Arc::new(CronTool::new(ctx.policy.clone(), scheduler)));
    }

    // Tools that require session router.
    if let Some(session_router) = ctx.session_router {
        registry.register(Arc::new(SessionSpawnTool::new(
            ctx.policy.clone(),
            session_router,
        )));
    }
}

/// Register core tools that only require a security policy.
///
/// This is a convenience function for simple setups that don't need
/// scheduler or session spawning capabilities.
pub fn register_core_tools(registry: &mut ToolRegistry, policy: Arc<SecurityPolicy>) {
    registry.register(Arc::new(ShellTool::new(policy.clone())));
    registry.register(Arc::new(FileReadTool::new(policy.clone())));
    registry.register(Arc::new(FileWriteTool::new(policy.clone())));
    registry.register(Arc::new(FileListTool::new(policy.clone())));
    registry.register(Arc::new(PatchTool::new(policy.clone())));
    registry.register(Arc::new(ProcessTool::new(policy)));
}
