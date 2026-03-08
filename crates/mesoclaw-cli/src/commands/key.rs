use serde_json::json;

use crate::client::MesoClient;

pub async fn set(client: &MesoClient, provider: &str, key: &str) -> Result<(), String> {
    let credential_key = format!("api_key:{provider}");
    let body = json!({
        "key": credential_key,
        "value": key,
    });
    let _resp: serde_json::Value = client.post("/credentials", &body).await?;
    println!("API key set for provider/service: {provider}");
    Ok(())
}

pub async fn remove(client: &MesoClient, provider: &str) -> Result<(), String> {
    let credential_key = format!("api_key:{provider}");
    client
        .delete(&format!("/credentials/{credential_key}"))
        .await?;
    println!("API key removed for provider/service: {provider}");
    Ok(())
}

pub async fn set_channel(
    client: &MesoClient,
    channel: &str,
    field: &str,
    value: &str,
) -> Result<(), String> {
    let credential_key = format!("channel:{channel}:{field}");
    let body = json!({
        "key": credential_key,
        "value": value,
    });
    let _resp: serde_json::Value = client.post("/credentials", &body).await?;
    println!("Channel credential set: {channel}/{field}");
    Ok(())
}

pub async fn remove_channel(client: &MesoClient, channel: &str, field: &str) -> Result<(), String> {
    let credential_key = format!("channel:{channel}:{field}");
    client
        .delete(&format!("/credentials/{credential_key}"))
        .await?;
    println!("Channel credential removed: {channel}/{field}");
    Ok(())
}

pub async fn set_raw(client: &MesoClient, key: &str, value: &str) -> Result<(), String> {
    let body = json!({
        "key": key,
        "value": value,
    });
    let _resp: serde_json::Value = client.post("/credentials", &body).await?;
    println!("Credential set: {key}");
    Ok(())
}

pub async fn remove_raw(client: &MesoClient, key: &str) -> Result<(), String> {
    client.delete(&format!("/credentials/{key}")).await?;
    println!("Credential removed: {key}");
    Ok(())
}

pub async fn list(client: &MesoClient) -> Result<(), String> {
    let keys: Vec<String> = client.get("/credentials").await?;
    if keys.is_empty() {
        println!("No credentials stored.");
        return Ok(());
    }

    let mut api_keys: Vec<&str> = Vec::new();
    let mut channel_keys: Vec<&str> = Vec::new();
    let mut other_keys: Vec<&str> = Vec::new();

    for key in &keys {
        if key.starts_with("api_key:") {
            api_keys.push(key);
        } else if key.starts_with("channel:") {
            channel_keys.push(key);
        } else {
            other_keys.push(key);
        }
    }

    if !api_keys.is_empty() {
        println!("AI Providers & Services:");
        for key in &api_keys {
            let name = key.strip_prefix("api_key:").unwrap_or(key);
            println!("  {name}");
        }
    }

    if !channel_keys.is_empty() {
        if !api_keys.is_empty() {
            println!();
        }
        println!("Channels:");
        for key in &channel_keys {
            let rest = key.strip_prefix("channel:").unwrap_or(key);
            println!("  {rest}");
        }
    }

    if !other_keys.is_empty() {
        if !api_keys.is_empty() || !channel_keys.is_empty() {
            println!();
        }
        println!("Other:");
        for key in &other_keys {
            println!("  {key}");
        }
    }

    Ok(())
}
