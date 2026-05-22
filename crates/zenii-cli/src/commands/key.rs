use serde_json::json;

use crate::client::ZeniiClient;
use crate::commands::encode_path_segment;

pub async fn set(client: &ZeniiClient, provider: &str, key: &str) -> Result<(), String> {
    let credential_key = format!("api_key:{provider}");
    let body = json!({
        "key": credential_key,
        "value": key,
    });
    let _resp: serde_json::Value = client.post("/credentials", &body).await?;
    println!("API key set for provider/service: {provider}");
    Ok(())
}

pub async fn remove(client: &ZeniiClient, provider: &str) -> Result<(), String> {
    let credential_key = format!("api_key:{provider}");
    client
        .delete(&format!("/credentials/{}", encode_path_segment(&credential_key)))
        .await?;
    println!("API key removed for provider/service: {provider}");
    Ok(())
}

pub async fn set_channel(
    client: &ZeniiClient,
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

pub async fn remove_channel(
    client: &ZeniiClient,
    channel: &str,
    field: &str,
) -> Result<(), String> {
    let credential_key = format!("channel:{channel}:{field}");
    client
        .delete(&format!("/credentials/{}", encode_path_segment(&credential_key)))
        .await?;
    println!("Channel credential removed: {channel}/{field}");
    Ok(())
}

pub async fn set_raw(client: &ZeniiClient, key: &str, value: &str) -> Result<(), String> {
    let body = json!({
        "key": key,
        "value": value,
    });
    let _resp: serde_json::Value = client.post("/credentials", &body).await?;
    println!("Credential set: {key}");
    Ok(())
}

pub async fn remove_raw(client: &ZeniiClient, key: &str) -> Result<(), String> {
    client
        .delete(&format!("/credentials/{}", encode_path_segment(key)))
        .await?;
    println!("Credential removed: {key}");
    Ok(())
}

pub async fn list(client: &ZeniiClient) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use serde_json::json;

    use super::*;

    fn test_client(port: u16) -> ZeniiClient {
        ZeniiClient::new("127.0.0.1", port, None)
    }

    // remove encodes the constructed credential key in the DELETE path
    #[tokio::test]
    async fn remove_encodes_provider_name() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            // space in provider name → api_key:my%20provider
            when.method(DELETE).path("/credentials/api_key:my%20provider");
            then.status(200).json_body(json!({}));
        });

        let result = remove(&test_client(server.port()), "my provider").await;
        assert!(result.is_ok());
        assert_eq!(mock.hits(), 1);
    }

    // remove_raw encodes slashes and other special chars in raw keys
    #[tokio::test]
    async fn remove_raw_encodes_slash_in_key() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            // slash encoded as %2F, colon preserved
            when.method(DELETE).path("/credentials/chan%2Fnel:foo");
            then.status(200).json_body(json!({}));
        });

        let result = remove_raw(&test_client(server.port()), "chan/nel:foo").await;
        assert!(result.is_ok());
        assert_eq!(mock.hits(), 1);
    }

    // remove_channel encodes the compound credential key (normal case — colons preserved)
    #[tokio::test]
    async fn remove_channel_normal_key() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(DELETE).path("/credentials/channel:telegram:token");
            then.status(200).json_body(json!({}));
        });

        let result = remove_channel(&test_client(server.port()), "telegram", "token").await;
        assert!(result.is_ok());
        assert_eq!(mock.hits(), 1);
    }

    // remove_channel encodes slashes in field names
    #[tokio::test]
    async fn remove_channel_encodes_slash_in_field() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(DELETE).path("/credentials/channel:slack:bot%2Ftoken");
            then.status(200).json_body(json!({}));
        });

        let result = remove_channel(&test_client(server.port()), "slack", "bot/token").await;
        assert!(result.is_ok());
        assert_eq!(mock.hits(), 1);
    }
}
