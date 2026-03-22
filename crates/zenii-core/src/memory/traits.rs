use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MemoryCategory {
    Core,
    Daily,
    Conversation,
    Custom(String),
}

// Serialize/Deserialize as plain strings so JSON is always `"core"`, `"daily"`, etc.
// Never `{"Custom":"foo"}` which breaks the frontend.
impl Serialize for MemoryCategory {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MemoryCategory {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(MemoryCategory::from(s.as_str()))
    }
}

impl fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Daily => write!(f, "daily"),
            Self::Conversation => write!(f, "conversation"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl From<&str> for MemoryCategory {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "core" => Self::Core,
            "daily" => Self::Daily,
            "conversation" => Self::Conversation,
            _ => Self::Custom(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub score: f32,
    pub created_at: String,
    pub updated_at: String,
}

#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> Result<()>;
    async fn recall(&self, query: &str, limit: usize, offset: usize) -> Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> Result<bool>;
    async fn store_daily(&self, content: &str) -> Result<()>;
    async fn recall_daily(&self, date: &str) -> Result<Option<String>>;
    async fn list_daily_dates(&self) -> Result<Vec<String>>;
}
