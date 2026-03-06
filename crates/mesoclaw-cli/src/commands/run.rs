use serde_json::json;

use crate::client::MesoClient;

pub async fn run(
    client: &MesoClient,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
) -> Result<(), String> {
    let mut body = json!({ "prompt": prompt });
    if let Some(sid) = session_id {
        body["session_id"] = json!(sid);
    }
    if let Some(m) = model {
        body["model"] = json!(m);
    }

    let resp: serde_json::Value = client.post("/chat", &body).await?;

    if let Some(response) = resp.get("response").and_then(|v| v.as_str()) {
        println!("{response}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
    }

    Ok(())
}
