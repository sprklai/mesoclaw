pub mod client;
pub mod convert;
pub mod server;

#[cfg(feature = "mcp-client")]
pub use client::{McpClientManager, McpToolInfo};
pub use server::ZeniiMcpServer;
