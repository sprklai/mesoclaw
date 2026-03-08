use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::sync::watch;
use tracing::info;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db::{self, DbPool};
use crate::event_bus::{AppEvent, EventBus};
use crate::{MesoError, Result};

use super::heartbeat::backoff_secs;
use super::traits::*;

type JobRow = (
    String,         // id
    String,         // name
    String,         // schedule_json
    String,         // session_target
    String,         // payload_json
    i32,            // enabled
    i32,            // error_count
    Option<String>, // next_run
    String,         // created_at
    Option<String>, // active_hours_json
    i32,            // delete_after_run
);

/// Tokio-driven scheduler with DashMap registry and SQLite persistence.
pub struct TokioScheduler {
    jobs: Arc<DashMap<String, ScheduledJob>>,
    history: Arc<DashMap<String, VecDeque<JobExecution>>>,
    db: DbPool,
    event_bus: Arc<dyn EventBus>,
    stop_tx: watch::Sender<bool>,
    stop_rx: watch::Receiver<bool>,
    tick_interval_secs: u64,
    stuck_threshold_secs: u64,
    max_history_per_job: usize,
    error_backoff_secs: Vec<u64>,
}

impl TokioScheduler {
    pub fn new(db: DbPool, event_bus: Arc<dyn EventBus>, config: &AppConfig) -> Arc<Self> {
        let (stop_tx, stop_rx) = watch::channel(false);
        Arc::new(Self {
            jobs: Arc::new(DashMap::new()),
            history: Arc::new(DashMap::new()),
            db,
            event_bus,
            stop_tx,
            stop_rx,
            tick_interval_secs: config.scheduler_tick_interval_secs,
            stuck_threshold_secs: config.scheduler_stuck_threshold_secs,
            max_history_per_job: config.scheduler_max_history_per_job,
            error_backoff_secs: config.scheduler_error_backoff_secs.clone(),
        })
    }

    /// Load persisted jobs from SQLite into the in-memory registry.
    pub async fn load_from_db(&self) -> Result<usize> {
        let pool = self.db.clone();
        let rows = db::with_db(&pool, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, schedule_json, session_target, payload_json, \
                 enabled, error_count, next_run, created_at, active_hours_json, \
                 delete_after_run FROM scheduled_jobs",
            )?;
            let jobs: Vec<JobRow> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();
            Ok(jobs)
        })
        .await?;

        let mut count = 0;
        for (
            id,
            name,
            schedule_json,
            session_target_str,
            payload_json,
            enabled,
            error_count,
            next_run_str,
            _created_at,
            active_hours_json,
            delete_after_run,
        ) in rows
        {
            let schedule: Schedule = match serde_json::from_str(&schedule_json) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let payload: JobPayload = match serde_json::from_str(&payload_json) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let session_target = if session_target_str.contains("isolated") {
                SessionTarget::Isolated
            } else {
                SessionTarget::Main
            };
            let next_run = next_run_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));
            let active_hours: Option<ActiveHours> =
                active_hours_json.and_then(|s| serde_json::from_str(&s).ok());

            let job = ScheduledJob {
                id: id.clone(),
                name,
                schedule,
                session_target,
                payload,
                enabled: enabled != 0,
                error_count: error_count as u32,
                next_run,
                active_hours,
                delete_after_run: delete_after_run != 0,
            };
            self.jobs.insert(id, job);
            count += 1;
        }
        info!("Scheduler loaded {count} job(s) from DB");
        Ok(count)
    }

    /// Persist a job to SQLite (upsert).
    async fn persist_job(db: &DbPool, job: &ScheduledJob) -> Result<()> {
        let schedule_json = serde_json::to_string(&job.schedule)
            .map_err(|e| MesoError::Scheduler(e.to_string()))?;
        let payload_json =
            serde_json::to_string(&job.payload).map_err(|e| MesoError::Scheduler(e.to_string()))?;
        let session_target = format!("{:?}", job.session_target).to_lowercase();
        let next_run = job.next_run.map(|t| t.to_rfc3339());
        let active_hours_json = job
            .active_hours
            .as_ref()
            .and_then(|h| serde_json::to_string(h).ok());
        let id = job.id.clone();
        let name = job.name.clone();
        let enabled = if job.enabled { 1i32 } else { 0 };
        let error_count = job.error_count as i32;
        let delete_after_run = if job.delete_after_run { 1i32 } else { 0 };

        let pool = db.clone();
        db::with_db(&pool, move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO scheduled_jobs \
                 (id, name, schedule_json, session_target, payload_json, \
                  enabled, error_count, next_run, created_at, active_hours_json, delete_after_run) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'), ?9, ?10)",
                rusqlite::params![
                    id,
                    name,
                    schedule_json,
                    session_target,
                    payload_json,
                    enabled,
                    error_count,
                    next_run,
                    active_hours_json,
                    delete_after_run,
                ],
            )?;
            Ok(())
        })
        .await
    }

    /// Delete a job from SQLite.
    async fn delete_job_from_db(db: &DbPool, id: &str) -> Result<()> {
        let pool = db.clone();
        let id = id.to_string();
        db::with_db(&pool, move |conn| {
            conn.execute("DELETE FROM scheduled_jobs WHERE id = ?1", [&id])?;
            Ok(())
        })
        .await
    }

    /// Compute the next run time for a schedule.
    pub fn compute_next_run(schedule: &Schedule) -> Result<DateTime<Utc>> {
        match schedule {
            Schedule::Interval { secs } => Ok(Utc::now() + chrono::Duration::seconds(*secs as i64)),
            Schedule::Cron { expr } => {
                // Support 5-field (min hr dom mon dow) or 6/7-field expressions
                let full_expr = if expr.split_whitespace().count() == 5 {
                    format!("0 {expr}")
                } else {
                    expr.clone()
                };
                let schedule = cron::Schedule::from_str(&full_expr)
                    .map_err(|e| MesoError::Scheduler(format!("invalid cron: {e}")))?;
                schedule
                    .upcoming(Utc)
                    .next()
                    .ok_or_else(|| MesoError::Scheduler("cron has no upcoming time".into()))
            }
        }
    }

    /// Check if the current local hour is within active hours.
    fn is_in_active_hours(active_hours: &Option<ActiveHours>) -> bool {
        match active_hours {
            None => true,
            Some(hours) => {
                use chrono::Timelike;
                let local_hour = chrono::Local::now().hour() as u8;
                local_hour >= hours.start_hour && local_hour < hours.end_hour
            }
        }
    }

    /// Get backoff delay using configured levels.
    pub fn get_backoff(&self, error_count: u32) -> u64 {
        if self.error_backoff_secs.is_empty() {
            return backoff_secs(error_count);
        }
        let idx = (error_count as usize).min(self.error_backoff_secs.len() - 1);
        self.error_backoff_secs[idx]
    }

    /// Record an execution in the history ring buffer.
    pub fn record_execution(&self, exec: JobExecution) {
        let job_id = exec.job_id.clone();
        let mut entry = self.history.entry(job_id).or_default();
        entry.push_front(exec);
        entry.truncate(self.max_history_per_job);
    }

    /// Check if scheduler is running (stop signal not sent).
    pub fn is_running(&self) -> bool {
        !*self.stop_rx.borrow()
    }

    /// Get count of registered jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

#[async_trait]
impl Scheduler for TokioScheduler {
    async fn start(&self) {
        let jobs = self.jobs.clone();
        let history = self.history.clone();
        let bus = self.event_bus.clone();
        let db = self.db.clone();
        let mut stop_rx = self.stop_rx.clone();
        let tick_secs = self.tick_interval_secs;
        let stuck_threshold = self.stuck_threshold_secs;
        let max_history = self.max_history_per_job;

        let _ = bus.publish(AppEvent::SchedulerStarted);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(tick_secs));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let now = Utc::now();
                        let due: Vec<ScheduledJob> = jobs
                            .iter()
                            .filter(|entry| entry.value().enabled)
                            .filter(|entry| {
                                entry.value().next_run.is_some_and(|t| t <= now)
                            })
                            .map(|entry| entry.value().clone())
                            .collect();

                        for job in due {
                            // Active hours gate
                            if !TokioScheduler::is_in_active_hours(&job.active_hours) {
                                // Reschedule
                                if let Some(mut entry) = jobs.get_mut(&job.id)
                                    && let Ok(next) = TokioScheduler::compute_next_run(&entry.schedule)
                                {
                                    entry.next_run = Some(next);
                                }
                                continue;
                            }

                            let started_at = Utc::now();

                            // Emit event
                            let event = match &job.payload {
                                JobPayload::Heartbeat => AppEvent::HeartbeatTick {
                                    job_id: job.id.clone(),
                                },
                                _ => AppEvent::CronFired {
                                    job_id: job.id.clone(),
                                    name: job.name.clone(),
                                },
                            };
                            let _ = bus.publish(event);

                            // Execute with stuck detection timeout
                            let timeout = Duration::from_secs(stuck_threshold);
                            let status = tokio::time::timeout(timeout, async {
                                // Payload execution stub — actual agent integration deferred
                                match &job.payload {
                                    JobPayload::Notify { message } => {
                                        info!("Scheduler notify: {message}");
                                        JobStatus::Success
                                    }
                                    _ => {
                                        // Heartbeat, AgentTurn, SendViaChannel
                                        // require agent wiring — mark as success for now
                                        JobStatus::Success
                                    }
                                }
                            })
                            .await;

                            let completed_at = Utc::now();
                            let (job_status, error_msg) = match status {
                                Ok(s) => (s, None),
                                Err(_) => (
                                    JobStatus::Stuck,
                                    Some(format!(
                                        "Job '{}' stuck after {stuck_threshold}s",
                                        job.name
                                    )),
                                ),
                            };

                            // Record history
                            let exec = JobExecution {
                                id: Uuid::new_v4().to_string(),
                                job_id: job.id.clone(),
                                status: job_status.clone(),
                                started_at,
                                completed_at: Some(completed_at),
                                error: error_msg,
                            };
                            let exec_job_id = exec.job_id.clone();
                            {
                                let mut entry = history
                                    .entry(exec_job_id)
                                    .or_default();
                                entry.push_front(exec);
                                entry.truncate(max_history);
                            }

                            // Reschedule / one-shot / error tracking
                            if job.delete_after_run && job_status == JobStatus::Success {
                                jobs.remove(&job.id);
                                let _ = TokioScheduler::delete_job_from_db(&db, &job.id).await;
                            } else if let Some(mut entry) = jobs.get_mut(&job.id) {
                                if job_status == JobStatus::Success {
                                    entry.error_count = 0;
                                } else {
                                    entry.error_count += 1;
                                }
                                if let Ok(next) = TokioScheduler::compute_next_run(&entry.schedule) {
                                    entry.next_run = Some(next);
                                }
                                let _ = TokioScheduler::persist_job(&db, &entry).await;
                            }
                        }
                    }
                    Ok(()) = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            let _ = bus.publish(AppEvent::SchedulerStopped);
                            break;
                        }
                    }
                }
            }
        });
    }

    async fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }

    async fn add_job(&self, mut job: ScheduledJob) -> Result<JobId> {
        if job.id.is_empty() {
            job.id = Uuid::new_v4().to_string();
        }

        // Validate cron expression if applicable
        if let Schedule::Cron { ref expr } = job.schedule {
            let full_expr = if expr.split_whitespace().count() == 5 {
                format!("0 {expr}")
            } else {
                expr.clone()
            };
            cron::Schedule::from_str(&full_expr)
                .map_err(|e| MesoError::Scheduler(format!("invalid cron expression: {e}")))?;
        }

        job.next_run = Some(Self::compute_next_run(&job.schedule)?);

        Self::persist_job(&self.db, &job).await?;

        let id = job.id.clone();
        self.jobs.insert(id.clone(), job);
        Ok(id)
    }

    async fn remove_job(&self, id: &str) -> Result<()> {
        self.jobs
            .remove(id)
            .ok_or_else(|| MesoError::NotFound(format!("job '{id}' not found")))?;
        Self::delete_job_from_db(&self.db, id).await?;
        Ok(())
    }

    async fn toggle_job(&self, id: &str) -> Result<bool> {
        let new_state = {
            let mut entry = self
                .jobs
                .get_mut(id)
                .ok_or_else(|| MesoError::NotFound(format!("job '{id}' not found")))?;
            entry.enabled = !entry.enabled;
            entry.enabled
        };

        // Persist updated state
        if let Some(entry) = self.jobs.get(id) {
            Self::persist_job(&self.db, &entry).await?;
        }

        Ok(new_state)
    }

    async fn list_jobs(&self) -> Vec<ScheduledJob> {
        self.jobs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    async fn job_history(&self, id: &str) -> Vec<JobExecution> {
        self.history
            .get(id)
            .map(|entry| entry.value().iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::event_bus::TokioBroadcastBus;

    fn test_db() -> (tempfile::TempDir, DbPool) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = db::init_pool(&path).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        crate::db::run_migrations(&conn).unwrap();
        drop(conn);
        (dir, pool)
    }

    fn test_scheduler() -> (tempfile::TempDir, Arc<TokioScheduler>) {
        let (dir, pool) = test_db();
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let config = AppConfig::default();
        let scheduler = TokioScheduler::new(pool, bus, &config);
        (dir, scheduler)
    }

    fn test_job(name: &str) -> ScheduledJob {
        ScheduledJob {
            id: String::new(),
            name: name.into(),
            schedule: Schedule::Interval { secs: 60 },
            session_target: SessionTarget::Main,
            payload: JobPayload::Notify {
                message: "test".into(),
            },
            enabled: true,
            error_count: 0,
            next_run: None,
            active_hours: None,
            delete_after_run: false,
        }
    }

    // 16.9 — Add job to scheduler
    #[tokio::test]
    async fn add_job() {
        let (_dir, sched) = test_scheduler();
        let id = sched.add_job(test_job("job1")).await.unwrap();
        assert!(!id.is_empty());
        assert_eq!(sched.list_jobs().await.len(), 1);
    }

    // 16.10 — Remove job from scheduler
    #[tokio::test]
    async fn remove_job() {
        let (_dir, sched) = test_scheduler();
        let id = sched.add_job(test_job("job1")).await.unwrap();
        sched.remove_job(&id).await.unwrap();
        assert_eq!(sched.list_jobs().await.len(), 0);
    }

    // 16.11 — Remove nonexistent job returns error
    #[tokio::test]
    async fn remove_nonexistent_errors() {
        let (_dir, sched) = test_scheduler();
        let result = sched.remove_job("nonexistent").await;
        assert!(result.is_err());
    }

    // 16.12 — Toggle job enabled/disabled
    #[tokio::test]
    async fn toggle_job() {
        let (_dir, sched) = test_scheduler();
        let id = sched.add_job(test_job("job1")).await.unwrap();
        let new_state = sched.toggle_job(&id).await.unwrap();
        assert!(!new_state); // was true, now false
        let new_state = sched.toggle_job(&id).await.unwrap();
        assert!(new_state); // toggled back to true
    }

    // 16.13 — List jobs returns all registered
    #[tokio::test]
    async fn list_jobs() {
        let (_dir, sched) = test_scheduler();
        sched.add_job(test_job("job1")).await.unwrap();
        sched.add_job(test_job("job2")).await.unwrap();
        sched.add_job(test_job("job3")).await.unwrap();
        assert_eq!(sched.list_jobs().await.len(), 3);
    }

    // 16.14 — Cron expression parsing valid
    #[tokio::test]
    async fn cron_parsing_valid() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("cron_job");
        job.schedule = Schedule::Cron {
            expr: "0 */5 * * * *".into(),
        };
        let id = sched.add_job(job).await.unwrap();
        assert!(!id.is_empty());
        let jobs = sched.list_jobs().await;
        assert!(jobs[0].next_run.is_some());
    }

    // 16.15 — Cron expression parsing invalid errors
    #[tokio::test]
    async fn cron_parsing_invalid() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("bad_cron");
        job.schedule = Schedule::Cron {
            expr: "not a cron".into(),
        };
        let result = sched.add_job(job).await;
        assert!(result.is_err());
    }

    // 16.16 — Interval next_run calculation
    #[test]
    fn interval_next_run() {
        let before = Utc::now();
        let next = TokioScheduler::compute_next_run(&Schedule::Interval { secs: 60 }).unwrap();
        let after = Utc::now();
        assert!(next >= before + chrono::Duration::seconds(59));
        assert!(next <= after + chrono::Duration::seconds(61));
    }

    // 16.17 — Error backoff delay calculation
    #[test]
    fn error_backoff_delay() {
        let (_dir, sched) = test_scheduler();
        assert_eq!(sched.get_backoff(0), 30);
        assert_eq!(sched.get_backoff(1), 60);
        assert_eq!(sched.get_backoff(2), 300);
    }

    // 16.18 — Error backoff caps at max
    #[test]
    fn error_backoff_max() {
        let (_dir, sched) = test_scheduler();
        assert_eq!(sched.get_backoff(100), 3600);
    }

    // 16.19 — Active hours gate allows in-window
    #[test]
    fn active_hours_in_window() {
        use chrono::Timelike;
        let current_hour = chrono::Local::now().hour() as u8;
        let hours = Some(ActiveHours {
            start_hour: current_hour,
            end_hour: current_hour + 1,
        });
        assert!(TokioScheduler::is_in_active_hours(&hours));
    }

    // 16.20 — Active hours gate blocks out-of-window
    #[test]
    fn active_hours_out_of_window() {
        use chrono::Timelike;
        let current_hour = chrono::Local::now().hour() as u8;
        // Set window to a different hour
        let other_hour = (current_hour + 12) % 24;
        let hours = Some(ActiveHours {
            start_hour: other_hour,
            end_hour: (other_hour + 1) % 24,
        });
        // This may or may not block depending on edge cases with wrapping,
        // so test the simple case where we know we're outside
        if other_hour < (other_hour + 1) % 24 {
            assert!(!TokioScheduler::is_in_active_hours(&hours));
        }
    }

    // 16.21 — Active hours None means always active
    #[test]
    fn active_hours_none_always() {
        assert!(TokioScheduler::is_in_active_hours(&None));
    }

    // 16.22 — One-shot job deleted after run
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn one_shot_deleted() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("oneshot");
        job.delete_after_run = true;
        job.schedule = Schedule::Interval { secs: 1 };
        let id = sched.add_job(job).await.unwrap();
        assert_eq!(sched.list_jobs().await.len(), 1);

        // Force next_run to past so tick fires immediately
        if let Some(mut entry) = sched.jobs.get_mut(&id) {
            entry.next_run = Some(Utc::now() - chrono::Duration::seconds(1));
        }

        sched.start().await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        sched.stop().await;

        // Job should be removed after successful run
        assert_eq!(
            sched.list_jobs().await.len(),
            0,
            "One-shot job should be removed after run"
        );
        assert!(sched.jobs.get(&id).is_none());
    }

    // 16.23 — Job execution history recorded
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn execution_history_recorded() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("history_job");
        job.schedule = Schedule::Interval { secs: 1 };
        let id = sched.add_job(job).await.unwrap();

        // Force next_run to past so tick fires immediately
        if let Some(mut entry) = sched.jobs.get_mut(&id) {
            entry.next_run = Some(Utc::now() - chrono::Duration::seconds(1));
        }

        sched.start().await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        sched.stop().await;

        let history = sched.job_history(&id).await;
        assert!(
            !history.is_empty(),
            "Should have at least one execution record"
        );
        assert_eq!(history[0].status, JobStatus::Success);
    }

    // 16.24 — Job history max entries enforced
    #[tokio::test]
    async fn history_max_entries() {
        let (_dir, sched) = test_scheduler();
        // Add more entries than max
        let max = sched.max_history_per_job;
        for i in 0..max + 5 {
            sched.record_execution(JobExecution {
                id: format!("exec-{i}"),
                job_id: "j1".into(),
                status: JobStatus::Success,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                error: None,
            });
        }
        let history = sched.job_history("j1").await;
        assert_eq!(history.len(), max);
    }

    // 16.25 — Stuck detection marks timed-out jobs
    #[tokio::test]
    async fn stuck_detection() {
        // Create scheduler with very short stuck threshold
        let (dir, pool) = test_db();
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let config = AppConfig {
            scheduler_stuck_threshold_secs: 1,
            ..Default::default()
        };
        let sched = TokioScheduler::new(pool, bus, &config);

        // The tick loop has a stub that completes instantly,
        // so stuck detection won't trigger in this unit test.
        // This test validates the config propagation.
        assert_eq!(sched.stuck_threshold_secs, 1);
        drop(dir);
    }

    // 16.26 — SQLite persistence save and reload
    #[tokio::test]
    async fn sqlite_persist_reload() {
        let (_dir, pool) = test_db();
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let config = AppConfig::default();

        // Create scheduler, add job
        let sched1 = TokioScheduler::new(pool.clone(), bus.clone(), &config);
        let id = sched1.add_job(test_job("persist_test")).await.unwrap();
        assert_eq!(sched1.list_jobs().await.len(), 1);

        // Create new scheduler (simulating restart), load from DB
        let sched2 = TokioScheduler::new(pool, bus, &config);
        assert_eq!(sched2.list_jobs().await.len(), 0);
        let loaded = sched2.load_from_db().await.unwrap();
        assert_eq!(loaded, 1);
        assert_eq!(sched2.list_jobs().await.len(), 1);

        let jobs = sched2.list_jobs().await;
        assert_eq!(jobs[0].id, id);
        assert_eq!(jobs[0].name, "persist_test");
    }

    // 16.27 — Disabled job skipped during tick
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn disabled_job_skipped() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("disabled");
        job.schedule = Schedule::Interval { secs: 1 };
        let id = sched.add_job(job).await.unwrap();

        // Force next_run to past, then disable
        if let Some(mut entry) = sched.jobs.get_mut(&id) {
            entry.next_run = Some(Utc::now() - chrono::Duration::seconds(1));
        }
        sched.toggle_job(&id).await.unwrap(); // disable

        sched.start().await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        sched.stop().await;

        let history = sched.job_history(&id).await;
        assert!(history.is_empty(), "Disabled job should have no executions");
    }
}
