use tokio::sync::broadcast;

use super::traits::{AppEvent, EventBus, EventFilter};

const DEFAULT_CAPACITY: usize = 1024;

/// [`EventBus`] implementation backed by a `tokio::sync::broadcast` channel.
pub struct TokioBroadcastBus {
    sender: broadcast::Sender<AppEvent>,
}

impl TokioBroadcastBus {
    /// Create with the default channel capacity (1024).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create with an explicit channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

impl Default for TokioBroadcastBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for TokioBroadcastBus {
    fn publish(&self, event: AppEvent) -> Result<(), String> {
        // `send` returns the number of active receivers (may be 0 — that is fine).
        self.sender
            .send(event)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    fn subscribe_filtered(&self, _filter: EventFilter) -> broadcast::Receiver<AppEvent> {
        // The underlying broadcast channel delivers all events; consumers must
        // apply EventFilter::matches() to discard unwanted messages.
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::traits::EventType;

    #[tokio::test]
    async fn publish_then_receive() {
        let bus = TokioBroadcastBus::new();
        let mut rx = bus.subscribe();

        let event = AppEvent::SystemReady;
        bus.publish(event).expect("publish failed");

        let received = rx.recv().await.expect("recv failed");
        assert!(matches!(received, AppEvent::SystemReady));
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        let bus = TokioBroadcastBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.publish(AppEvent::SystemReady).unwrap();

        assert!(matches!(rx1.recv().await.unwrap(), AppEvent::SystemReady));
        assert!(matches!(rx2.recv().await.unwrap(), AppEvent::SystemReady));
    }

    #[tokio::test]
    async fn event_fields_round_trip() {
        let bus = TokioBroadcastBus::new();
        let mut rx = bus.subscribe();

        let event = AppEvent::SystemError {
            message: "disk full".to_string(),
        };
        bus.publish(event).unwrap();

        match rx.recv().await.unwrap() {
            AppEvent::SystemError { message } => assert_eq!(message, "disk full"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn subscribe_filtered_returns_receiver() {
        let bus = TokioBroadcastBus::new();
        let filter = EventFilter::new(vec![EventType::SystemReady]);
        let mut rx = bus.subscribe_filtered(filter.clone());

        bus.publish(AppEvent::SystemReady).unwrap();
        let event = rx.recv().await.unwrap();
        assert!(filter.matches(&event));
    }

    #[tokio::test]
    async fn lagged_receiver_reports_error() {
        // Capacity-1 bus so the second publish lags a subscriber that hasn't consumed yet.
        let bus = TokioBroadcastBus::with_capacity(1);
        let mut slow_rx = bus.subscribe();

        // Fill the channel and overflow it.
        bus.publish(AppEvent::SystemReady).unwrap();
        bus.publish(AppEvent::SystemError {
            message: "overflow".to_string(),
        })
        .unwrap();

        // The first receive on a lagged receiver returns a Lagged error.
        let result = slow_rx.recv().await;
        assert!(
            matches!(result, Err(broadcast::error::RecvError::Lagged(_))),
            "expected Lagged, got {result:?}"
        );
    }
}
