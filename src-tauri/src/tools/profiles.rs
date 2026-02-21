//! Tool profiles and groups for access control.
//!
//! Provides a profile-based filtering mechanism so different agents can have
//! different levels of tool access.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Logical grouping of tools by capability area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolGroup {
    /// Runtime/container tools (shell execution, process management).
    Runtime,
    /// Filesystem tools (read, write, list).
    Fs,
    /// Session management tools (spawn, list, kill sessions).
    Sessions,
    /// Memory tools (store, recall, embeddings).
    Memory,
    /// Web/network tools (fetch, HTTP requests).
    Web,
    /// UI interaction tools (dialogs, notifications).
    Ui,
}

impl ToolGroup {
    /// Returns the tools that belong to this group.
    pub fn tools(&self) -> &'static [&'static str] {
        match self {
            ToolGroup::Runtime => &["shell", "process"],
            ToolGroup::Fs => &["file_read", "file_write", "file_list"],
            ToolGroup::Sessions => &["sessions_spawn", "sessions_list", "sessions_kill"],
            ToolGroup::Memory => &["memory_store", "memory_recall", "memory_forget"],
            ToolGroup::Web => &["web_fetch", "web_request"],
            ToolGroup::Ui => &["ui_dialog", "ui_notify", "ui_prompt"],
        }
    }

    /// Try to determine the group for a tool by name.
    pub fn from_tool_name(name: &str) -> Option<ToolGroup> {
        for group in [
            ToolGroup::Runtime,
            ToolGroup::Fs,
            ToolGroup::Sessions,
            ToolGroup::Memory,
            ToolGroup::Web,
            ToolGroup::Ui,
        ] {
            if group.tools().contains(&name) {
                return Some(group);
            }
        }
        None
    }
}

/// Predefined tool access profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolProfile {
    /// Minimal tools only - safe for restricted agents.
    /// Includes: basic file read, memory recall.
    Minimal,

    /// Development-focused tools.
    /// Includes: Runtime, Fs, Memory.
    Coding,

    /// Communication-focused tools.
    /// Includes: Memory, Web, Ui.
    Messaging,

    /// Full access to all tools.
    #[default]
    Full,
}

impl ToolProfile {
    /// Returns the tool groups allowed by this profile.
    pub fn allowed_groups(&self) -> Vec<ToolGroup> {
        match self {
            ToolProfile::Minimal => vec![ToolGroup::Memory],
            ToolProfile::Coding => vec![ToolGroup::Runtime, ToolGroup::Fs, ToolGroup::Memory],
            ToolProfile::Messaging => vec![ToolGroup::Memory, ToolGroup::Web, ToolGroup::Ui],
            ToolProfile::Full => vec![
                ToolGroup::Runtime,
                ToolGroup::Fs,
                ToolGroup::Sessions,
                ToolGroup::Memory,
                ToolGroup::Web,
                ToolGroup::Ui,
            ],
        }
    }

    /// Returns the set of allowed tool names for this profile.
    ///
    /// For Minimal profile, this includes read-only operations like file_read
    /// but excludes destructive operations like file_write.
    pub fn allowed_tools(&self) -> HashSet<&'static str> {
        let mut tools: HashSet<&'static str> = self
            .allowed_groups()
            .iter()
            .flat_map(|g| g.tools().iter().copied())
            .collect();

        // Minimal profile gets file_read but not file_write/file_list
        if *self == ToolProfile::Minimal {
            tools.insert("file_read");
            tools.insert("file_list");
        }

        tools
    }

    /// Check if a tool name is allowed by this profile.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        // If the tool isn't in any known group, allow it by default
        // (backwards compatibility for custom tools)
        if ToolGroup::from_tool_name(tool_name).is_none() {
            return true;
        }
        self.allowed_tools().contains(tool_name)
    }

    /// Returns a human-readable description of this profile.
    pub fn description(&self) -> &'static str {
        match self {
            ToolProfile::Minimal => {
                "Minimal access: basic file reading and memory recall only"
            }
            ToolProfile::Coding => {
                "Development access: shell, filesystem, and memory tools"
            }
            ToolProfile::Messaging => {
                "Communication access: memory, web, and UI interaction tools"
            }
            ToolProfile::Full => "Full access: all available tools",
        }
    }

    /// Returns all available profiles.
    pub fn all() -> &'static [ToolProfile] {
        &[
            ToolProfile::Minimal,
            ToolProfile::Coding,
            ToolProfile::Messaging,
            ToolProfile::Full,
        ]
    }
}

impl std::fmt::Display for ToolProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolProfile::Minimal => write!(f, "minimal"),
            ToolProfile::Coding => write!(f, "coding"),
            ToolProfile::Messaging => write!(f, "messaging"),
            ToolProfile::Full => write!(f, "full"),
        }
    }
}

impl std::str::FromStr for ToolProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(ToolProfile::Minimal),
            "coding" => Ok(ToolProfile::Coding),
            "messaging" => Ok(ToolProfile::Messaging),
            "full" => Ok(ToolProfile::Full),
            _ => Err(format!(
                "Unknown tool profile: {}. Valid options: minimal, coding, messaging, full",
                s
            )),
        }
    }
}

impl std::fmt::Display for ToolGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolGroup::Runtime => write!(f, "runtime"),
            ToolGroup::Fs => write!(f, "fs"),
            ToolGroup::Sessions => write!(f, "sessions"),
            ToolGroup::Memory => write!(f, "memory"),
            ToolGroup::Web => write!(f, "web"),
            ToolGroup::Ui => write!(f, "ui"),
        }
    }
}

impl std::str::FromStr for ToolGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "runtime" => Ok(ToolGroup::Runtime),
            "fs" => Ok(ToolGroup::Fs),
            "sessions" => Ok(ToolGroup::Sessions),
            "memory" => Ok(ToolGroup::Memory),
            "web" => Ok(ToolGroup::Web),
            "ui" => Ok(ToolGroup::Ui),
            _ => Err(format!(
                "Unknown tool group: {}. Valid options: runtime, fs, sessions, memory, web, ui",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_profile_allowed_tools() {
        let minimal = ToolProfile::Minimal;
        assert!(minimal.is_tool_allowed("file_read"));
        assert!(minimal.is_tool_allowed("memory_recall"));
        assert!(!minimal.is_tool_allowed("shell"));
        assert!(!minimal.is_tool_allowed("file_write"));

        let coding = ToolProfile::Coding;
        assert!(coding.is_tool_allowed("shell"));
        assert!(coding.is_tool_allowed("file_read"));
        assert!(coding.is_tool_allowed("file_write"));
        assert!(!coding.is_tool_allowed("web_fetch"));
        assert!(!coding.is_tool_allowed("ui_notify"));

        let messaging = ToolProfile::Messaging;
        assert!(messaging.is_tool_allowed("memory_store"));
        assert!(messaging.is_tool_allowed("web_fetch"));
        assert!(messaging.is_tool_allowed("ui_notify"));
        assert!(!messaging.is_tool_allowed("shell"));
        assert!(!messaging.is_tool_allowed("file_write"));

        let full = ToolProfile::Full;
        assert!(full.is_tool_allowed("shell"));
        assert!(full.is_tool_allowed("file_read"));
        assert!(full.is_tool_allowed("memory_store"));
        assert!(full.is_tool_allowed("web_fetch"));
        assert!(full.is_tool_allowed("ui_notify"));
        assert!(full.is_tool_allowed("sessions_spawn"));
    }

    #[test]
    fn test_unknown_tool_allowed_by_default() {
        // Unknown tools should be allowed by default for backwards compatibility
        let minimal = ToolProfile::Minimal;
        assert!(minimal.is_tool_allowed("custom_tool_xyz"));
        assert!(minimal.is_tool_allowed("future_tool"));
    }

    #[test]
    fn test_tool_group_from_name() {
        assert_eq!(ToolGroup::from_tool_name("shell"), Some(ToolGroup::Runtime));
        assert_eq!(
            ToolGroup::from_tool_name("file_read"),
            Some(ToolGroup::Fs)
        );
        assert_eq!(
            ToolGroup::from_tool_name("sessions_spawn"),
            Some(ToolGroup::Sessions)
        );
        assert_eq!(
            ToolGroup::from_tool_name("memory_store"),
            Some(ToolGroup::Memory)
        );
        assert_eq!(
            ToolGroup::from_tool_name("web_fetch"),
            Some(ToolGroup::Web)
        );
        assert_eq!(ToolGroup::from_tool_name("ui_notify"), Some(ToolGroup::Ui));
        assert_eq!(ToolGroup::from_tool_name("unknown"), None);
    }

    #[test]
    fn test_profile_from_str() {
        assert_eq!(ToolProfile::from_str("minimal").unwrap(), ToolProfile::Minimal);
        assert_eq!(ToolProfile::from_str("coding").unwrap(), ToolProfile::Coding);
        assert_eq!(
            ToolProfile::from_str("messaging").unwrap(),
            ToolProfile::Messaging
        );
        assert_eq!(ToolProfile::from_str("full").unwrap(), ToolProfile::Full);
        assert!(ToolProfile::from_str("invalid").is_err());
    }

    #[test]
    fn test_profile_all() {
        let all = ToolProfile::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&ToolProfile::Minimal));
        assert!(all.contains(&ToolProfile::Coding));
        assert!(all.contains(&ToolProfile::Messaging));
        assert!(all.contains(&ToolProfile::Full));
    }

    #[test]
    fn test_serde_roundtrip() {
        let profile = ToolProfile::Coding;
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(json, "\"coding\"");
        let deserialized: ToolProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, profile);

        let group = ToolGroup::Fs;
        let json = serde_json::to_string(&group).unwrap();
        assert_eq!(json, "\"fs\"");
        let deserialized: ToolGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, group);
    }
}
