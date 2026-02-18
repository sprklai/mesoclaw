//! Tokio-backed scheduler implementation.
//!
//! `TokioScheduler` drives a background task that wakes every second, scans
//! for due jobs, executes them, records history, and reschedules.
//!
//! ## Stuck detection
//! A job is considered stuck if it doesn't complete within 120 seconds.
//! A `SystemError` event is emitted and the job's error count is incremented.
//!
//! ## Persistence
//! Jobs are held in memory.  Persistence to SQLite is a planned follow-up
//! (see TODO in [`TokioScheduler::add_job`]).

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    database::{schema::scheduled_jobs, DbPool},
    event_bus::{AppEvent, EventBus},
};

use super::traits::{JobExecution, JobId, JobPayload, JobStatus, Schedule, ScheduledJob, Scheduler};

// ─── Diesel row type ─────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = scheduled_jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct ScheduledJobRow {
    id: String,
    name: String,
    schedule_json: String,
    session_target: String,
    payload_json: String,
    enabled: i32,
    error_count: i32,
    next_run: Option<String>,
    created_at: String,
}

impl ScheduledJobRow {
    fn from_job(job: &ScheduledJob) -> Option<Self> {
        Some(Self {
            id: job.id.clone(),
            name: job.name.clone(),
            schedule_json: serde_json::to_string(&job.schedule).ok()?,
            session_target: format!("{:?}", job.session_target),
            payload_json: serde_json::to_string(&job.payload).ok()?,
            enabled: if job.enabled { 1 } else { 0 },
            error_count: job.error_count as i32,
            next_run: job.next_run.map(|t| t.to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
        })
    }

    fn into_job(self) -> Option<ScheduledJob> {
        use super::traits::SessionTarget;
        let schedule: Schedule = serde_json::from_str(&self.schedule_json).ok()?;
        let payload: JobPayload = serde_json::from_str(&self.payload_json).ok()?;
        let session_target = if self.session_target.contains("Isolated") {
            SessionTarget::Isolated
        } else {
            SessionTarget::Main
        };
        let next_run = self
            .next_run
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        Some(ScheduledJob {
            id: self.id,
            name: self.name,
            schedule,
            session_target,
            payload,
            enabled: self.enabled != 0,
            error_count: self.error_count as u32,
            next_run,
        })
    }
}

// ─── Constants ────────────────────────────────────────────────────────────────

/// Maximum execution time before a job is flagged as stuck.
const STUCK_THRESHOLD_SECS: u64 = 120;

/// Scheduler tick interval — how often we check for due jobs.
const TICK_INTERVAL_SECS: u64 = 1;

/// Maximum history entries kept per job.
const MAX_HISTORY_PER_JOB: usize = 100;

// ─── Internal state ───────────────────────────────────────────────────────────

type JobMap = HashMap<JobId, ScheduledJob>;
type HistoryMap = HashMap<JobId, Vec<JobExecution>>;

// ─── TokioScheduler ───────────────────────────────────────────────────────────

/// In-memory, Tokio-driven scheduler with optional SQLite persistence.
pub struct TokioScheduler {
    jobs: Arc<RwLock<JobMap>>,
    history: Arc<RwLock<HistoryMap>>,
    bus: Arc<dyn EventBus>,
    /// Optional database pool for job persistence.
    pool: Option<DbPool>,
    /// Send `true` to stop the background task.
    stop_tx: watch::Sender<bool>,
    stop_rx: watch::Receiver<bool>,
}

impl TokioScheduler {
    /// Create a scheduler without persistence (used in tests and early boot).
    pub fn new(bus: Arc<dyn EventBus>) -> Arc<Self> {
        Self::new_with_persistence(bus, None)
    }

    /// Create a scheduler with SQLite persistence.
    pub fn new_with_persistence(bus: Arc<dyn EventBus>, pool: Option<DbPool>) -> Arc<Self> {
        let (stop_tx, stop_rx) = watch::channel(false);
        let scheduler = Arc::new(Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
            bus,
            pool,
            stop_tx,
            stop_rx,
        });
        // Load persisted jobs from DB on construction.
        if let Some(ref pool) = scheduler.pool {
            scheduler.load_jobs_from_db(pool);
        }
        scheduler
    }

    /// Load all persisted jobs from the `scheduled_jobs` table into memory.
    fn load_jobs_from_db(&self, pool: &DbPool) {
        let Ok(mut conn) = pool.get() else { return };
        let rows: Vec<ScheduledJobRow> = match scheduled_jobs::table
            .select(ScheduledJobRow::as_select())
            .load(&mut conn)
        {
            Ok(r) => r,
            Err(e) => {
                log::warn!("scheduler: failed to load persisted jobs: {e}");
                return;
            }
        };
        let Ok(mut map) = self.jobs.write() else { return };
        for row in rows {
            if let Some(job) = row.into_job() {
                map.insert(job.id.clone(), job);
            }
        }
        log::info!("scheduler: loaded {} persisted job(s) from DB", map.len());
    }

    /// Persist a job to the database (upsert).
    fn persist_job(&self, job: &ScheduledJob) {
        let Some(ref pool) = self.pool else { return };
        let Ok(mut conn) = pool.get() else { return };
        let Some(row) = ScheduledJobRow::from_job(job) else { return };
        if let Err(e) = diesel::replace_into(scheduled_jobs::table)
            .values(&row)
            .execute(&mut conn)
        {
            log::warn!("scheduler: failed to persist job '{}': {e}", job.id);
        }
    }

    /// Delete a job from the database.
    fn delete_job_from_db(&self, id: &str) {
        let Some(ref pool) = self.pool else { return };
        let Ok(mut conn) = pool.get() else { return };
        if let Err(e) = diesel::delete(scheduled_jobs::table.filter(scheduled_jobs::id.eq(id)))
            .execute(&mut conn)
        {
            log::warn!("scheduler: failed to delete job '{id}' from DB: {e}");
        }
    }

    /// Compute the next run time for a job based on its schedule.
    ///
    /// Returns `None` if the schedule cannot be parsed (invalid cron expression).
    pub fn compute_next_run(schedule: &Schedule) -> Option<DateTime<Utc>> {
        match schedule {
            Schedule::Interval { secs } => {
                Some(Utc::now() + chrono::Duration::seconds(*secs as i64))
            }
            Schedule::Cron { expr } => {
                use std::str::FromStr;
                // The `cron` crate expects a 6-field expression (sec min hr dom mon dow).
                // We support either 5-field (min hr dom mon dow) or 6-field.
                let full_expr = if expr.split_whitespace().count() == 5 {
                    format!("0 {expr}")
                } else {
                    expr.clone()
                };
                cron::Schedule::from_str(&full_expr).ok()?.upcoming(Utc).next()
            }
        }
    }

    /// Record a job execution in the history ring buffer.
    fn record_history(history: &Arc<RwLock<HistoryMap>>, exec: JobExecution) {
        if let Ok(mut map) = history.write() {
            let entries = map.entry(exec.job_id.clone()).or_default();
            entries.insert(0, exec);
            entries.truncate(MAX_HISTORY_PER_JOB);
        }
    }
}

#[async_trait]
impl Scheduler for TokioScheduler {
    async fn start(&self) {
        let jobs = self.jobs.clone();
        let history = self.history.clone();
        let bus = self.bus.clone();
        let mut stop_rx = self.stop_rx.clone();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(TICK_INTERVAL_SECS));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let due: Vec<ScheduledJob> = {
                            let guard = match jobs.read() {
                                Ok(g) => g,
                                Err(_) => continue,
                            };
                            guard
                                .values()
                                .filter(|j| j.enabled)
                                .filter(|j| {
                                    j.next_run.map_or(false, |t| t <= Utc::now())
                                })
                                .cloned()
                                .collect()
                        };

                        for job in due {
                            let started_at = Utc::now();
                            let bus_clone = bus.clone();
                            let history_clone = history.clone();
                            let jobs_clone = jobs.clone();
                            let job_clone = job.clone();

                            tokio::spawn(async move {
                                // Emit CronFired / HeartbeatTick event.
                                let event = match &job_clone.payload {
                                    JobPayload::Heartbeat => {
                                        AppEvent::HeartbeatTick {
                                            timestamp: started_at.to_rfc3339(),
                                        }
                                    }
                                    _ => AppEvent::CronFired {
                                        job_id: job_clone.id.clone(),
                                        schedule: format!("{:?}", job_clone.schedule),
                                    },
                                };
                                let _ = bus_clone.publish(event);

                                // Execute with timeout for stuck detection.
                                let timeout = Duration::from_secs(STUCK_THRESHOLD_SECS);
                                let status = tokio::time::timeout(timeout, execute_job(&job_clone))
                                    .await;

                                let (job_status, output) = match status {
                                    Ok((s, o)) => (s, o),
                                    Err(_) => {
                                        let _ = bus_clone.publish(AppEvent::SystemError {
                                            message: format!(
                                                "Job '{}' stuck after {}s",
                                                job_clone.name, STUCK_THRESHOLD_SECS
                                            ),
                                        });
                                        (JobStatus::Stuck, "Execution timed out".to_string())
                                    }
                                };

                                let finished_at = Utc::now();

                                // Record history.
                                let exec = JobExecution {
                                    job_id: job_clone.id.clone(),
                                    started_at,
                                    finished_at,
                                    status: job_status.clone(),
                                    output,
                                };
                                Self::record_history(&history_clone, exec);

                                // Reschedule and update error_count.
                                if let Ok(mut map) = jobs_clone.write() {
                                    if let Some(j) = map.get_mut(&job_clone.id) {
                                        if job_status == JobStatus::Success {
                                            j.error_count = 0;
                                        } else {
                                            j.error_count += 1;
                                        }
                                        j.next_run = Self::compute_next_run(&j.schedule);
                                    }
                                }
                            });
                        }
                    }
                    Ok(()) = stop_rx.changed() => {
                        if *stop_rx.borrow() {
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

    async fn add_job(&self, mut job: ScheduledJob) -> JobId {
        // Assign a fresh ID if none given.
        if job.id.is_empty() {
            job.id = Uuid::new_v4().to_string();
        }
        // Compute initial next_run.
        job.next_run = Self::compute_next_run(&job.schedule);

        // Persist to SQLite before updating in-memory map.
        self.persist_job(&job);

        let id = job.id.clone();
        if let Ok(mut map) = self.jobs.write() {
            map.insert(id.clone(), job);
        }
        id
    }

    async fn remove_job(&self, id: &JobId) -> bool {
        // Remove from SQLite.
        self.delete_job_from_db(id);

        if let Ok(mut map) = self.jobs.write() {
            map.remove(id).is_some()
        } else {
            false
        }
    }

    async fn list_jobs(&self) -> Vec<ScheduledJob> {
        self.jobs
            .read()
            .map(|m| {
                let mut jobs: Vec<ScheduledJob> = m.values().cloned().collect();
                jobs.sort_by(|a, b| a.name.cmp(&b.name));
                jobs
            })
            .unwrap_or_default()
    }

    async fn job_history(&self, id: &JobId) -> Vec<JobExecution> {
        self.history
            .read()
            .map(|m| m.get(id).cloned().unwrap_or_default())
            .unwrap_or_default()
    }
}

// ─── execute_job ─────────────────────────────────────────────────────────────

/// Execute a job's payload, returning `(status, output)`.
///
/// Agent-turn execution is deferred pending full agent state wiring (Phase 3
/// follow-up).  The Heartbeat and Notify payloads are lightweight.
async fn execute_job(job: &ScheduledJob) -> (JobStatus, String) {
    match &job.payload {
        JobPayload::Heartbeat => {
            // ## TODO (Phase 3 follow-up): run heartbeat items via AgentLoop.
            (JobStatus::Success, "Heartbeat tick recorded.".to_string())
        }
        JobPayload::AgentTurn { prompt } => {
            // ## TODO (Phase 3 follow-up): run prompt through AgentLoop.
            (
                JobStatus::Skipped,
                format!("AgentTurn skipped (not yet wired): {prompt}"),
            )
        }
        JobPayload::Notify { message } => {
            // Just log — the event was already published before execute_job is called.
            (JobStatus::Success, format!("Notification sent: {message}"))
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::TokioBroadcastBus;

    fn make_scheduler() -> Arc<TokioScheduler> {
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new());
        TokioScheduler::new(bus)
    }

    fn interval_job(name: &str, secs: u64) -> ScheduledJob {
        ScheduledJob {
            id: String::new(), // assigned by add_job
            name: name.to_string(),
            schedule: Schedule::Interval { secs },
            session_target: super::super::traits::SessionTarget::Main,
            payload: JobPayload::Heartbeat,
            enabled: true,
            error_count: 0,
            next_run: None,
        }
    }

    #[tokio::test]
    async fn add_and_list_job() {
        let scheduler = make_scheduler();
        let job = interval_job("my-job", 60);
        let id = scheduler.add_job(job.clone()).await;

        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 1, "should have 1 job");
        assert_eq!(jobs[0].id, id);
        assert_eq!(jobs[0].name, "my-job");
    }

    #[tokio::test]
    async fn remove_existing_job_returns_true() {
        let scheduler = make_scheduler();
        let id = scheduler.add_job(interval_job("j", 10)).await;
        let removed = scheduler.remove_job(&id).await;
        assert!(removed, "should remove existing job");

        let jobs = scheduler.list_jobs().await;
        assert!(jobs.is_empty(), "no jobs after removal");
    }

    #[tokio::test]
    async fn remove_nonexistent_job_returns_false() {
        let scheduler = make_scheduler();
        let removed = scheduler.remove_job(&"no-such-id".to_string()).await;
        assert!(!removed, "removing unknown job → false");
    }

    #[tokio::test]
    async fn add_job_assigns_id_when_empty() {
        let scheduler = make_scheduler();
        let mut job = interval_job("j", 10);
        job.id = String::new();
        let id = scheduler.add_job(job).await;
        assert!(!id.is_empty(), "scheduler should assign a non-empty id");
    }

    #[tokio::test]
    async fn add_job_uses_provided_id() {
        let scheduler = make_scheduler();
        let mut job = interval_job("j", 10);
        job.id = "custom-id".to_string();
        let id = scheduler.add_job(job).await;
        assert_eq!(id, "custom-id", "should use provided job id");
    }

    #[tokio::test]
    async fn job_history_empty_for_new_job() {
        let scheduler = make_scheduler();
        let id = scheduler.add_job(interval_job("j", 10)).await;
        let hist = scheduler.job_history(&id).await;
        assert!(hist.is_empty(), "new job has no history");
    }

    #[test]
    fn compute_next_run_interval() {
        let before = Utc::now();
        let next = TokioScheduler::compute_next_run(&Schedule::Interval { secs: 300 });
        assert!(next.is_some(), "interval schedule should produce a next_run");
        assert!(next.unwrap() > before, "next_run should be in the future");
    }

    #[test]
    fn compute_next_run_cron_valid() {
        // "* * * * *" = every minute
        let next = TokioScheduler::compute_next_run(&Schedule::Cron {
            expr: "* * * * *".to_string(),
        });
        assert!(next.is_some(), "valid cron expression should produce a next_run");
    }

    #[test]
    fn compute_next_run_cron_invalid() {
        let next = TokioScheduler::compute_next_run(&Schedule::Cron {
            expr: "not a cron expression".to_string(),
        });
        assert!(next.is_none(), "invalid cron expression → None");
    }

    #[test]
    fn compute_next_run_cron_six_field() {
        // 6-field cron (with seconds) should also be accepted.
        let next = TokioScheduler::compute_next_run(&Schedule::Cron {
            expr: "0 * * * * *".to_string(),
        });
        assert!(next.is_some(), "6-field cron should produce a next_run");
    }

    #[tokio::test]
    async fn list_jobs_sorted_by_name() {
        let scheduler = make_scheduler();
        scheduler.add_job(interval_job("zebra", 10)).await;
        scheduler.add_job(interval_job("alpha", 10)).await;
        scheduler.add_job(interval_job("mango", 10)).await;

        let jobs = scheduler.list_jobs().await;
        let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mango", "zebra"], "jobs should be sorted by name");
    }
}
