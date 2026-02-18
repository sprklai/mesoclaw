//! Channel abstraction layer for MesoClaw inter-agent messaging.
//!
//! A **channel** is any transport layer that delivers [`traits::ChannelMessage`]s
//! between the agent runtime and an external peer (desktop user, webhook, Telegram, etc.).
//!
//! # Architecture
//!
//! ```text
//!   EventBus / HTTP webhook / Telegram API
//!         │
//!         ▼
//!   ┌─────────────┐
//!   │  Channel    │  (impl: TauriIpcChannel, WebhookChannel, …)
//!   └──────┬──────┘
//!          │ mpsc::Sender<ChannelMessage>
//!          ▼
//!   ┌─────────────────┐
//!   │  ChannelManager │  aggregates all channels into one receiver
//!   └──────┬──────────┘
//!          │ mpsc::Receiver<ChannelMessage>
//!          ▼
//!       Agent loop
//! ```
//!
//! # Registering a new channel
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use crate::channels::{ChannelManager, TauriIpcChannel};
//! use crate::event_bus::TokioBroadcastBus;
//!
//! let bus = Arc::new(TokioBroadcastBus::new());
//! let mgr = ChannelManager::new();
//! mgr.register(Arc::new(TauriIpcChannel::new(bus))).await.unwrap();
//! let (rx, _handles) = mgr.start_all(64).await;
//! // poll rx in the agent loop
//! ```

pub mod manager;
pub mod tauri_ipc;
pub mod traits;

pub use manager::ChannelManager;
pub use tauri_ipc::TauriIpcChannel;
pub use traits::{Channel, ChannelEvent, ChannelMessage};
