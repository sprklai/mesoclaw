use std::io::{self, BufRead, Write};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::tungstenite;

use crate::client::ZeniiClient;

pub async fn run(
    client: &ZeniiClient,
    session_id: Option<&str>,
    model: Option<&str>,
    delegate: bool,
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

    println!("Connected to Zenii. Type your message and press Enter. Ctrl+C to exit.");
    if let Some(sid) = session_id {
        println!("Session: {sid}");
    }
    if let Some(m) = model {
        println!("Model: {m}");
    }
    if delegate {
        eprintln!("\x1b[36mMode: delegation (multi-agent)\x1b[0m");
    }
    println!();

    loop {
        print!("> ");
        io::stdout().flush().unwrap_or(());

        // Acquire and release the stdin lock per iteration — never hold it across .await.
        let line_opt = tokio::task::spawn_blocking(|| {
            let mut buf = String::new();
            match io::stdin().lock().read_line(&mut buf) {
                Ok(0) | Err(_) => None,
                Ok(_) => Some(buf),
            }
        })
        .await
        .unwrap_or(None);

        let line = match line_opt {
            Some(l) => l.trim().to_string(),
            None => break,
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
        if delegate {
            msg["delegation"] = json!(true);
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
                        "delegation_started" => {
                            let count = chunk
                                .get("agent_count")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            eprintln!(
                                "\n\x1b[36m\u{1F500} Delegation started ({count} agents)\x1b[0m"
                            );
                            if let Some(agents) = chunk.get("agents").and_then(|v| v.as_array()) {
                                let last = agents.len().saturating_sub(1);
                                for (i, agent) in agents.iter().enumerate() {
                                    let id =
                                        agent.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                    let desc = agent
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let connector = if i == last {
                                        "\u{2514}\u{2500}\u{2500}"
                                    } else {
                                        "\u{251C}\u{2500}\u{2500}"
                                    };
                                    eprintln!("\x1b[36m{connector} {id}: {desc}\x1b[0m");
                                }
                            }
                        }
                        "agent_progress" => {
                            let agent_id = chunk
                                .get("agent_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let activity = chunk
                                .get("current_activity")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let tools =
                                chunk.get("tool_uses").and_then(|v| v.as_u64()).unwrap_or(0);
                            let tokens = chunk
                                .get("tokens_used")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            eprintln!(
                                "\x1b[33m\u{26A1} {agent_id}: {activity} ({tools} tools, {tokens} tokens)\x1b[0m"
                            );
                        }
                        "agent_completed" => {
                            let agent_id = chunk
                                .get("agent_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let status = chunk
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let dur = chunk
                                .get("duration_ms")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let tools =
                                chunk.get("tool_uses").and_then(|v| v.as_u64()).unwrap_or(0);
                            let tokens = chunk
                                .get("tokens_used")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let dur_s = dur / 1000.0;
                            if status == "failed" {
                                eprintln!(
                                    "\x1b[31m\u{2717} {agent_id}: Failed ({dur_s:.1}s, {tools} tools, {tokens} tokens)\x1b[0m"
                                );
                            } else {
                                eprintln!(
                                    "\x1b[32m\u{2713} {agent_id}: Completed ({dur_s:.1}s, {tools} tools, {tokens} tokens)\x1b[0m"
                                );
                            }
                        }
                        "delegation_completed" => {
                            let dur = chunk
                                .get("total_duration_ms")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let tokens = chunk
                                .get("total_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let dur_s = dur / 1000.0;
                            eprintln!(
                                "\n\x1b[32m\u{2705} Delegation complete ({dur_s:.1}s, {tokens} tokens)\x1b[0m\n"
                            );
                        }
                        "approval_request" => {
                            let tool_name = chunk
                                .get("tool_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let args_summary = chunk
                                .get("args_summary")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let risk = chunk
                                .get("risk_level")
                                .and_then(|v| v.as_str())
                                .unwrap_or("medium");
                            let reason = chunk.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                            let approval_id = chunk
                                .get("approval_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let timeout = chunk
                                .get("timeout_secs")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(120);

                            eprintln!(
                                "\n\x1b[33m\u{26A0}\u{FE0F}  Tool \"{tool_name}\" needs approval\x1b[0m"
                            );
                            eprintln!("    Command: {args_summary}");
                            eprintln!("    Risk: {risk}");
                            if !reason.is_empty() {
                                eprintln!("    Reason: {reason}");
                            }
                            eprint!(
                                "    [a]pprove / [A]lways allow / [d]eny (auto-deny in {timeout}s): "
                            );
                            io::stderr().flush().unwrap_or(());

                            let decision_result = tokio::time::timeout(
                                Duration::from_secs(timeout),
                                tokio::task::spawn_blocking(|| -> &'static str {
                                    let mut input = String::new();
                                    if io::stdin().lock().read_line(&mut input).is_ok() {
                                        match input.trim() {
                                            "a" | "approve" | "y" | "yes" => "approve",
                                            "A" | "always" => "approve_always",
                                            _ => "deny",
                                        }
                                    } else {
                                        "deny"
                                    }
                                }),
                            )
                            .await;

                            let decision = match decision_result {
                                Ok(Ok(d)) => d,
                                _ => {
                                    eprintln!("\n    (auto-denied: timeout)");
                                    "deny"
                                }
                            };

                            let response = json!({
                                "type": "approval_response",
                                "approval_id": approval_id,
                                "decision": decision,
                            });
                            let _ = write
                                .send(tungstenite::Message::Text(response.to_string().into()))
                                .await;
                            eprintln!("    \u{2192} {decision}");
                        }
                        "approval_resolved" => {
                            let decision = chunk
                                .get("decision")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            eprintln!("\x1b[33m  Approval resolved: {decision}\x1b[0m");
                        }
                        "done" => break,
                        "error" => {
                            if let Some(err) = chunk.get("error").and_then(|v| v.as_str()) {
                                eprintln!("\x1b[31mError: {err}\x1b[0m");
                                if let Some(hint) = chunk.get("hint").and_then(|v| v.as_str()) {
                                    eprintln!("\x1b[33m  Hint: {hint}\x1b[0m");
                                }
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
