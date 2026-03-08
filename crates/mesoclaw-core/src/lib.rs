pub mod config;
pub mod credential;
pub mod db;
pub mod error;
pub mod event_bus;
pub mod identity;
pub mod memory;
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

pub use error::MesoError;

pub type Result<T> = std::result::Result<T, MesoError>;
