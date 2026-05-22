use serde_json::json;

use crate::client::ZeniiClient;
use crate::commands::encode_path_segment;

pub async fn list(client: &ZeniiClient) -> Result<(), String> {
    let providers: Vec<serde_json::Value> = client.get("/providers/with-key-status").await?;

    if providers.is_empty() {
        println!("No providers configured.");
        return Ok(());
    }

    for p in &providers {
        let id = p["id"].as_str().unwrap_or("?");
        let name = p["name"].as_str().unwrap_or("?");
        let has_key = p["has_api_key"].as_bool().unwrap_or(false);
        let requires_key = p["requires_api_key"].as_bool().unwrap_or(true);

        let status = if !requires_key {
            "Local"
        } else if has_key {
            "Configured"
        } else {
            "Not Configured"
        };

        println!("  {id:<25} {name:<20} [{status}]");
    }
    Ok(())
}

pub async fn test_connection(client: &ZeniiClient, provider_id: &str) -> Result<(), String> {
    println!("Testing connection to {provider_id}...");
    let result: serde_json::Value = client
        .post(&format!("/providers/{provider_id}/test"), &json!({}))
        .await?;

    let success = result["success"].as_bool().unwrap_or(false);
    let message = result["message"].as_str().unwrap_or("unknown");
    let latency = result["latency_ms"].as_u64();

    if success {
        if let Some(ms) = latency {
            println!("  Success ({ms}ms): {message}");
        } else {
            println!("  Success: {message}");
        }
    } else {
        println!("  Failed: {message}");
    }
    Ok(())
}

pub async fn add(client: &ZeniiClient, id: &str, name: &str, base_url: &str) -> Result<(), String> {
    let body = json!({
        "id": id,
        "name": name,
        "base_url": base_url,
        "requires_api_key": true,
        "models": [],
    });
    let _resp: serde_json::Value = client.post("/providers", &body).await?;
    println!("Provider '{id}' added.");
    Ok(())
}

pub async fn remove(client: &ZeniiClient, id: &str) -> Result<(), String> {
    client.delete(&format!("/providers/{id}")).await?;
    println!("Provider '{id}' removed.");
    Ok(())
}

pub async fn set_default(
    client: &ZeniiClient,
    provider_id: &str,
    model_id: &str,
) -> Result<(), String> {
    let body = json!({
        "provider_id": provider_id,
        "model_id": model_id,
    });
    let _resp: serde_json::Value = client.put("/providers/default", &body).await?;
    println!("Default model set to {provider_id}/{model_id}");
    Ok(())
}
