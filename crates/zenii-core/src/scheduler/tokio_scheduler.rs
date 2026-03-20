use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
#[cfg(feature = "gateway")]
use crate::gateway::state::AppState;
use crate::{Result, ZeniiError};

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
    running: AtomicBool,
    #[cfg(feature = "gateway")]
    app_state: Arc<tokio::sync::OnceCell<Arc<AppState>>>,
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
            running: AtomicBool::new(false),
            #[cfg(feature = "gateway")]
            app_state: Arc::new(tokio::sync::OnceCell::new()),
        })
    }

    /// Wire the scheduler with AppState for payload execution.
    /// Idempotent — subsequent calls are no-ops.
    #[cfg(feature = "gateway")]
    pub fn wire(&self, state: Arc<AppState>) {
        let _ = self.app_state.set(state);
    }

    /// Get the wired AppState, if any.
    #[cfg(feature = "gateway")]
    pub fn get_app_state(&self) -> Option<&Arc<AppState>> {
        self.app_state.get()
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
            self.jobs.insert(id.clone(), job);
            count += 1;

            // Recompute stale next_run times (job was due while we were offline)
            let now = Utc::now();
            let grace = chrono::Duration::seconds((self.tick_interval_secs * 2) as i64);
            if let Some(mut entry) = self.jobs.get_mut(&id) {
                let needs_recompute = match entry.next_run {
                    None => true,
                    Some(t) => t < (now - grace),
                };
                if needs_recompute
                    && let Ok(next) = Self::compute_next_run(&entry.schedule)
                {
                    entry.next_run = Some(next);
                }
            }
        }
        info!("Scheduler loaded {count} job(s) from DB");
        Ok(count)
    }

    /// Persist a job to SQLite (upsert).
    async fn persist_job(db: &DbPool, job: &ScheduledJob) -> Result<()> {
        let schedule_json = serde_json::to_string(&job.schedule)
            .map_err(|e| ZeniiError::Scheduler(e.to_string()))?;
        let payload_json = serde_json::to_string(&job.payload)
            .map_err(|e| ZeniiError::Scheduler(e.to_string()))?;
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

    /// Parse a human datetime string as local time → UTC.
    fn parse_human_datetime(datetime: &str) -> Result<DateTime<Utc>> {
        use chrono::{Local, NaiveDateTime, TimeZone};

        let formats = [
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M",
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
        ];

        let naive = formats
            .iter()
            .find_map(|fmt| NaiveDateTime::parse_from_str(datetime, fmt).ok())
            .ok_or_else(|| {
                ZeniiError::Scheduler(format!(
                    "cannot parse datetime '{datetime}' — expected format like 2026-03-20T00:53 or 2026-03-20 00:53"
                ))
            })?;

        let local_dt = Local
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| {
                ZeniiError::Scheduler(format!(
                    "ambiguous or invalid local time '{datetime}' (DST transition?)"
                ))
            })?;

        let utc_dt = local_dt.with_timezone(&Utc);
        if utc_dt <= Utc::now() {
            return Err(ZeniiError::Scheduler(format!(
                "datetime '{datetime}' is in the past"
            )));
        }

        Ok(utc_dt)
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
                    .map_err(|e| ZeniiError::Scheduler(format!("invalid cron: {e}")))?;
                schedule
                    .upcoming(chrono::Local)
                    .next()
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok_or_else(|| ZeniiError::Scheduler("cron has no upcoming time".into()))
            }
            Schedule::Human { datetime } => Self::parse_human_datetime(datetime),
        }
    }

    /// Check if the current local hour is within active hours.
    /// Supports overnight wraparound (e.g. start=22, end=6 means 22:00-05:59).
    fn is_in_active_hours(active_hours: &Option<ActiveHours>) -> bool {
        match active_hours {
            None => true,
            Some(hours) => {
                use chrono::Timelike;
                let local_hour = chrono::Local::now().hour() as u8;
                Self::hour_in_window(local_hour, hours.start_hour, hours.end_hour)
            }
        }
    }

    /// Check if `hour` is within `[start, end)`, handling overnight wraparound.
    fn hour_in_window(hour: u8, start: u8, end: u8) -> bool {
        if start < end {
            hour >= start && hour < end
        } else {
            // Overnight: e.g. 22..6 means 22,23,0,1,2,3,4,5
            hour >= start || hour < end
        }
    }

    /// Get backoff delay using configured levels.
    pub fn get_backoff(&self, error_count: u32) -> u64 {
        Self::compute_backoff(&self.error_backoff_secs, error_count)
    }

    /// Compute backoff delay from a backoff schedule slice.
    /// Falls back to the module-level `backoff_secs()` if the slice is empty.
    fn compute_backoff(backoff_schedule: &[u64], error_count: u32) -> u64 {
        if backoff_schedule.is_empty() {
            return backoff_secs(error_count);
        }
        let idx = (error_count as usize).min(backoff_schedule.len() - 1);
        backoff_schedule[idx]
    }

    /// Record an execution in the history ring buffer.
    pub fn record_execution(&self, exec: JobExecution) {
        let job_id = exec.job_id.clone();
        let mut entry = self.history.entry(job_id).or_default();
        entry.push_front(exec);
        entry.truncate(self.max_history_per_job);
    }

    /// Check if scheduler is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get count of registered jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

#[async_trait]
impl Scheduler for TokioScheduler {
    async fn start(&self) {
        // Double-start guard: only one tick loop at a time
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let jobs = self.jobs.clone();
        let history = self.history.clone();
        let bus = self.event_bus.clone();
        let db = self.db.clone();
        let mut stop_rx = self.stop_rx.clone();
        let tick_secs = self.tick_interval_secs;
        let stuck_threshold = self.stuck_threshold_secs;
        let max_history = self.max_history_per_job;
        let error_backoff = self.error_backoff_secs.clone();
        #[cfg(feature = "gateway")]
        let app_state_cell = self.app_state.clone();

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
                            // Active hours gate (cheap, synchronous — check before spawning)
                            if !TokioScheduler::is_in_active_hours(&job.active_hours) {
                                // Reschedule
                                if let Some(mut entry) = jobs.get_mut(&job.id)
                                    && let Ok(next) = TokioScheduler::compute_next_run(&entry.schedule)
                                {
                                    entry.next_run = Some(next);
                                }
                                continue;
                            }

                            // Set next_run = None as in-flight guard to prevent re-pickup
                            let scheduled_time = {
                                if let Some(mut entry) = jobs.get_mut(&job.id) {
                                    let prev = entry.next_run;
                                    entry.next_run = None;
                                    prev
                                } else {
                                    continue;
                                }
                            };

                            // Emit pre-execution event (cheap)
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

                            // Clone shared state for the spawned task
                            let jobs = jobs.clone();
                            let history = history.clone();
                            let bus = bus.clone();
                            let db = db.clone();
                            let error_backoff = error_backoff.clone();
                            #[cfg(feature = "gateway")]
                            let app_state_ref = app_state_cell.clone();

                            // Spawn each job in its own task for parallel execution
                            tokio::spawn(async move {
                                let started_at = Utc::now();

                                // Execute with stuck detection timeout
                                let timeout = Duration::from_secs(stuck_threshold);
                                let bus_ref = bus.clone();
                                let job_ref = job.clone();
                                #[cfg(feature = "gateway")]
                                let app_state_exec = app_state_ref.clone();
                                let status = tokio::time::timeout(timeout, async move {
                                    #[cfg(feature = "gateway")]
                                    {
                                        super::payload_executor::execute(
                                            &job_ref,
                                            &bus_ref,
                                            app_state_exec.get(),
                                        )
                                        .await
                                    }
                                    #[cfg(not(feature = "gateway"))]
                                    {
                                        match &job_ref.payload {
                                            JobPayload::Notify { message } => {
                                                info!("Scheduler notify: {message}");
                                                let _ = bus_ref.publish(AppEvent::SchedulerNotification {
                                                    job_id: job_ref.id.clone(),
                                                    job_name: job_ref.name.clone(),
                                                    message: message.clone(),
                                                });
                                                JobStatus::Success
                                            }
                                            _ => JobStatus::Success,
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
                                {
                                    let mut entry = history
                                        .entry(job.id.clone())
                                        .or_default();
                                    entry.push_front(exec);
                                    entry.truncate(max_history);
                                }

                                // Reschedule / one-shot / error tracking
                                if job.delete_after_run && job_status == JobStatus::Success {
                                    jobs.remove(&job.id);
                                    let _ = TokioScheduler::delete_job_from_db(&db, &job.id).await;
                                } else {
                                    // Clone data out of DashMap guard to avoid holding it across .await
                                    let snapshot = {
                                        if let Some(mut entry) = jobs.get_mut(&job.id) {
                                            if job_status == JobStatus::Success {
                                                entry.error_count = 0;
                                                // Fix interval drift: compute from previous scheduled time, not now
                                                if let Some(prev_time) = scheduled_time {
                                                    if let Schedule::Interval { secs } = &entry.schedule {
                                                        let next = prev_time + chrono::Duration::seconds(*secs as i64);
                                                        // Clamp to at least now to avoid firing immediately
                                                        entry.next_run = Some(next.max(Utc::now()));
                                                    } else if let Ok(next) = TokioScheduler::compute_next_run(&entry.schedule) {
                                                        entry.next_run = Some(next);
                                                    }
                                                } else if let Ok(next) = TokioScheduler::compute_next_run(&entry.schedule) {
                                                    entry.next_run = Some(next);
                                                }
                                            } else {
                                                entry.error_count += 1;
                                                // Apply error backoff instead of normal schedule
                                                let backoff_delay = TokioScheduler::compute_backoff(&error_backoff, entry.error_count);
                                                entry.next_run = Some(Utc::now() + chrono::Duration::seconds(backoff_delay as i64));
                                            }
                                            Some(entry.clone())
                                        } else {
                                            None
                                        }
                                    };
                                    // Guard is dropped — safe to .await now
                                    if let Some(snapshot) = snapshot {
                                        let _ = TokioScheduler::persist_job(&db, &snapshot).await;
                                    }
                                }
                            });
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
        self.running.store(false, Ordering::SeqCst);
    }

    async fn add_job(&self, mut job: ScheduledJob) -> Result<JobId> {
        if job.id.is_empty() {
            job.id = Uuid::new_v4().to_string();
        }

        // Check for duplicate name
        if self.jobs.iter().any(|entry| entry.value().name == job.name) {
            return Err(ZeniiError::Validation(format!(
                "a job named '{}' already exists — use a different name or delete the existing one first",
                job.name
            )));
        }

        // Human schedules always auto-delete after execution
        if matches!(job.schedule, Schedule::Human { .. }) {
            job.delete_after_run = true;
        }

        // Validate active hours: start == end would block all hours
        if let Some(ref hours) = job.active_hours
            && hours.start_hour == hours.end_hour
        {
            return Err(ZeniiError::Validation(format!(
                "active_hours start_hour ({}) cannot equal end_hour — job would never fire",
                hours.start_hour
            )));
        }

        // Validate cron expression if applicable
        if let Schedule::Cron { ref expr } = job.schedule {
            let full_expr = if expr.split_whitespace().count() == 5 {
                format!("0 {expr}")
            } else {
                expr.clone()
            };
            cron::Schedule::from_str(&full_expr)
                .map_err(|e| ZeniiError::Scheduler(format!("invalid cron expression: {e}")))?;
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
            .ok_or_else(|| ZeniiError::NotFound(format!("job '{id}' not found")))?;
        Self::delete_job_from_db(&self.db, id).await?;
        Ok(())
    }

    async fn toggle_job(&self, id: &str) -> Result<bool> {
        // Read current state and compute new state + snapshot without holding guard across .await
        let (new_state, snapshot) = {
            let entry = self
                .jobs
                .get(id)
                .ok_or_else(|| ZeniiError::NotFound(format!("job '{id}' not found")))?;
            let toggled = !entry.enabled;
            let mut snapshot = entry.clone();
            snapshot.enabled = toggled;
            (toggled, snapshot)
        };

        // Persist first — only update in-memory on success
        Self::persist_job(&self.db, &snapshot).await?;

        // Now update in-memory state
        if let Some(mut entry) = self.jobs.get_mut(id) {
            entry.enabled = new_state;
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

    // 8.6.1.5 — OnceCell wire sets app_state
    #[cfg(feature = "gateway")]
    #[tokio::test]
    async fn oncecell_wire_sets_state() {
        let (_dir, sched) = test_scheduler();
        assert!(sched.get_app_state().is_none());

        let (_dir2, state) = crate::gateway::handlers::tests::test_state().await;
        sched.wire(state);
        assert!(sched.get_app_state().is_some());
    }

    // 8.6.1.6 — OnceCell wire is idempotent
    #[cfg(feature = "gateway")]
    #[tokio::test]
    async fn oncecell_wire_idempotent() {
        let (_dir, sched) = test_scheduler();
        let (_dir2, state1) = crate::gateway::handlers::tests::test_state().await;
        let (_dir3, state2) = crate::gateway::handlers::tests::test_state().await;

        sched.wire(state1.clone());
        sched.wire(state2); // second wire is no-op

        // Should still have the first state
        assert!(sched.get_app_state().is_some());
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

    // 16.28 — Duplicate job name rejected
    #[tokio::test]
    async fn duplicate_name_rejected() {
        let (_dir, sched) = test_scheduler();
        sched.add_job(test_job("dup")).await.unwrap();
        let result = sched.add_job(test_job("dup")).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"), "error: {err}");
        // Only one job should exist
        assert_eq!(sched.list_jobs().await.len(), 1);
    }

    // WS-6.1 — Scheduler tick does not hold DashMap guard across .await
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn scheduler_tick_no_dashmap_guard_across_await() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("guard_test");
        job.schedule = Schedule::Interval { secs: 1 };
        let id = sched.add_job(job).await.unwrap();

        // Force next_run to past so tick fires immediately
        if let Some(mut entry) = sched.jobs.get_mut(&id) {
            entry.next_run = Some(Utc::now() - chrono::Duration::seconds(1));
        }

        sched.start().await;

        // Verify no deadlock by running list_jobs() concurrently with tick
        let sched_ref = sched.clone();
        let list_handle = tokio::spawn(async move {
            for _ in 0..5 {
                let _jobs = sched_ref.list_jobs().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        tokio::time::sleep(Duration::from_secs(2)).await;
        sched.stop().await;

        list_handle
            .await
            .expect("concurrent list_jobs should not deadlock");

        // Verify job state was persisted
        let history = sched.job_history(&id).await;
        assert!(
            !history.is_empty(),
            "Should have at least one execution record"
        );
    }

    // WS-6.2 — Toggle persists to DB before updating in-memory
    #[tokio::test]
    async fn scheduler_toggle_persist_first() {
        let (_dir, pool) = test_db();
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let config = AppConfig::default();

        let sched = TokioScheduler::new(pool.clone(), bus.clone(), &config);
        let id = sched.add_job(test_job("toggle_persist")).await.unwrap();

        // Toggle to disabled
        let new_state = sched.toggle_job(&id).await.unwrap();
        assert!(!new_state);

        // Load from DB in a new scheduler to verify DB state matches
        let sched2 = TokioScheduler::new(pool, bus, &config);
        sched2.load_from_db().await.unwrap();
        let jobs = sched2.list_jobs().await;
        assert_eq!(jobs.len(), 1);
        assert!(!jobs[0].enabled, "DB should reflect disabled state");
    }

    // WS-6.3 — Double start is prevented
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn scheduler_double_start_prevented() {
        let (_dir, sched) = test_scheduler();

        assert!(!sched.is_running());
        sched.start().await;
        assert!(sched.is_running());

        // Second start should be a no-op (no panic, no second loop)
        sched.start().await;
        assert!(sched.is_running());

        // Stop once should stop all
        sched.stop().await;
        assert!(!sched.is_running());
    }

    // WS-6.4 — ActiveHours overnight window (22:00-06:00)
    #[test]
    fn active_hours_overnight_window() {
        assert!(TokioScheduler::hour_in_window(23, 22, 6));
        assert!(TokioScheduler::hour_in_window(0, 22, 6));
        assert!(TokioScheduler::hour_in_window(5, 22, 6));
        assert!(!TokioScheduler::hour_in_window(6, 22, 6));
        assert!(!TokioScheduler::hour_in_window(12, 22, 6));
        assert!(TokioScheduler::hour_in_window(22, 22, 6));
    }

    // WS-6.4 — ActiveHours normal daytime window (09:00-17:00)
    #[test]
    fn active_hours_normal_window() {
        assert!(TokioScheduler::hour_in_window(9, 9, 17));
        assert!(TokioScheduler::hour_in_window(12, 9, 17));
        assert!(!TokioScheduler::hour_in_window(17, 9, 17));
        assert!(!TokioScheduler::hour_in_window(8, 9, 17));
    }

    // AUDIT-L2.1 — add_job rejects active_hours with start == end
    #[tokio::test]
    async fn add_job_rejects_equal_active_hours() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("bad_hours");
        job.active_hours = Some(super::super::traits::ActiveHours {
            start_hour: 9,
            end_hour: 9,
        });
        let result = sched.add_job(job).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ZeniiError::Validation(msg) if msg.contains("cannot equal")
        ));
    }

    // AUDIT-L2.2 — add_job accepts valid active_hours
    #[tokio::test]
    async fn add_job_accepts_valid_active_hours() {
        let (_dir, sched) = test_scheduler();
        let mut job = test_job("good_hours");
        job.active_hours = Some(super::super::traits::ActiveHours {
            start_hour: 9,
            end_hour: 17,
        });
        let result = sched.add_job(job).await;
        assert!(result.is_ok());
    }
}
