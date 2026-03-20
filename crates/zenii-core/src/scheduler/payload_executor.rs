use std::sync::Arc;

use tracing::{info, warn};

use crate::event_bus::{AppEvent, EventBus};
#[cfg(feature = "gateway")]
use crate::gateway::state::AppState;

use super::traits::{JobPayload, JobStatus, ScheduledJob};

/// Execute the payload of a scheduled job.
///
/// This is the core dispatcher that handles all payload types:
/// - Notify: publishes a SchedulerNotification event
/// - AgentTurn: resolves agent, runs chat with timeout
/// - Heartbeat: gathers sysinfo, publishes HeartbeatAlert
/// - SendViaChannel: sends via channel registry (feature-gated)
#[cfg(feature = "gateway")]
pub async fn execute(
    job: &ScheduledJob,
    event_bus: &Arc<dyn EventBus>,
    app_state: Option<&Arc<AppState>>,
) -> JobStatus {
    let result = match &job.payload {
        JobPayload::Notify { message } => execute_notify(job, message, event_bus),
        JobPayload::AgentTurn { prompt } => {
            execute_agent_turn(job, prompt, app_state, event_bus).await
        }
        JobPayload::Heartbeat => execute_heartbeat(job, event_bus, app_state).await,
        JobPayload::SendViaChannel { channel, message } => {
            execute_send_via_channel(job, channel, message, app_state).await
        }
        JobPayload::Workflow { workflow_id } => {
            execute_workflow(job, workflow_id, app_state).await
        }
    };

    // Publish completion event
    let status_str = match &result {
        JobStatus::Success => "success",
        JobStatus::Failed => "failed",
        JobStatus::Stuck => "stuck",
        JobStatus::Skipped => "skipped",
    };
    let _ = event_bus.publish(AppEvent::SchedulerJobCompleted {
        job_id: job.id.clone(),
        job_name: job.name.clone(),
        status: status_str.to_string(),
        error: None,
    });

    result
}

/// Execute a Notify payload: publish event and log.
fn execute_notify(job: &ScheduledJob, message: &str, event_bus: &Arc<dyn EventBus>) -> JobStatus {
    info!("Scheduler notify [{}]: {message}", job.name);
    let _ = event_bus.publish(AppEvent::SchedulerNotification {
        job_id: job.id.clone(),
        job_name: job.name.clone(),
        message: message.to_string(),
    });
    JobStatus::Success
}

/// Execute an AgentTurn payload: resolve agent, run chat with full context.
/// Publishes the agent's response as a SchedulerNotification so the user sees it.
#[cfg(feature = "gateway")]
async fn execute_agent_turn(
    job: &ScheduledJob,
    prompt: &str,
    app_state: Option<&Arc<AppState>>,
    event_bus: &Arc<dyn EventBus>,
) -> JobStatus {
    let Some(state) = app_state else {
        warn!(
            "Scheduler job '{}': AgentTurn skipped — no AppState wired",
            job.name
        );
        return JobStatus::Skipped;
    };

    // Build full preamble so the agent has identity + environment + reasoning protocol
    let preamble = {
        let config = state.config.load();
        let enabled = state
            .context_injection_enabled
            .load(std::sync::atomic::Ordering::Relaxed);
        let self_evo = state
            .self_evolution_enabled
            .load(std::sync::atomic::Ordering::Relaxed);
        let mut context_engine =
            crate::ai::context::ContextEngine::new(state.db.clone(), config.clone(), enabled)
                .with_skill_registry(state.skill_registry.clone())
                .with_self_evolution(self_evo);
        #[cfg(feature = "channels")]
        {
            context_engine = context_engine.with_channel_registry(state.channel_registry.clone());
        }
        #[cfg(feature = "scheduler")]
        if let Some(ref sched) = state.scheduler {
            context_engine = context_engine.with_scheduler(sched.clone());
        }

        match context_engine
            .compose(
                &crate::ai::context::ContextLevel::Full,
                &state.boot_context,
                "scheduler",
                None,
                None,
                None,
            )
            .await
        {
            Ok(p) => Some(p),
            Err(e) => {
                warn!(
                    "Scheduler job '{}': failed to compose preamble, proceeding without: {e}",
                    job.name
                );
                None
            }
        }
    };

    let agent =
        match crate::ai::resolve_agent(None, state, None, preamble.as_deref(), "scheduler").await {
            Ok(a) => a,
            Err(e) => {
                warn!(
                    "Scheduler job '{}': AgentTurn failed to resolve agent: {e}",
                    job.name
                );
                return JobStatus::Failed;
            }
        };

    match state.reasoning_engine.chat(&agent, prompt, vec![]).await {
        Ok(chat_result) => {
            let response = chat_result.response;
            info!(
                "Scheduler job '{}': AgentTurn completed, response len={}",
                job.name,
                response.len()
            );

            // Publish the agent's response so the user can see it via WS/toast
            let _ = event_bus.publish(AppEvent::SchedulerNotification {
                job_id: job.id.clone(),
                job_name: job.name.clone(),
                message: response,
            });

            JobStatus::Success
        }
        Err(e) => {
            warn!("Scheduler job '{}': AgentTurn chat failed: {e}", job.name);
            JobStatus::Failed
        }
    }
}

/// Execute a Heartbeat payload: gather sysinfo, publish HeartbeatAlert.
#[cfg(feature = "gateway")]
async fn execute_heartbeat(
    job: &ScheduledJob,
    event_bus: &Arc<dyn EventBus>,
    _app_state: Option<&Arc<AppState>>,
) -> JobStatus {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_pct = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64) * 100.0
    } else {
        0.0
    };

    let message = format!(
        "Heartbeat [{}]: memory {:.1}% ({}/{}MB)",
        job.name,
        mem_pct,
        used_mem / 1_048_576,
        total_mem / 1_048_576,
    );

    info!("{message}");
    let _ = event_bus.publish(AppEvent::HeartbeatAlert { message });
    JobStatus::Success
}

/// Execute a SendViaChannel payload.
#[cfg(feature = "gateway")]
async fn execute_send_via_channel(
    job: &ScheduledJob,
    channel: &str,
    message: &str,
    app_state: Option<&Arc<AppState>>,
) -> JobStatus {
    #[cfg(feature = "channels")]
    {
        let Some(state) = app_state else {
            warn!(
                "Scheduler job '{}': SendViaChannel skipped — no AppState wired",
                job.name
            );
            return JobStatus::Skipped;
        };

        let ch_msg = crate::channels::message::ChannelMessage::new(channel, message);
        match state.channel_registry.send(channel, ch_msg).await {
            Ok(()) => {
                info!(
                    "Scheduler job '{}': sent message via channel '{channel}'",
                    job.name
                );
                JobStatus::Success
            }
            Err(e) => {
                warn!("Scheduler job '{}': SendViaChannel failed: {e}", job.name);
                JobStatus::Failed
            }
        }
    }
    #[cfg(not(feature = "channels"))]
    {
        let _ = (app_state, channel, message);
        warn!(
            "Scheduler job '{}': SendViaChannel skipped — channels feature not enabled",
            job.name
        );
        JobStatus::Skipped
    }
}

/// Execute a Workflow payload.
#[cfg(feature = "gateway")]
async fn execute_workflow(
    job: &ScheduledJob,
    workflow_id: &str,
    app_state: Option<&Arc<AppState>>,
) -> JobStatus {
    #[cfg(feature = "workflows")]
    {
        let Some(state) = app_state else {
            warn!(
                "Scheduler job '{}': Workflow skipped — no AppState wired",
                job.name
            );
            return JobStatus::Skipped;
        };

        let Some(ref registry) = state.workflow_registry else {
            warn!(
                "Scheduler job '{}': Workflow skipped — workflow feature not initialized",
                job.name
            );
            return JobStatus::Skipped;
        };

        let Some(workflow) = registry.get(workflow_id) else {
            warn!(
                "Scheduler job '{}': Workflow '{}' not found",
                job.name, workflow_id
            );
            return JobStatus::Failed;
        };

        let Some(ref executor) = state.workflow_executor else {
            warn!(
                "Scheduler job '{}': Workflow executor not initialized",
                job.name
            );
            return JobStatus::Skipped;
        };

        match executor.execute(&workflow, &state.tools, state.event_bus.as_ref()).await {
            Ok(run) => {
                info!(
                    "Scheduler job '{}': Workflow '{}' completed (run {})",
                    job.name, workflow_id, run.id
                );
                if run.status == crate::workflows::WorkflowRunStatus::Completed {
                    JobStatus::Success
                } else {
                    JobStatus::Failed
                }
            }
            Err(e) => {
                warn!(
                    "Scheduler job '{}': Workflow '{}' failed: {e}",
                    job.name, workflow_id
                );
                JobStatus::Failed
            }
        }
    }
    #[cfg(not(feature = "workflows"))]
    {
        let _ = (app_state, workflow_id);
        warn!(
            "Scheduler job '{}': Workflow skipped — workflows feature not enabled",
            job.name
        );
        JobStatus::Skipped
    }
}

#[cfg(test)]
#[cfg(feature = "gateway")]
mod tests {
    use super::*;
    use crate::event_bus::TokioBroadcastBus;
    use crate::scheduler::traits::*;

    fn make_job(name: &str, payload: JobPayload) -> ScheduledJob {
        ScheduledJob {
            id: "test-id".into(),
            name: name.into(),
            schedule: Schedule::Interval { secs: 60 },
            session_target: SessionTarget::Main,
            payload,
            enabled: true,
            error_count: 0,
            next_run: None,
            active_hours: None,
            delete_after_run: false,
        }
    }

    // 8.6.1.7 — Notify payload publishes SchedulerNotification event
    #[tokio::test]
    async fn notify_publishes_event() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let mut rx = bus.subscribe();

        let job = make_job(
            "test_notify",
            JobPayload::Notify {
                message: "hello world".into(),
            },
        );

        let status = execute(&job, &bus, None).await;
        assert_eq!(status, JobStatus::Success);

        // Should receive SchedulerNotification then SchedulerJobCompleted
        let event = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            AppEvent::SchedulerNotification { message, .. } if message == "hello world"
        ));
    }

    // 8.6.1.8 — Notify payload returns Success
    #[tokio::test]
    async fn notify_returns_success() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let job = make_job(
            "test_notify",
            JobPayload::Notify {
                message: "test".into(),
            },
        );
        let status = execute(&job, &bus, None).await;
        assert_eq!(status, JobStatus::Success);
    }

    // 8.6.1.9 — Heartbeat payload returns Success
    #[tokio::test]
    async fn heartbeat_returns_success() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let job = make_job("test_heartbeat", JobPayload::Heartbeat);
        let status = execute(&job, &bus, None).await;
        assert_eq!(status, JobStatus::Success);
    }

    // 8.6.1.10 — Heartbeat publishes HeartbeatAlert event
    #[tokio::test]
    async fn heartbeat_publishes_alert() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let mut rx = bus.subscribe();
        let job = make_job("test_heartbeat", JobPayload::Heartbeat);

        execute(&job, &bus, None).await;

        let event = rx.recv().await.unwrap();
        assert!(
            matches!(event, AppEvent::HeartbeatAlert { message } if message.contains("Heartbeat"))
        );
    }

    // 8.6.1.11 — AgentTurn without AppState returns Skipped
    #[tokio::test]
    async fn agent_turn_no_state_skipped() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let job = make_job(
            "test_agent",
            JobPayload::AgentTurn {
                prompt: "hello".into(),
            },
        );
        let status = execute(&job, &bus, None).await;
        assert_eq!(status, JobStatus::Skipped);
    }

    // 8.6.1.12 — AgentTurn with AppState but no API key returns Failed
    #[tokio::test]
    async fn agent_turn_no_api_key_failed() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;

        let job = make_job(
            "test_agent",
            JobPayload::AgentTurn {
                prompt: "hello".into(),
            },
        );
        let status = execute(&job, &bus, Some(&state)).await;
        // No API key configured in test state, so agent resolution should fail
        assert!(status == JobStatus::Failed || status == JobStatus::Skipped);
    }

    // 8.6.1.13 — SendViaChannel without channels feature returns Skipped
    #[tokio::test]
    async fn send_via_channel_skipped_without_feature() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let job = make_job(
            "test_send",
            JobPayload::SendViaChannel {
                channel: "telegram".into(),
                message: "hello".into(),
            },
        );
        let status = execute(&job, &bus, None).await;
        // Without AppState or without channels feature, should be Skipped
        assert!(status == JobStatus::Skipped || status == JobStatus::Failed);
    }

    // 8.6.1.18 — Notify payload execution publishes SchedulerNotification event end-to-end
    #[tokio::test]
    async fn scheduler_notify_end_to_end() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let mut rx = bus.subscribe();

        let job = make_job(
            "e2e_notify",
            JobPayload::Notify {
                message: "end-to-end test".into(),
            },
        );

        let status = execute(&job, &bus, None).await;
        assert_eq!(status, JobStatus::Success);

        // First event should be SchedulerNotification
        let event = rx.recv().await.unwrap();
        assert!(
            matches!(event, AppEvent::SchedulerNotification { job_id, job_name, message }
                if job_id == "test-id" && job_name == "e2e_notify" && message == "end-to-end test")
        );
        // Second event should be SchedulerJobCompleted
        let event2 = rx.recv().await.unwrap();
        assert!(
            matches!(event2, AppEvent::SchedulerJobCompleted { status, .. } if status == "success")
        );
    }

    // 8.6.1.19 — Heartbeat payload publishes HeartbeatAlert end-to-end
    #[tokio::test]
    async fn scheduler_heartbeat_end_to_end() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let mut rx = bus.subscribe();

        let job = make_job("e2e_heartbeat", JobPayload::Heartbeat);
        let status = execute(&job, &bus, None).await;
        assert_eq!(status, JobStatus::Success);

        // First event should be HeartbeatAlert
        let event = rx.recv().await.unwrap();
        assert!(
            matches!(event, AppEvent::HeartbeatAlert { message } if message.contains("Heartbeat"))
        );
        // Second event should be SchedulerJobCompleted
        let event2 = rx.recv().await.unwrap();
        assert!(
            matches!(event2, AppEvent::SchedulerJobCompleted { status, .. } if status == "success")
        );
    }

    // 8.6.1.20 — AgentTurn without API key returns Failed gracefully (no panic)
    #[tokio::test]
    async fn scheduler_agent_failure_graceful() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;

        let job = make_job(
            "agent_graceful",
            JobPayload::AgentTurn {
                prompt: "should fail gracefully".into(),
            },
        );

        // Should not panic; returns Failed or Skipped because no API key is configured
        let status = execute(&job, &bus, Some(&state)).await;
        assert!(
            status == JobStatus::Failed || status == JobStatus::Skipped,
            "AgentTurn without API key should fail gracefully, got: {status:?}"
        );
    }

    // 8.6.1.21 — SendViaChannel for nonexistent channel returns Failed
    #[cfg(feature = "channels")]
    #[tokio::test]
    async fn scheduler_send_channel_not_found() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;

        let job = make_job(
            "send_nonexistent",
            JobPayload::SendViaChannel {
                channel: "nonexistent_channel".into(),
                message: "hello".into(),
            },
        );

        let status = execute(&job, &bus, Some(&state)).await;
        assert_eq!(
            status,
            JobStatus::Failed,
            "SendViaChannel for nonexistent channel should return Failed"
        );
    }

    // 8.6.1.14 — execute always publishes SchedulerJobCompleted
    #[tokio::test]
    async fn execute_publishes_completion() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let mut rx = bus.subscribe();

        let job = make_job(
            "test_complete",
            JobPayload::Notify {
                message: "done".into(),
            },
        );

        execute(&job, &bus, None).await;

        // Drain events until we find SchedulerJobCompleted
        let mut found = false;
        for _ in 0..5 {
            if let Ok(event) = rx.try_recv() {
                if matches!(event, AppEvent::SchedulerJobCompleted { .. }) {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "Should publish SchedulerJobCompleted event");
    }
}
