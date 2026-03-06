use serde_json::json;

use crate::client::MesoClient;

pub async fn search(
    client: &MesoClient,
    query: &str,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<(), String> {
    let mut path = format!("/memory?q={}", urlencoded(query));
    if let Some(l) = limit {
        path.push_str(&format!("&limit={l}"));
    }
    if let Some(o) = offset {
        path.push_str(&format!("&offset={o}"));
    }

    let results: Vec<serde_json::Value> = client.get(&path).await?;

    if results.is_empty() {
        println!("No memories found.");
    } else {
        for entry in &results {
            let key = entry.get("key").and_then(|v| v.as_str()).unwrap_or("?");
            let content = entry.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let category = entry
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("core");
            println!("[{category}] {key}: {content}");
        }
        println!("\n{} result(s)", results.len());
    }

    Ok(())
}

pub async fn add(client: &MesoClient, key: &str, content: &str) -> Result<(), String> {
    let body = json!({
        "key": key,
        "content": content,
    });
    client.post_no_response("/memory", &body).await?;
    println!("Memory stored: {key}");
    Ok(())
}

pub async fn remove(client: &MesoClient, key: &str) -> Result<(), String> {
    client
        .delete(&format!("/memory/{}", urlencoded(key)))
        .await?;
    println!("Memory removed: {key}");
    Ok(())
}

fn urlencoded(s: &str) -> String {
    // Percent-encode query parameters
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    result
}
