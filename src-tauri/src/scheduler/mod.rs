//! Background job scheduler for the MesoClaw agent.
//!
//! # Architecture
//! ```text
//! Scheduler (trait)
//!   └── TokioScheduler          ← Tokio-driven in-memory scheduler
//!         ├── Schedule           ← Interval | Cron
//!         ├── JobPayload         ← Heartbeat | AgentTurn | Notify
//!         └── heartbeat          ← Parses HEARTBEAT.md checklist items
//! ```
//!
//! # Key behaviours
//! - Ticks every second; executes due jobs concurrently via `tokio::spawn`
//! - Stuck detection: jobs running > 120 s emit `SystemError` via `EventBus`
//! - Error back-off: `30s → 60s → 300s → 900s → 3600s`
//! - History ring-buffer: last 100 executions per job
//!
//! # IPC commands
//! - [`commands::list_jobs_command`]
//! - [`commands::create_job_command`]
//! - [`commands::toggle_job_command`]
//! - [`commands::delete_job_command`]
//! - [`commands::job_history_command`]

pub mod commands;
pub mod heartbeat;
pub mod tokio_scheduler;
pub mod traits;

pub use commands::{
    create_job_command, delete_job_command, job_history_command, list_jobs_command,
    toggle_job_command,
};
pub use heartbeat::{
    DEFAULT_HEARTBEAT_INTERVAL_SECS, ERROR_BACKOFF_SECS, backoff_secs, parse_heartbeat_items,
};
pub use tokio_scheduler::TokioScheduler;
pub use traits::{
    JobExecution, JobId, JobPayload, JobStatus, Schedule, ScheduledJob, Scheduler, SessionTarget,
};
