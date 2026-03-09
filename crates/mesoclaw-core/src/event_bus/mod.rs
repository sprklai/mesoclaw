use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    SessionCreated {
        session_id: String,
    },
    SessionDeleted {
        session_id: String,
    },
    MessageReceived {
        session_id: String,
        role: String,
    },
    StreamChunk {
        session_id: String,
        content: String,
    },
    StreamDone {
        session_id: String,
    },
    ToolExecutionStarted {
        tool_name: String,
    },
    ToolExecutionCompleted {
        tool_name: String,
        success: bool,
    },
    ProviderChanged {
        provider: String,
        model: String,
    },
    MemoryStored {
        key: String,
    },
    ConfigUpdated,
    GatewayStarted {
        port: u16,
    },
    ChannelConnected {
        channel: String,
    },
    ChannelDisconnected {
        channel: String,
        reason: String,
    },
    ChannelMessageReceived {
        channel: String,
        sender: String,
    },
    HeartbeatTick {
        job_id: String,
    },
    CronFired {
        job_id: String,
        name: String,
    },
    HeartbeatAlert {
        message: String,
    },
    SchedulerStarted,
    SchedulerStopped,
    SchedulerNotification {
        job_id: String,
        job_name: String,
        message: String,
    },
    SchedulerJobCompleted {
        job_id: String,
        job_name: String,
        status: String,
        error: Option<String>,
    },
    Shutdown,
}

#[async_trait]
pub trait EventBus: Send + Sync {
    fn publish(&self, event: AppEvent) -> Result<()>;
    fn subscribe(&self) -> broadcast::Receiver<AppEvent>;
}

pub struct TokioBroadcastBus {
    sender: broadcast::Sender<AppEvent>,
}

impl TokioBroadcastBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

#[async_trait]
impl EventBus for TokioBroadcastBus {
    fn publish(&self, event: AppEvent) -> Result<()> {
        // Ignore error when there are no subscribers — this is expected during startup
        let _ = self.sender.send(event);
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::ConfigUpdated).unwrap();

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AppEvent::ConfigUpdated));
    }

    #[tokio::test]
    async fn multi_subscriber_fanout() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.publish(AppEvent::Shutdown).unwrap();

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert!(matches!(e1, AppEvent::Shutdown));
        assert!(matches!(e2, AppEvent::Shutdown));
    }

    #[tokio::test]
    async fn channel_connected_event() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::ChannelConnected {
            channel: "telegram".into(),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AppEvent::ChannelConnected { channel } if channel == "telegram"));
    }

    // 16.40 — HeartbeatTick event serialization
    #[tokio::test]
    async fn heartbeat_tick_event() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::HeartbeatTick {
            job_id: "j1".into(),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AppEvent::HeartbeatTick { job_id } if job_id == "j1"));
    }

    // 16.41 — CronFired event serialization
    #[tokio::test]
    async fn cron_fired_event() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::CronFired {
            job_id: "j2".into(),
            name: "daily_check".into(),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(
            matches!(event, AppEvent::CronFired { job_id, name } if job_id == "j2" && name == "daily_check")
        );
    }

    // 8.6.1.1 — SchedulerNotification event serde round-trip
    #[tokio::test]
    async fn scheduler_notification_event() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::SchedulerNotification {
            job_id: "j1".into(),
            job_name: "daily_check".into(),
            message: "All systems go".into(),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(
            matches!(event, AppEvent::SchedulerNotification { job_id, job_name, message }
                if job_id == "j1" && job_name == "daily_check" && message == "All systems go")
        );
    }

    // 8.6.1.1b — SchedulerNotification JSON serde round-trip
    #[test]
    fn scheduler_notification_event_serde() {
        let event = AppEvent::SchedulerNotification {
            job_id: "j1".into(),
            job_name: "daily_check".into(),
            message: "All systems go".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: AppEvent = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(back, AppEvent::SchedulerNotification { job_id, job_name, message }
                if job_id == "j1" && job_name == "daily_check" && message == "All systems go")
        );
    }

    // 8.6.1.2 — SchedulerJobCompleted event serde round-trip
    #[tokio::test]
    async fn scheduler_job_completed_event() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::SchedulerJobCompleted {
            job_id: "j2".into(),
            job_name: "heartbeat".into(),
            status: "success".into(),
            error: None,
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(
            matches!(event, AppEvent::SchedulerJobCompleted { job_id, job_name, status, error }
                if job_id == "j2" && job_name == "heartbeat" && status == "success" && error.is_none())
        );
    }

    // 8.6.1.2b — SchedulerJobCompleted JSON serde round-trip
    #[test]
    fn scheduler_job_completed_event_serde() {
        let event = AppEvent::SchedulerJobCompleted {
            job_id: "j2".into(),
            job_name: "heartbeat".into(),
            status: "success".into(),
            error: Some("timeout".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: AppEvent = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(back, AppEvent::SchedulerJobCompleted { job_id, job_name, status, error }
                if job_id == "j2" && job_name == "heartbeat" && status == "success" && error == Some("timeout".into()))
        );
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_ok() {
        let bus = TokioBroadcastBus::new(16);
        let result = bus.publish(AppEvent::ConfigUpdated);
        assert!(result.is_ok());
    }
}
