pub mod file_ops;
pub mod registry;
pub mod shell;
pub mod traits;

use std::sync::Arc;

use crate::security::SecurityPolicy;

pub use file_ops::{FileListTool, FileReadTool, FileWriteTool};
pub use registry::ToolRegistry;
pub use shell::ShellTool;
pub use traits::{Tool, ToolInfo, ToolResult};

/// Register all built-in tools into `registry` using the given `policy` for
/// access-control decisions.
pub fn register_builtin_tools(registry: &mut ToolRegistry, policy: Arc<SecurityPolicy>) {
    registry.register(Arc::new(ShellTool::new(policy.clone())));
    registry.register(Arc::new(FileReadTool::new(policy.clone())));
    registry.register(Arc::new(FileWriteTool::new(policy.clone())));
    registry.register(Arc::new(FileListTool::new(policy)));
}
