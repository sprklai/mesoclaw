# Scheduling Jobs in MesoClaw

MesoClaw includes a built-in job scheduler for recurring tasks like periodic agent prompts, notifications, heartbeat checks, and channel messages. The scheduler is **feature-gated** — enable it with the `scheduler` feature flag.

## Quick Start

### Enable the scheduler

```bash
# Build daemon with scheduler support
cargo run -p mesoclaw-daemon --features scheduler

# Or build with all features
cargo run -p mesoclaw-daemon --all-features
```

The scheduler starts automatically on boot, loading any previously saved jobs from the database.

---

## Concepts

### Schedule Types

| Type | Description | Example |
|------|-------------|---------|
| **Interval** | Run every N seconds | Every 5 minutes (`300s`) |
| **Cron** | Standard cron expression (5, 6, or 7 fields) | `0 9 * * 1-5` (weekdays at 9 AM) |

### Payload Types

| Payload | Description | Use Case |
|---------|-------------|----------|
| **Heartbeat** | Run a heartbeat checklist | System health monitoring |
| **Agent Turn** | Execute the AI agent with a prompt | Periodic summaries, reports |
| **Notify** | Publish a notification event | Reminders, alerts |
| **Send via Channel** | Send a message through a named channel | Telegram/Slack/Discord updates |

### Key Features

- **Active hours** — Restrict jobs to a time window (e.g., 9 AM – 5 PM). Jobs outside the window are skipped.
- **One-shot jobs** — Run once and auto-delete on success. Useful for deferred tasks.
- **Error backoff** — Failed jobs retry with exponential backoff: 30s → 60s → 5m → 15m → 1h.
- **Execution history** — Each job keeps the last 100 execution records (configurable).
- **Persistence** — Jobs survive daemon restarts (stored in SQLite).

---

## Using the Scheduler

### Desktop App (Tauri)

Open the **Schedule** page from the sidebar navigation.

**Creating a job:**

1. Click **"New Job"** to expand the creation form
2. Enter a **name** (e.g., "Morning Briefing")
3. Select a **schedule type**:
   - *Interval*: enter seconds (e.g., `3600` for hourly)
   - *Cron*: enter a cron expression (e.g., `0 9 * * *` for daily at 9 AM)
4. Select a **payload type** and fill in the required fields:
   - *Notify*: enter a message
   - *Agent Turn*: enter a prompt
   - *Heartbeat*: no extra fields
5. Optionally set **session target** (Main or Isolated)
6. Optionally check **One-shot** for single-run jobs
7. Optionally enable **Active hours** and set the time window
8. Click **Create**

**Managing jobs:**

- **Toggle** (play/pause icon) — Enable or disable a job without deleting it
- **History** (clock icon) — View past executions with status badges (green = success, red = failed, yellow = stuck, gray = skipped)
- **Delete** (trash icon) — Permanently remove a job

The scheduler status bar at the top shows whether the scheduler is running and how many jobs are registered.

---

### CLI

All scheduler commands go through the daemon's HTTP API, so ensure the daemon is running with the `scheduler` feature enabled.

```bash
# Check scheduler status
mesoclaw-cli schedule status
# Output: Scheduler: Running (3 jobs)
```

**List all jobs:**

```bash
mesoclaw-cli schedule list
```

Output:

```
ID                                   | Name              | Status   | Schedule         | Next Run              | Errors
-------------------------------------+-------------------+----------+------------------+-----------------------+-------
a1b2c3d4-...                         | Morning Briefing  | enabled  | Cron: 0 9 * * *  | 2026-03-09 09:00:00   | 0
e5f6g7h8-...                         | Health Check      | enabled  | Every 5m         | 2026-03-08 15:35:00   | 0
i9j0k1l2-...                         | Weekly Report     | disabled | Cron: 0 17 * * 5 | —                     | 2
```

**Create a notification job (every 30 minutes):**

```bash
mesoclaw-cli schedule create \
  --name "Hydration Reminder" \
  --schedule-type interval \
  --interval-secs 1800 \
  --payload-type notify \
  --message "Time to drink some water!"
```

**Create a daily agent prompt (weekdays at 9 AM):**

```bash
mesoclaw-cli schedule create \
  --name "Morning Briefing" \
  --schedule-type cron \
  --cron-expr "0 9 * * 1-5" \
  --payload-type agent_turn \
  --prompt "Summarize my unread messages and today's calendar. Keep it brief."
```

**Create a one-shot reminder (run once in 1 hour, then delete):**

```bash
mesoclaw-cli schedule create \
  --name "Deploy Reminder" \
  --schedule-type interval \
  --interval-secs 3600 \
  --payload-type notify \
  --message "Don't forget to deploy the hotfix!" \
  --one-shot
```

**Create a heartbeat job (every 2 minutes, only during work hours):**

```bash
mesoclaw-cli schedule create \
  --name "System Heartbeat" \
  --schedule-type interval \
  --interval-secs 120 \
  --payload-type heartbeat
```

> Active hours are set via the API or Desktop UI. The CLI `create` command creates jobs with no active hours restriction by default.

**Toggle a job on/off:**

```bash
mesoclaw-cli schedule toggle a1b2c3d4-e5f6-7890-abcd-ef1234567890
# Output: Job a1b2c3d4-...: disabled
```

**View execution history:**

```bash
mesoclaw-cli schedule history a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

Output:

```
[success] 2026-03-08 09:00:01
[success] 2026-03-07 09:00:02
[failed]  2026-03-06 09:00:01 — connection timeout
[skipped] 2026-03-05 09:00:00
```

**Delete a job:**

```bash
mesoclaw-cli schedule delete a1b2c3d4-e5f6-7890-abcd-ef1234567890
# Output: Job a1b2c3d4-... deleted.
```

---

### TUI

The TUI (`mesoclaw-tui`) communicates with the same daemon HTTP API. Scheduler management is available through the schedule view panel. Navigation and controls mirror the CLI semantics — list, create, toggle, delete, and view history — rendered in a terminal UI.

---

### HTTP API (Direct)

For scripting or custom integrations, call the gateway endpoints directly. The daemon listens on `127.0.0.1:18981` by default.

**Get scheduler status:**

```bash
curl http://127.0.0.1:18981/scheduler/status
# {"running":true,"job_count":3}
```

**List all jobs:**

```bash
curl http://127.0.0.1:18981/scheduler/jobs
```

**Create a job:**

```bash
curl -X POST http://127.0.0.1:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Evening Summary",
    "schedule": { "Cron": { "expr": "0 18 * * *" } },
    "session_target": "Main",
    "payload": { "AgentTurn": { "prompt": "Give me a summary of what I accomplished today." } },
    "enabled": true,
    "delete_after_run": false
  }'
# {"id":"generated-uuid-here"}
```

**Create a channel notification (requires `channels` feature):**

```bash
curl -X POST http://127.0.0.1:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Telegram Daily Update",
    "schedule": { "Cron": { "expr": "0 8 * * *" } },
    "session_target": "Isolated",
    "payload": { "SendViaChannel": { "channel": "telegram", "message": "Good morning! Here is your daily update." } },
    "enabled": true,
    "active_hours": { "start_hour": 7, "end_hour": 22 },
    "delete_after_run": false
  }'
```

**Toggle a job:**

```bash
curl -X PUT http://127.0.0.1:18981/scheduler/jobs/{job_id}/toggle
# {"id":"...","enabled":false}
```

**View job history:**

```bash
curl http://127.0.0.1:18981/scheduler/jobs/{job_id}/history
```

**Delete a job:**

```bash
curl -X DELETE http://127.0.0.1:18981/scheduler/jobs/{job_id}
# 204 No Content
```

---

## Real-World Examples

### 1. Daily standup prep (weekday mornings)

Have the agent prepare a standup summary every weekday at 8:45 AM:

```bash
mesoclaw-cli schedule create \
  --name "Standup Prep" \
  --schedule-type cron \
  --cron-expr "45 8 * * 1-5" \
  --payload-type agent_turn \
  --prompt "Review my git commits from yesterday, open PRs, and any blocked tasks. Format as a standup update: what I did, what I'm doing, blockers."
```

### 2. Hourly system health check

Run a heartbeat every hour during business hours:

```bash
curl -X POST http://127.0.0.1:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Hourly Health Check",
    "schedule": { "Interval": { "secs": 3600 } },
    "payload": { "Heartbeat": {} },
    "enabled": true,
    "active_hours": { "start_hour": 9, "end_hour": 18 }
  }'
```

### 3. Weekly project report (Friday at 5 PM)

```bash
mesoclaw-cli schedule create \
  --name "Weekly Report" \
  --schedule-type cron \
  --cron-expr "0 17 * * 5" \
  --payload-type agent_turn \
  --prompt "Generate a weekly project report: summarize completed tasks, highlight achievements, list outstanding items, and suggest priorities for next week."
```

### 4. One-shot deploy reminder

Remind yourself in 2 hours to check the deployment:

```bash
mesoclaw-cli schedule create \
  --name "Check Deploy" \
  --schedule-type interval \
  --interval-secs 7200 \
  --payload-type notify \
  --message "Check the production deployment status and verify monitoring dashboards." \
  --one-shot
```

### 5. Telegram channel daily digest

Send a daily digest to your Telegram bot at 7 PM:

```bash
curl -X POST http://127.0.0.1:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Telegram Daily Digest",
    "schedule": { "Cron": { "expr": "0 19 * * *" } },
    "session_target": "Isolated",
    "payload": { "SendViaChannel": { "channel": "telegram", "message": "Daily digest: tasks completed, pending items, tomorrow priorities." } },
    "enabled": true
  }'
```

### 6. Pomodoro break reminders (every 25 minutes during work)

```bash
curl -X POST http://127.0.0.1:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Pomodoro Break",
    "schedule": { "Interval": { "secs": 1500 } },
    "payload": { "Notify": { "message": "Pomodoro complete! Take a 5-minute break." } },
    "enabled": true,
    "active_hours": { "start_hour": 9, "end_hour": 17 }
  }'
```

---

## Configuration

All scheduler tunables live in `~/.config/mesoclaw/config.toml`:

```toml
# How often the scheduler checks for due jobs (seconds)
scheduler_tick_interval_secs = 1

# Time before a running job is marked "stuck" (seconds)
scheduler_stuck_threshold_secs = 120

# Exponential backoff delays for failed jobs (seconds)
scheduler_error_backoff_secs = [30, 60, 300, 900, 3600]

# Maximum execution history entries kept per job
scheduler_max_history_per_job = 100
```

**Tuning tips:**

- For low-resource machines, increase `scheduler_tick_interval_secs` to `5` or `10` to reduce CPU wake-ups
- Increase `scheduler_stuck_threshold_secs` for jobs that call slow external APIs
- Adjust `scheduler_error_backoff_secs` to match your retry tolerance — the array caps at the last value (1 hour by default)

---

## Cron Expression Reference

MesoClaw supports standard 5-field, 6-field (with seconds), and 7-field cron expressions:

```
┌───────────── second (0–59, optional)
│ ┌───────────── minute (0–59)
│ │ ┌───────────── hour (0–23)
│ │ │ ┌───────────── day of month (1–31)
│ │ │ │ ┌───────────── month (1–12)
│ │ │ │ │ ┌───────────── day of week (0–6, Sun=0)
│ │ │ │ │ │
* * * * * *
```

| Expression | Meaning |
|------------|---------|
| `* * * * *` | Every minute |
| `0 * * * *` | Every hour |
| `0 9 * * *` | Daily at 9:00 AM |
| `0 9 * * 1-5` | Weekdays at 9:00 AM |
| `0 0 1 * *` | First of every month at midnight |
| `*/15 * * * *` | Every 15 minutes |
| `0 9,17 * * *` | At 9:00 AM and 5:00 PM |
| `0 0 * * 0` | Every Sunday at midnight |

5-field expressions are normalized by prepending `0` for the seconds field automatically.

---

## Architecture Notes

- **Persistence**: Jobs are stored in the `scheduled_jobs` SQLite table (migration v4). On daemon start, all jobs are loaded into a lock-free `DashMap` for fast in-memory access.
- **Tick loop**: A background `tokio::spawn` task wakes every `tick_interval_secs`, evaluates due jobs, executes payloads, and records results.
- **Event bus**: The scheduler publishes events (`SchedulerStarted`, `SchedulerStopped`, `HeartbeatTick`, `CronFired`) for other system components to react to.
- **Feature gate**: All scheduler code is behind `#[cfg(feature = "scheduler")]`. When disabled, the scheduler routes return empty and no background task runs.
