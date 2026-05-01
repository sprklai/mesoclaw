use serde_json::json;

use crate::client::ZeniiClient;

pub async fn list(client: &ZeniiClient) -> Result<(), String> {
    let workflows: Vec<serde_json::Value> = client.get("/workflows").await?;

    if workflows.is_empty() {
        println!("No workflows.");
        return Ok(());
    }

    for wf in &workflows {
        let id = wf["id"].as_str().unwrap_or("?");
        let name = wf["name"].as_str().unwrap_or("?");
        let step_count = wf["steps"].as_array().map(|s| s.len()).unwrap_or(0);
        let schedule = wf["schedule"].as_str().unwrap_or("—");

        println!("  {id:<30} {name:<30} {step_count} steps  schedule: {schedule}");
    }
    Ok(())
}

pub async fn get(client: &ZeniiClient, id: &str) -> Result<(), String> {
    let wf: serde_json::Value = client.get(&format!("/workflows/{id}")).await?;

    let name = wf["name"].as_str().unwrap_or("?");
    let description = wf["description"].as_str().unwrap_or("");
    let schedule = wf["schedule"].as_str().unwrap_or("none");

    println!("Workflow: {name}");
    println!("ID:       {id}");
    if !description.is_empty() {
        println!("Desc:     {description}");
    }
    println!("Schedule: {schedule}");

    if let Some(steps) = wf["steps"].as_array() {
        println!("\nSteps ({}):", steps.len());
        for step in steps {
            let sname = step["name"].as_str().unwrap_or("?");
            let stype = step["type"].as_str().unwrap_or("?");
            let deps = step["depends_on"]
                .as_array()
                .map(|d| {
                    d.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let deps_str = if deps.is_empty() {
                String::new()
            } else {
                format!(" (depends: {deps})")
            };
            println!("  - {sname} [{stype}]{deps_str}");
        }
    }
    Ok(())
}

pub async fn show(client: &ZeniiClient, id: &str) -> Result<(), String> {
    let toml = client.get_text(&format!("/workflows/{id}/raw")).await?;
    println!("{toml}");
    Ok(())
}

pub async fn create(client: &ZeniiClient, file: &str) -> Result<(), String> {
    let toml_content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {file}: {e}"))?;

    let body = json!({ "toml_content": toml_content });
    let result: serde_json::Value = client.post("/workflows", &body).await?;
    let id = result["id"].as_str().unwrap_or("?");
    println!("Workflow created: {id}");
    Ok(())
}

pub async fn update(client: &ZeniiClient, id: &str, file: &str) -> Result<(), String> {
    let toml_content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {file}: {e}"))?;

    let body = json!({ "toml_content": toml_content });
    let result: serde_json::Value = client.put(&format!("/workflows/{id}"), &body).await?;
    let updated_id = result["id"].as_str().unwrap_or(id);
    println!("Workflow updated: {updated_id}");
    Ok(())
}

pub async fn run(client: &ZeniiClient, id: &str) -> Result<(), String> {
    let result: serde_json::Value = client
        .post(&format!("/workflows/{id}/run"), &json!({}))
        .await?;
    let wf_id = result["workflow_id"].as_str().unwrap_or(id);
    let run_id = result["run_id"].as_str().unwrap_or("?");
    println!("Workflow started: {wf_id} (run_id: {run_id})");
    Ok(())
}

pub async fn delete(client: &ZeniiClient, id: &str) -> Result<(), String> {
    client.delete(&format!("/workflows/{id}")).await?;
    println!("Workflow {id} deleted.");
    Ok(())
}

pub async fn history(client: &ZeniiClient, id: &str) -> Result<(), String> {
    let entries: Vec<serde_json::Value> = client.get(&format!("/workflows/{id}/history")).await?;

    if entries.is_empty() {
        println!("No execution history for workflow {id}.");
        return Ok(());
    }

    for entry in &entries {
        let status = entry["status"].as_str().unwrap_or("?");
        let started = entry["started_at"].as_str().unwrap_or("?");
        let duration = entry["duration_ms"].as_u64();
        let error = entry["error"].as_str();

        let dur_str = duration.map(|ms| format!(" ({ms}ms)")).unwrap_or_default();
        if let Some(err) = error {
            println!("  [{status}] {started}{dur_str} — {err}");
        } else {
            println!("  [{status}] {started}{dur_str}");
        }
    }
    Ok(())
}

pub async fn cancel(client: &ZeniiClient, id: &str, run_id: &str) -> Result<(), String> {
    client
        .post_no_response(&format!("/workflows/{id}/runs/{run_id}/cancel"), &json!({}))
        .await?;
    println!("Workflow {id} run {run_id} cancelled.");
    Ok(())
}
