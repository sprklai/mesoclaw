//! Core types and the `Scheduler` trait for the scheduler subsystem.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── JobId ───────────────────────────────────────────────────────────────────

pub type JobId = String;

// ─── Schedule ────────────────────────────────────────────────────────────────

/// When a job runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Schedule {
    /// Run every `secs` seconds.
    Interval { secs: u64 },
    /// Run according to a 5-field cron expression (e.g. `"*/30 * * * *"`).
    Cron { expr: String },
}

// ─── SessionTarget ───────────────────────────────────────────────────────────

/// Which session context a job runs in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionTarget {
    /// The main foreground session shared with the user.
    #[default]
    Main,
    /// An isolated background session that doesn't affect the main chat.
    Isolated,
}

// ─── JobPayload ──────────────────────────────────────────────────────────────

/// What a job does when it fires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum JobPayload {
    /// Run the heartbeat checklist from `HEARTBEAT.md`.
    Heartbeat,
    /// Run an agent turn with the given prompt.
    AgentTurn { prompt: String },
    /// Publish a custom message to the event bus.
    Notify { message: String },
}

// ─── ScheduledJob ────────────────────────────────────────────────────────────

/// A registered job in the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledJob {
    pub id: JobId,
    pub name: String,
    pub schedule: Schedule,
    pub session_target: SessionTarget,
    pub payload: JobPayload,
    pub enabled: bool,
    pub error_count: u32,
    pub next_run: Option<DateTime<Utc>>,
}

// ─── JobStatus ───────────────────────────────────────────────────────────────

/// Outcome of a single job execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Success,
    Failed,
    Stuck,
    Skipped,
}

// ─── JobExecution ────────────────────────────────────────────────────────────

/// Record of one job run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobExecution {
    pub job_id: JobId,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub status: JobStatus,
    pub output: String,
}

// ─── Scheduler trait ─────────────────────────────────────────────────────────

/// Abstraction over the background job scheduler.
#[async_trait]
pub trait Scheduler: Send + Sync {
    /// Start the scheduler background task.  Idempotent.
    async fn start(&self);

    /// Stop the scheduler background task.
    async fn stop(&self);

    /// Add (or replace) a job.  Returns the assigned [`JobId`].
    async fn add_job(&self, job: ScheduledJob) -> JobId;

    /// Remove a job by id.  Returns `true` if it was found and removed.
    async fn remove_job(&self, id: &JobId) -> bool;

    /// List all registered jobs.
    async fn list_jobs(&self) -> Vec<ScheduledJob>;

    /// Retrieve execution history for a job (most recent first).
    async fn job_history(&self, id: &JobId) -> Vec<JobExecution>;
}
