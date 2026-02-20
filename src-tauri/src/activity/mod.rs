//! Activity tracking for the dashboard.
//!
//! Provides a lightweight ring buffer that subscribes to EventBus events
//! and maintains a rolling history of recent activities for display.

pub mod commands;

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::event_bus::{AppEvent, EventBus, EventFilter, EventType};

/// Maximum number of activities to retain in the buffer.
const DEFAULT_MAX_SIZE: usize = 500;

/// Source of an activity event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivitySource {
    Agent,
    Scheduler,
    System,
    Channel,
}

/// Activity status indicates the current state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivityStatus {
    // Active states
    Running,
    Awaiting,
    Pending,
    Paused,
    // Terminal states
    Success,
    Error,
    Cancelled,
    Terminated,
    Stuck,
    Skipped,
}

/// An individual activity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Unique identifier.
    pub id: String,
    /// Source of the activity.
    pub source: ActivitySource,
    /// Short, actionable title.
    pub title: String,
    /// Current status.
    pub status: ActivityStatus,
    /// ISO timestamp when activity started.
    pub started_at: String,
    /// ISO timestamp when activity completed (optional for active).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// Navigation path for related page (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_to: Option<String>,
}

/// A scheduled job that hasn't started yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedJob {
    /// Job identifier.
    pub id: String,
    /// Job name.
    pub name: String,
    /// ISO timestamp of next scheduled run.
    pub next_run: String,
    /// Navigation path to job details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_to: Option<String>,
}

/// Ring buffer for tracking recent activities.
pub struct ActivityBuffer {
    events: RwLock<VecDeque<Activity>>,
    max_size: usize,
}

impl ActivityBuffer {
    /// Create a new ActivityBuffer with the given max size.
    pub fn new(max_size: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    /// Create with default size (500 events).
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_MAX_SIZE)
    }

    /// Subscribe to the event bus and process events into activities.
    pub fn subscribe_to_bus(self: Arc<Self>, bus: Arc<dyn EventBus>) {
        tokio::spawn(async move {
            let mut rx = bus.subscribe_filtered(EventFilter::new(vec![
                EventType::AgentToolStart,
                EventType::AgentToolResult,
                EventType::AgentStarted,
                EventType::AgentComplete,
                EventType::ApprovalNeeded,
                EventType::HeartbeatTick,
                EventType::HeartbeatAlert,
                EventType::CronFired,
                EventType::ChannelMessage,
                EventType::MemoryStored,
                EventType::SystemError,
            ]));

            use tokio::sync::broadcast::error::RecvError;
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Some(activity) = Self::event_to_activity(&event) {
                            self.push(activity).await;
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        log::warn!("ActivityBuffer: lagged {n} events");
                    }
                    Err(RecvError::Closed) => {
                        log::info!("ActivityBuffer: event bus closed, exiting");
                        break;
                    }
                }
            }
        });
    }

    /// Convert an AppEvent to an Activity.
    fn event_to_activity(event: &AppEvent) -> Option<Activity> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = || chrono::Utc::now().to_rfc3339();

        match event {
            AppEvent::AgentToolStart { tool_name, .. } => Some(Activity {
                id,
                source: ActivitySource::Agent,
                title: format!("Running: {tool_name}"),
                status: ActivityStatus::Running,
                started_at: now(),
                completed_at: None,
                link_to: None,
            }),
            AppEvent::AgentToolResult {
                tool_name, success, ..
            } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::Agent,
                    title: tool_name.clone(),
                    status: if *success {
                        ActivityStatus::Success
                    } else {
                        ActivityStatus::Error
                    },
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: None,
                })
            }
            AppEvent::AgentStarted { session_id } => Some(Activity {
                id,
                source: ActivitySource::Agent,
                title: "Agent started".to_string(),
                status: ActivityStatus::Running,
                started_at: now(),
                completed_at: None,
                link_to: Some(format!("/chat?session={}", session_id)),
            }),
            AppEvent::AgentComplete { message, .. } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::Agent,
                    title: if message.len() > 50 {
                        format!("{}...", &message[..50])
                    } else {
                        "Agent completed".to_string()
                    },
                    status: ActivityStatus::Success,
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: None,
                })
            }
            AppEvent::ApprovalNeeded { tool_name, .. } => Some(Activity {
                id,
                source: ActivitySource::Agent,
                title: format!("Approval: {tool_name}"),
                status: ActivityStatus::Awaiting,
                started_at: now(),
                completed_at: None,
                link_to: Some("/settings?tab=approvals".to_string()),
            }),
            AppEvent::HeartbeatTick { .. } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::Scheduler,
                    title: "Heartbeat check".to_string(),
                    status: ActivityStatus::Success,
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: Some("/settings?tab=scheduler".to_string()),
                })
            }
            AppEvent::HeartbeatAlert { content } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::Scheduler,
                    title: if content.len() > 50 {
                        format!("Alert: {}...", &content[..47])
                    } else {
                        format!("Alert: {content}")
                    },
                    status: ActivityStatus::Success,
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: Some("/settings?tab=scheduler".to_string()),
                })
            }
            AppEvent::CronFired { job_id, schedule } => Some(Activity {
                id,
                source: ActivitySource::Scheduler,
                title: format!("Job fired: {schedule}"),
                status: ActivityStatus::Running,
                started_at: now(),
                completed_at: None,
                link_to: Some(format!("/settings?tab=scheduler&job={}", job_id)),
            }),
            AppEvent::ChannelMessage { channel, from, .. } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::Channel,
                    title: format!("Message from {from} ({channel})"),
                    status: ActivityStatus::Success,
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: Some("/channels".to_string()),
                })
            }
            AppEvent::MemoryStored { summary, .. } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::System,
                    title: if summary.len() > 50 {
                        format!("Memory: {}...", &summary[..47])
                    } else {
                        format!("Memory: {summary}")
                    },
                    status: ActivityStatus::Success,
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: Some("/memory".to_string()),
                })
            }
            AppEvent::SystemError { message } => {
                let ts = now();
                Some(Activity {
                    id,
                    source: ActivitySource::System,
                    title: if message.len() > 50 {
                        format!("Error: {}...", &message[..47])
                    } else {
                        format!("Error: {message}")
                    },
                    status: ActivityStatus::Error,
                    started_at: ts.clone(),
                    completed_at: Some(ts),
                    link_to: None,
                })
            }
            _ => None,
        }
    }

    /// Add an activity to the buffer.
    async fn push(&self, activity: Activity) {
        let mut events = self.events.write().await;
        if events.len() >= self.max_size {
            events.pop_front();
        }
        events.push_back(activity);
    }

    /// Get activities within a time window (default 1 hour).
    pub async fn get_recent(&self, within_ms: u64) -> Vec<Activity> {
        let events = self.events.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let cutoff = now.saturating_sub(within_ms);

        events
            .iter()
            .filter(|a| {
                // Parse started_at timestamp and compare
                chrono::DateTime::parse_from_rfc3339(&a.started_at)
                    .map(|dt| dt.timestamp_millis() as u64 >= cutoff)
                    .unwrap_or(true) // Include if we can't parse
            })
            .cloned()
            .collect()
    }
}
