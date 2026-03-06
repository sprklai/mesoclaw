use serde_json::json;

use crate::client::MesoClient;

pub async fn set(client: &MesoClient, provider: &str, key: &str) -> Result<(), String> {
    let body = json!({
        "provider": provider,
        "api_key": key,
    });
    let _resp: serde_json::Value = client.post("/providers", &body).await?;
    println!("API key set for provider: {provider}");
    Ok(())
}

pub async fn remove(client: &MesoClient, provider: &str) -> Result<(), String> {
    client.delete(&format!("/providers/{provider}")).await?;
    println!("API key removed for provider: {provider}");
    Ok(())
}
