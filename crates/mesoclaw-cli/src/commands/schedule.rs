use serde_json::json;

use crate::client::MesoClient;

pub async fn list(client: &MesoClient) -> Result<(), String> {
    let jobs: Vec<serde_json::Value> = client.get("/scheduler/jobs").await?;

    if jobs.is_empty() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    for job in &jobs {
        let id = job["id"].as_str().unwrap_or("?");
        let name = job["name"].as_str().unwrap_or("?");
        let enabled = job["enabled"].as_bool().unwrap_or(false);
        let error_count = job["error_count"].as_u64().unwrap_or(0);
        let next_run = job["next_run"].as_str().unwrap_or("—");

        let schedule = if let Some(sched) = job["schedule"].as_object() {
            match sched.get("type").and_then(|t| t.as_str()) {
                Some("interval") => {
                    let secs = sched.get("secs").and_then(|s| s.as_u64()).unwrap_or(0);
                    format!("every {secs}s")
                }
                Some("cron") => {
                    let expr = sched.get("expr").and_then(|e| e.as_str()).unwrap_or("?");
                    format!("cron: {expr}")
                }
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        };

        let status = if enabled { "enabled" } else { "disabled" };
        let errors = if error_count > 0 {
            format!(" ({error_count} errors)")
        } else {
            String::new()
        };

        println!("  {id:<36} {name:<25} [{status}] {schedule:<20} next: {next_run}{errors}");
    }
    Ok(())
}

pub struct CreateJobArgs<'a> {
    pub name: &'a str,
    pub schedule_type: &'a str,
    pub interval_secs: Option<u64>,
    pub cron_expr: Option<&'a str>,
    pub payload_type: &'a str,
    pub message: Option<&'a str>,
    pub prompt: Option<&'a str>,
    pub one_shot: bool,
}

pub async fn create(client: &MesoClient, args: CreateJobArgs<'_>) -> Result<(), String> {
    let schedule = match args.schedule_type {
        "interval" => {
            let secs = args
                .interval_secs
                .ok_or("--interval-secs required for interval schedule")?;
            json!({ "type": "interval", "secs": secs })
        }
        "cron" => {
            let expr = args
                .cron_expr
                .ok_or("--cron-expr required for cron schedule")?;
            json!({ "type": "cron", "expr": expr })
        }
        _ => return Err(format!("Unknown schedule type: {}", args.schedule_type)),
    };

    let payload = match args.payload_type {
        "heartbeat" => json!({ "type": "heartbeat" }),
        "notify" => {
            let msg = args
                .message
                .ok_or("--message required for notify payload")?;
            json!({ "type": "notify", "message": msg })
        }
        "agent_turn" => {
            let p = args
                .prompt
                .ok_or("--prompt required for agent_turn payload")?;
            json!({ "type": "agent_turn", "prompt": p })
        }
        _ => return Err(format!("Unknown payload type: {}", args.payload_type)),
    };

    let body = json!({
        "id": "",
        "name": args.name,
        "schedule": schedule,
        "session_target": "main",
        "payload": payload,
        "active_hours": null,
        "delete_after_run": args.one_shot,
    });

    let result: serde_json::Value = client.post("/scheduler/jobs", &body).await?;
    let id = result["id"].as_str().unwrap_or("?");
    println!("Job created: {id}");
    Ok(())
}

pub async fn toggle(client: &MesoClient, id: &str) -> Result<(), String> {
    let result: serde_json::Value = client
        .put(&format!("/scheduler/jobs/{id}/toggle"), &json!({}))
        .await?;
    let enabled = result["enabled"].as_bool().unwrap_or(false);
    let state = if enabled { "enabled" } else { "disabled" };
    println!("Job {id}: {state}");
    Ok(())
}

pub async fn delete(client: &MesoClient, id: &str) -> Result<(), String> {
    client.delete(&format!("/scheduler/jobs/{id}")).await?;
    println!("Job {id} deleted.");
    Ok(())
}

pub async fn status(client: &MesoClient) -> Result<(), String> {
    let result: serde_json::Value = client.get("/scheduler/status").await?;
    let running = result["running"].as_bool().unwrap_or(false);
    let job_count = result["job_count"].as_u64().unwrap_or(0);
    let state = if running { "Running" } else { "Stopped" };
    println!("Scheduler: {state} ({job_count} jobs)");
    Ok(())
}

pub async fn history(client: &MesoClient, id: &str) -> Result<(), String> {
    let entries: Vec<serde_json::Value> =
        client.get(&format!("/scheduler/jobs/{id}/history")).await?;

    if entries.is_empty() {
        println!("No execution history for job {id}.");
        return Ok(());
    }

    for entry in &entries {
        let status = entry["status"].as_str().unwrap_or("?");
        let started = entry["started_at"].as_str().unwrap_or("?");
        let error = entry["error"].as_str();

        if let Some(err) = error {
            println!("  [{status}] {started} — {err}");
        } else {
            println!("  [{status}] {started}");
        }
    }
    Ok(())
}
