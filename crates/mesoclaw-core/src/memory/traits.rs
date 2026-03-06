use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum MemoryCategory {
    Core,
    Daily,
    Conversation,
    Custom(String),
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
        match s {
            "core" => Self::Core,
            "daily" => Self::Daily,
            "conversation" => Self::Conversation,
            other => Self::Custom(other.to_string()),
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
