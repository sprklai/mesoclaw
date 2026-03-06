use serde_json::json;

use crate::client::MesoClient;

pub async fn show(client: &MesoClient) -> Result<(), String> {
    let config: serde_json::Value = client.get("/config").await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&config).unwrap_or_default()
    );
    Ok(())
}

pub async fn set(client: &MesoClient, key: &str, value: &str) -> Result<(), String> {
    // Try to parse value as JSON (number, bool, etc.), fall back to string
    let json_value: serde_json::Value = serde_json::from_str(value).unwrap_or(json!(value));

    let body = json!({ key: json_value });
    let resp: serde_json::Value = client.put("/config", &body).await?;

    let status = resp
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    println!("Config update {status}: {key} = {value}");
    Ok(())
}
