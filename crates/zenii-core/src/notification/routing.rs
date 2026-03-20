use serde::{Deserialize, Serialize};

use super::target::NotificationTarget;

/// Configurable routing: which targets receive notifications for each event type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NotificationRouting {
    pub scheduler_notification: Vec<NotificationTarget>,
    pub scheduler_job_completed: Vec<NotificationTarget>,
    pub heartbeat_alert: Vec<NotificationTarget>,
    pub channel_message: Vec<NotificationTarget>,
}

impl Default for NotificationRouting {
    fn default() -> Self {
        Self {
            scheduler_notification: vec![NotificationTarget::Toast, NotificationTarget::Desktop],
            scheduler_job_completed: vec![NotificationTarget::Toast, NotificationTarget::Desktop],
            heartbeat_alert: vec![NotificationTarget::Toast, NotificationTarget::Desktop],
            channel_message: vec![NotificationTarget::Toast, NotificationTarget::Desktop],
        }
    }
}

/// Empty slice used as a fallback for unknown event types.
static EMPTY_TARGETS: &[NotificationTarget] = &[];

impl NotificationRouting {
    /// Get targets for a given event type key string.
    pub fn targets_for(&self, event_type: &str) -> &[NotificationTarget] {
        match event_type {
            "scheduler_notification" => &self.scheduler_notification,
            "scheduler_job_completed" => &self.scheduler_job_completed,
            "heartbeat_alert" => &self.heartbeat_alert,
            "channel_message" => &self.channel_message,
            _ => EMPTY_TARGETS,
        }
    }

    /// Get only channel targets for an event type.
    pub fn channel_targets_for(&self, event_type: &str) -> Vec<&NotificationTarget> {
        self.targets_for(event_type)
            .iter()
            .filter(|t| t.is_channel())
            .collect()
    }

    /// Check if a specific target is enabled for an event type.
    pub fn has_target(&self, event_type: &str, target: &NotificationTarget) -> bool {
        self.targets_for(event_type).contains(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 8.12.5 — Default routing has toast+desktop for all event types
    #[test]
    fn default_routing() {
        let routing = NotificationRouting::default();
        let expected = vec![NotificationTarget::Toast, NotificationTarget::Desktop];
        assert_eq!(routing.scheduler_notification, expected);
        assert_eq!(routing.scheduler_job_completed, expected);
        assert_eq!(routing.channel_message, expected);
    }

    // 8.12.6 — targets_for returns correct targets for known event type
    #[test]
    fn targets_for_known() {
        let routing = NotificationRouting {
            scheduler_notification: vec![NotificationTarget::Toast, NotificationTarget::Telegram],
            ..Default::default()
        };
        let targets = routing.targets_for("scheduler_notification");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0], NotificationTarget::Toast);
        assert_eq!(targets[1], NotificationTarget::Telegram);
    }

    // 8.12.7 — targets_for returns empty for unknown event type
    #[test]
    fn targets_for_unknown() {
        let routing = NotificationRouting::default();
        let targets = routing.targets_for("unknown_event");
        assert!(targets.is_empty());
    }

    // 8.12.8 — channel_targets_for filters to channel-only targets
    #[test]
    fn channel_targets_filter() {
        let routing = NotificationRouting {
            scheduler_notification: vec![
                NotificationTarget::Toast,
                NotificationTarget::Desktop,
                NotificationTarget::Telegram,
                NotificationTarget::Slack,
            ],
            ..Default::default()
        };
        let channel_targets = routing.channel_targets_for("scheduler_notification");
        assert_eq!(channel_targets.len(), 2);
        assert_eq!(*channel_targets[0], NotificationTarget::Telegram);
        assert_eq!(*channel_targets[1], NotificationTarget::Slack);
    }

    // 8.12.9 — has_target returns true for enabled target
    #[test]
    fn has_target_enabled() {
        let routing = NotificationRouting::default();
        assert!(routing.has_target("scheduler_notification", &NotificationTarget::Toast));
        assert!(routing.has_target("scheduler_notification", &NotificationTarget::Desktop));
    }

    // 8.12.10 — has_target returns false for disabled target
    #[test]
    fn has_target_disabled() {
        let routing = NotificationRouting::default();
        assert!(!routing.has_target("scheduler_notification", &NotificationTarget::Telegram));
        assert!(!routing.has_target("unknown_event", &NotificationTarget::Toast));
    }

    // AUDIT-C5.1 — heartbeat_alert has default targets
    #[test]
    fn heartbeat_alert_default_targets() {
        let routing = NotificationRouting::default();
        let targets = routing.targets_for("heartbeat_alert");
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&NotificationTarget::Toast));
        assert!(targets.contains(&NotificationTarget::Desktop));
    }

    // AUDIT-C5.2 — heartbeat_alert has_target works correctly
    #[test]
    fn heartbeat_alert_has_target() {
        let routing = NotificationRouting::default();
        assert!(routing.has_target("heartbeat_alert", &NotificationTarget::Toast));
        assert!(routing.has_target("heartbeat_alert", &NotificationTarget::Desktop));
        assert!(!routing.has_target("heartbeat_alert", &NotificationTarget::Telegram));
    }

    // AUDIT-C5.3 — heartbeat_alert TOML deserialization
    #[test]
    fn heartbeat_alert_toml_deser() {
        let toml_str = r#"
            scheduler_notification = ["toast"]
            scheduler_job_completed = ["desktop"]
            heartbeat_alert = ["toast", "telegram"]
            channel_message = ["toast"]
        "#;
        let routing: NotificationRouting = toml::from_str(toml_str).unwrap();
        assert_eq!(routing.heartbeat_alert.len(), 2);
        assert!(routing.heartbeat_alert.contains(&NotificationTarget::Toast));
        assert!(routing.heartbeat_alert.contains(&NotificationTarget::Telegram));
    }

    // 8.12.11 — TOML deserialization with channel targets
    #[test]
    fn toml_deser_with_channels() {
        let toml_str = r#"
            scheduler_notification = ["toast", "desktop", "telegram"]
            scheduler_job_completed = ["toast", "slack"]
            channel_message = ["toast"]
        "#;
        let routing: NotificationRouting = toml::from_str(toml_str).unwrap();
        assert_eq!(routing.scheduler_notification.len(), 3);
        assert!(
            routing
                .scheduler_notification
                .contains(&NotificationTarget::Telegram)
        );
        assert_eq!(routing.scheduler_job_completed.len(), 2);
        assert!(
            routing
                .scheduler_job_completed
                .contains(&NotificationTarget::Slack)
        );
        assert_eq!(routing.channel_message.len(), 1);
    }

    // 8.12.12 — JSON serialization round-trip
    #[test]
    fn json_roundtrip() {
        let routing = NotificationRouting {
            scheduler_notification: vec![NotificationTarget::Toast, NotificationTarget::Telegram],
            scheduler_job_completed: vec![NotificationTarget::Desktop],
            heartbeat_alert: vec![NotificationTarget::Toast, NotificationTarget::Desktop],
            channel_message: vec![
                NotificationTarget::Toast,
                NotificationTarget::Desktop,
                NotificationTarget::Discord,
            ],
        };

        let json = serde_json::to_string(&routing).unwrap();
        let parsed: NotificationRouting = serde_json::from_str(&json).unwrap();
        assert_eq!(routing, parsed);
    }
}
