pub mod config;
pub mod credential;
pub mod db;
pub mod error;
pub mod event_bus;
pub mod identity;
pub mod memory;
pub mod notification;
pub mod onboarding;
pub mod plugins;
pub mod security;
pub mod skills;
pub mod tools;
pub mod user;

#[cfg(feature = "ai")]
pub mod ai;
pub mod boot;
#[cfg(feature = "channels")]
pub mod channels;
#[cfg(feature = "gateway")]
pub mod gateway;
#[cfg(feature = "scheduler")]
pub mod scheduler;

pub use error::ZeniiError;

pub type Result<T> = std::result::Result<T, ZeniiError>;
