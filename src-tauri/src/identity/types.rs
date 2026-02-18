use serde::{Deserialize, Serialize};

/// Metadata parsed from `IDENTITY.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Default for IdentityMeta {
    fn default() -> Self {
        Self {
            name: "Claw".to_string(),
            version: "0.0.1".to_string(),
            description: "A local AI agent powered by MesoClaw.".to_string(),
        }
    }
}

/// All identity files collected into a single struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    /// Agent personality (SOUL.md)
    pub soul: String,
    /// User profile (USER.md)
    pub user: String,
    /// Operating instructions (AGENTS.md)
    pub agents: String,
    /// Agent metadata (IDENTITY.md)
    pub identity: IdentityMeta,
    /// Tool guidance (TOOLS.md)
    pub tools: String,
    /// Heartbeat checklist (HEARTBEAT.md)
    pub heartbeat: String,
    /// Boot checklist (BOOT.md)
    pub boot: String,
}

/// Metadata for listing identity files in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityFileInfo {
    pub name: String,
    pub file_name: String,
    pub description: String,
}

/// All canonical identity file names.
pub const IDENTITY_FILES: &[(&str, &str)] = &[
    ("SOUL.md", "Agent personality and core traits"),
    ("USER.md", "User profile and preferences"),
    ("AGENTS.md", "Operating instructions"),
    ("IDENTITY.md", "Agent name and metadata"),
    ("TOOLS.md", "Tool usage guidance"),
    ("HEARTBEAT.md", "Recurring heartbeat checklist"),
    ("BOOT.md", "Startup checklist"),
];
