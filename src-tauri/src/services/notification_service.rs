//! Desktop notification routing service.
//!
//! `NotificationService` subscribes to the [`EventBus`] and routes
//! notification-worthy events to the OS desktop notification system via
//! `tauri-plugin-notification`.
//!
//! ## Event routing
//! | Event               | Category    | Priority |
//! |---------------------|-------------|----------|
//! | `HeartbeatTick`     | heartbeat   | low      |
//! | `CronFired`         | cron        | normal   |
//! | `AgentComplete`     | agent       | normal   |
//! | `ApprovalNeeded`    | approval    | high     |
//! | `SystemError`       | system      | high     |
//!
//! ## Preferences
//! Each category can be independently enabled/disabled.  A global "Do Not
//! Disturb" mode suppresses all notifications.

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::event_bus::{AppEvent, EventBus};

#[cfg(feature = "desktop")]
use tauri_plugin_notification::NotificationExt;

// ─── NotificationCategory ────────────────────────────────────────────────────

/// Notification category — used for per-category preference settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    Heartbeat,
    Cron,
    Agent,
    Approval,
    System,
}

// ─── NotificationSpec ────────────────────────────────────────────────────────

/// A notification ready to be displayed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSpec {
    pub title: String,
    pub body: String,
    pub category: NotificationCategory,
    /// Optional identifier for click-to-open navigation.
    pub session_id: Option<String>,
}

// ─── NotificationConfig ──────────────────────────────────────────────────────

/// Per-category preferences and global Do Not Disturb flag.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    /// When `true`, all notifications are suppressed regardless of category.
    pub do_not_disturb: bool,
    /// Per-category enable flag.  Missing keys default to `true` (enabled).
    pub categories: std::collections::HashMap<String, bool>,
}

impl NotificationConfig {
    /// Return `true` if notifications for `category` should be displayed.
    pub fn is_enabled_for(&self, category: NotificationCategory) -> bool {
        if self.do_not_disturb {
            return false;
        }
        let key = format!("{category:?}").to_lowercase();
        *self.categories.get(&key).unwrap_or(&true)
    }

    /// Set the enabled state for a category.
    pub fn set_enabled(&mut self, category: NotificationCategory, enabled: bool) {
        let key = format!("{category:?}").to_lowercase();
        self.categories.insert(key, enabled);
    }
}

// ─── event_to_notification ───────────────────────────────────────────────────

/// Map an [`AppEvent`] to a [`NotificationSpec`], or `None` if the event
/// should not trigger a notification.
pub fn event_to_notification(event: &AppEvent) -> Option<NotificationSpec> {
    match event {
        AppEvent::HeartbeatTick { timestamp } => Some(NotificationSpec {
            title: "Heartbeat".to_string(),
            body: format!("Heartbeat tick at {timestamp}"),
            category: NotificationCategory::Heartbeat,
            session_id: None,
        }),
        AppEvent::CronFired { job_id, schedule } => Some(NotificationSpec {
            title: "Scheduled Job".to_string(),
            body: format!("Job '{job_id}' fired ({schedule})"),
            category: NotificationCategory::Cron,
            session_id: Some(job_id.clone()),
        }),
        AppEvent::AgentComplete {
            session_id,
            message,
        } => Some(NotificationSpec {
            title: "Agent Task Complete".to_string(),
            body: message.chars().take(120).collect(),
            category: NotificationCategory::Agent,
            session_id: Some(session_id.clone()),
        }),
        AppEvent::ApprovalNeeded {
            action_id,
            tool_name,
            description,
            ..
        } => Some(NotificationSpec {
            title: format!("Approval Required: {tool_name}"),
            body: description.chars().take(120).collect(),
            category: NotificationCategory::Approval,
            session_id: Some(action_id.clone()),
        }),
        AppEvent::SystemError { message } => Some(NotificationSpec {
            title: "System Error".to_string(),
            body: message.chars().take(120).collect(),
            category: NotificationCategory::System,
            session_id: None,
        }),
        // All other events do not trigger notifications.
        _ => None,
    }
}

// ─── NotificationService ─────────────────────────────────────────────────────

/// Subscribes to the event bus and routes events to OS desktop notifications.
pub struct NotificationService {
    config: Arc<RwLock<NotificationConfig>>,
    bus: Arc<dyn EventBus>,
    /// When running inside the Tauri desktop shell, this handle is used to
    /// dispatch real OS notifications via `tauri-plugin-notification`.
    #[cfg(feature = "desktop")]
    app_handle: Option<tauri::AppHandle>,
}

impl NotificationService {
    pub fn new(bus: Arc<dyn EventBus>) -> Self {
        Self {
            config: Arc::new(RwLock::new(NotificationConfig::default())),
            bus,
            #[cfg(feature = "desktop")]
            app_handle: None,
        }
    }

    pub fn with_config(bus: Arc<dyn EventBus>, config: NotificationConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            bus,
            #[cfg(feature = "desktop")]
            app_handle: None,
        }
    }

    /// Attach a Tauri `AppHandle` so that `start()` can dispatch real OS
    /// notifications via `tauri-plugin-notification`.
    #[cfg(feature = "desktop")]
    pub fn with_app_handle(mut self, handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
    }

    /// Update the notification configuration at runtime.
    pub fn update_config(&self, new_config: NotificationConfig) {
        if let Ok(mut guard) = self.config.write() {
            *guard = new_config;
        }
    }

    /// Return `true` if the event passes both the mapping and config filter.
    pub fn should_notify(&self, event: &AppEvent) -> bool {
        let Some(spec) = event_to_notification(event) else {
            return false;
        };
        self.config
            .read()
            .map(|c| c.is_enabled_for(spec.category))
            .unwrap_or(false)
    }

    /// Start the background listener loop.
    ///
    /// Dispatches OS desktop notifications using `tauri-plugin-notification`
    /// when an `AppHandle` is available, otherwise falls back to logging.
    pub fn start(self: Arc<Self>) {
        let config = self.config.clone();
        let mut rx = self.bus.subscribe();

        #[cfg(feature = "desktop")]
        let app_handle = self.app_handle.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let Some(spec) = event_to_notification(&event) else {
                            continue;
                        };
                        let enabled = config
                            .read()
                            .map(|c| c.is_enabled_for(spec.category))
                            .unwrap_or(false);

                        if enabled {
                            #[cfg(feature = "desktop")]
                            {
                                if let Some(ref handle) = app_handle {
                                    if let Err(e) = handle
                                        .notification()
                                        .builder()
                                        .title(&spec.title)
                                        .body(&spec.body)
                                        .show()
                                    {
                                        log::warn!("[notification] OS notification failed: {e}");
                                    }
                                } else {
                                    log::info!("[notification] {} — {}", spec.title, spec.body);
                                }
                            }
                            #[cfg(not(feature = "desktop"))]
                            {
                                log::info!("[notification] {} — {}", spec.title, spec.body);
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("[notification] lagged by {n} events");
                    }
                }
            }
        });
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::TokioBroadcastBus;

    fn make_service() -> NotificationService {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new());
        NotificationService::new(bus)
    }

    // ─── event_to_notification tests ─────────────────────────────────────────

    #[test]
    fn heartbeat_tick_maps_to_notification() {
        let event = AppEvent::HeartbeatTick {
            timestamp: "2026-02-18T12:00:00Z".to_string(),
        };
        let spec = event_to_notification(&event);
        assert!(
            spec.is_some(),
            "HeartbeatTick should produce a notification"
        );
        assert_eq!(spec.unwrap().category, NotificationCategory::Heartbeat);
    }

    #[test]
    fn cron_fired_maps_to_notification() {
        let event = AppEvent::CronFired {
            job_id: "job-1".to_string(),
            schedule: "0 * * * *".to_string(),
        };
        let spec = event_to_notification(&event).unwrap();
        assert_eq!(spec.category, NotificationCategory::Cron);
        assert_eq!(spec.session_id.as_deref(), Some("job-1"));
    }

    #[test]
    fn agent_complete_maps_to_notification() {
        let event = AppEvent::AgentComplete {
            session_id: "sess-1".to_string(),
            message: "Task done!".to_string(),
        };
        let spec = event_to_notification(&event).unwrap();
        assert_eq!(spec.category, NotificationCategory::Agent);
        assert_eq!(spec.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn approval_needed_maps_to_notification() {
        let event = AppEvent::ApprovalNeeded {
            action_id: "act-1".to_string(),
            tool_name: "shell".to_string(),
            description: "Run command".to_string(),
            risk_level: "high".to_string(),
        };
        let spec = event_to_notification(&event).unwrap();
        assert_eq!(spec.category, NotificationCategory::Approval);
        assert!(
            spec.title.contains("shell"),
            "title should include tool name"
        );
    }

    #[test]
    fn system_error_maps_to_notification() {
        let event = AppEvent::SystemError {
            message: "Something failed".to_string(),
        };
        let spec = event_to_notification(&event).unwrap();
        assert_eq!(spec.category, NotificationCategory::System);
    }

    #[test]
    fn system_ready_does_not_trigger_notification() {
        let spec = event_to_notification(&AppEvent::SystemReady);
        assert!(
            spec.is_none(),
            "SystemReady should not produce a notification"
        );
    }

    #[test]
    fn memory_stored_does_not_trigger_notification() {
        let event = AppEvent::MemoryStored {
            key: "k".to_string(),
            summary: "s".to_string(),
        };
        assert!(event_to_notification(&event).is_none());
    }

    // ─── NotificationConfig tests ─────────────────────────────────────────────

    #[test]
    fn default_config_enables_all_categories() {
        let config = NotificationConfig::default();
        assert!(config.is_enabled_for(NotificationCategory::Heartbeat));
        assert!(config.is_enabled_for(NotificationCategory::Cron));
        assert!(config.is_enabled_for(NotificationCategory::Agent));
        assert!(config.is_enabled_for(NotificationCategory::Approval));
        assert!(config.is_enabled_for(NotificationCategory::System));
    }

    #[test]
    fn dnd_mode_suppresses_all() {
        let mut config = NotificationConfig::default();
        config.do_not_disturb = true;
        assert!(!config.is_enabled_for(NotificationCategory::Approval));
        assert!(!config.is_enabled_for(NotificationCategory::System));
    }

    #[test]
    fn per_category_disable_works() {
        let mut config = NotificationConfig::default();
        config.set_enabled(NotificationCategory::Heartbeat, false);
        assert!(!config.is_enabled_for(NotificationCategory::Heartbeat));
        assert!(config.is_enabled_for(NotificationCategory::Agent)); // others unaffected
    }

    #[test]
    fn re_enabling_category_works() {
        let mut config = NotificationConfig::default();
        config.set_enabled(NotificationCategory::Cron, false);
        config.set_enabled(NotificationCategory::Cron, true);
        assert!(config.is_enabled_for(NotificationCategory::Cron));
    }

    // ─── NotificationService tests ────────────────────────────────────────────

    #[test]
    fn should_notify_returns_true_for_matching_enabled_event() {
        let service = make_service();
        let event = AppEvent::AgentComplete {
            session_id: "s".to_string(),
            message: "done".to_string(),
        };
        assert!(service.should_notify(&event));
    }

    #[test]
    fn should_notify_returns_false_for_dnd() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new());
        let mut config = NotificationConfig::default();
        config.do_not_disturb = true;
        let service = NotificationService::with_config(bus, config);

        let event = AppEvent::AgentComplete {
            session_id: "s".to_string(),
            message: "done".to_string(),
        };
        assert!(!service.should_notify(&event));
    }

    #[test]
    fn should_notify_returns_false_for_non_notification_event() {
        let service = make_service();
        assert!(!service.should_notify(&AppEvent::SystemReady));
    }

    #[test]
    fn update_config_takes_effect() {
        let service = make_service();
        let mut new_config = NotificationConfig::default();
        new_config.do_not_disturb = true;
        service.update_config(new_config);

        let event = AppEvent::SystemError {
            message: "err".to_string(),
        };
        assert!(
            !service.should_notify(&event),
            "DND should suppress after update_config"
        );
    }

    #[test]
    fn notification_body_truncated_at_120_chars() {
        let long_msg = "x".repeat(200);
        let event = AppEvent::AgentComplete {
            session_id: "s".to_string(),
            message: long_msg,
        };
        let spec = event_to_notification(&event).unwrap();
        assert!(
            spec.body.len() <= 120,
            "body should be truncated to 120 chars"
        );
    }
}
