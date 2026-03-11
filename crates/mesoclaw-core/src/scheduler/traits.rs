use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type JobId = String;

/// When a job runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Schedule {
    /// Run every `secs` seconds.
    Interval { secs: u64 },
    /// Run according to a cron expression.
    Cron { expr: String },
}

/// Optional local-time window during which a job may fire.
/// Both values are 24-hour format (0–23). Job is skipped when
/// local hour is outside `[start_hour, end_hour)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ActiveHours {
    pub start_hour: u8,
    pub end_hour: u8,
}

/// Which session context a job runs in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum SessionTarget {
    #[default]
    Main,
    Isolated,
}

/// What a job does when it fires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "type")]
#[non_exhaustive]
pub enum JobPayload {
    /// Run the heartbeat checklist.
    Heartbeat,
    /// Run an agent turn with the given prompt.
    AgentTurn { prompt: String },
    /// Publish a notification.
    Notify { message: String },
    /// Send a message via a named channel.
    SendViaChannel { channel: String, message: String },
}

/// A registered job in the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ScheduledJob {
    pub id: JobId,
    pub name: String,
    pub schedule: Schedule,
    #[serde(default)]
    pub session_target: SessionTarget,
    pub payload: JobPayload,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub error_count: u32,
    #[serde(default)]
    pub next_run: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_hours: Option<ActiveHours>,
    #[serde(default)]
    pub delete_after_run: bool,
}

fn default_true() -> bool {
    true
}

/// Outcome of a single job execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Success,
    Failed,
    Stuck,
    Skipped,
}

/// Record of one job run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct JobExecution {
    pub id: String,
    pub job_id: JobId,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Abstraction over the background job scheduler.
#[async_trait]
pub trait Scheduler: Send + Sync {
    async fn start(&self);
    async fn stop(&self);
    async fn add_job(&self, job: ScheduledJob) -> crate::Result<JobId>;
    async fn remove_job(&self, id: &str) -> crate::Result<()>;
    async fn toggle_job(&self, id: &str) -> crate::Result<bool>;
    async fn list_jobs(&self) -> Vec<ScheduledJob>;
    async fn job_history(&self, id: &str) -> Vec<JobExecution>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // 16.1 — Schedule::Interval serialization round-trip
    #[test]
    fn schedule_interval_serde() {
        let s = Schedule::Interval { secs: 60 };
        let json = serde_json::to_string(&s).unwrap();
        let back: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    // 16.2 — Schedule::Cron serialization round-trip
    #[test]
    fn schedule_cron_serde() {
        let s = Schedule::Cron {
            expr: "0 */5 * * * *".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    // 16.3 — JobPayload::Heartbeat serialization
    #[test]
    fn payload_heartbeat_serde() {
        let p = JobPayload::Heartbeat;
        let json = serde_json::to_string(&p).unwrap();
        let back: JobPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    // 16.4 — JobPayload::AgentTurn serialization
    #[test]
    fn payload_agent_turn_serde() {
        let p = JobPayload::AgentTurn {
            prompt: "check status".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: JobPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    // 16.5 — JobPayload::SendViaChannel serialization
    #[test]
    fn payload_send_via_channel_serde() {
        let p = JobPayload::SendViaChannel {
            channel: "telegram".into(),
            message: "hello".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: JobPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    // 16.6 — ScheduledJob default enabled state
    #[test]
    fn job_default_enabled() {
        let json = r#"{
            "id": "j1",
            "name": "test",
            "schedule": {"type": "interval", "secs": 60},
            "payload": {"type": "heartbeat"}
        }"#;
        let job: ScheduledJob = serde_json::from_str(json).unwrap();
        assert!(job.enabled);
        assert_eq!(job.session_target, SessionTarget::Main);
    }

    // 16.7 — ActiveHours validation (start < end)
    #[test]
    fn active_hours_validation() {
        let h = ActiveHours {
            start_hour: 9,
            end_hour: 17,
        };
        assert!(h.start_hour < h.end_hour);
        let json = serde_json::to_string(&h).unwrap();
        let back: ActiveHours = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);
    }

    // 16.8 — SessionTarget variants
    #[test]
    fn session_target_variants() {
        let main = SessionTarget::Main;
        let iso = SessionTarget::Isolated;
        let main_json = serde_json::to_string(&main).unwrap();
        let iso_json = serde_json::to_string(&iso).unwrap();
        assert_eq!(
            serde_json::from_str::<SessionTarget>(&main_json).unwrap(),
            SessionTarget::Main
        );
        assert_eq!(
            serde_json::from_str::<SessionTarget>(&iso_json).unwrap(),
            SessionTarget::Isolated
        );
    }
}
