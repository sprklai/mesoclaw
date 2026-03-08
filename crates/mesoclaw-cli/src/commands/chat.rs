use std::io::{self, BufRead, Write};

use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::tungstenite;

use crate::client::MesoClient;

pub async fn run(
    client: &MesoClient,
    session_id: Option<&str>,
    model: Option<&str>,
) -> Result<(), String> {
    let url = client.ws_url("/ws/chat");

    let mut request = tungstenite::client::IntoClientRequest::into_client_request(url.as_str())
        .map_err(|e| format!("invalid WS URL: {e}"))?;

    if let Some(auth) = client.auth_header_value() {
        request.headers_mut().insert(
            "authorization",
            auth.parse()
                .map_err(|e| format!("invalid auth header: {e}"))?,
        );
    }

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("failed to connect to daemon: {e}"))?;

    let (mut write, mut read) = ws.split();

    println!("Connected to MesoClaw. Type your message and press Enter. Ctrl+C to exit.");
    if let Some(sid) = session_id {
        println!("Session: {sid}");
    }
    if let Some(m) = model {
        println!("Model: {m}");
    }
    println!();

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        print!("> ");
        io::stdout().flush().unwrap_or(());

        let line = match lines.next() {
            Some(Ok(l)) => l,
            _ => break,
        };

        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let mut msg = json!({ "prompt": line });
        if let Some(sid) = session_id {
            msg["session_id"] = json!(sid);
        }
        if let Some(m) = model {
            msg["model"] = json!(m);
        }

        write
            .send(tungstenite::Message::Text(msg.to_string().into()))
            .await
            .map_err(|e| format!("send error: {e}"))?;

        // Read response chunks until "done" or "error"
        while let Some(msg_result) = read.next().await {
            let msg = msg_result.map_err(|e| format!("ws read error: {e}"))?;
            match msg {
                tungstenite::Message::Text(text) => {
                    let chunk: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                    let chunk_type = chunk.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match chunk_type {
                        "text" => {
                            if let Some(content) = chunk.get("content").and_then(|v| v.as_str()) {
                                println!("{content}");
                            }
                        }
                        "tool_call" => {
                            let name = chunk
                                .get("tool_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            eprintln!("\x1b[33m  \u{26A1} {name}...\x1b[0m");
                        }
                        "tool_result" => {
                            let name = chunk
                                .get("tool_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let ok = chunk
                                .get("success")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let ms = chunk
                                .get("duration_ms")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let icon = if ok { "\u{2713}" } else { "\u{2717}" };
                            eprintln!("\x1b[33m  {icon} {name} ({ms}ms)\x1b[0m");
                        }
                        "done" => break,
                        "error" => {
                            if let Some(err) = chunk.get("error").and_then(|v| v.as_str()) {
                                eprintln!("Error: {err}");
                            }
                            break;
                        }
                        _ => {}
                    }
                }
                tungstenite::Message::Close(_) => {
                    println!("Connection closed by server.");
                    return Ok(());
                }
                _ => {}
            }
        }

        println!();
    }

    Ok(())
}
