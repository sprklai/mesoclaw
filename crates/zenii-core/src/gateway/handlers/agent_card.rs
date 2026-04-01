use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::gateway::state::AppState;

/// Serve the A2A Agent Card at `/.well-known/agent.json`.
///
/// Returns a JSON document describing Zenii's capabilities for
/// agent-to-agent discovery per the A2A protocol spec.
pub async fn agent_card(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = state.config.load();
    let version = env!("CARGO_PKG_VERSION");
    let has_auth = cfg.gateway_auth_token.is_some();

    Json(serde_json::json!({
        "name": "Zenii",
        "description": format!(
            "{} — local AI backend with persistent memory, {} tools, and {} API routes.",
            cfg.identity_description,
            state.tools.len(),
            "114+"
        ),
        "url": format!("http://{}:{}", cfg.gateway_host, cfg.gateway_port),
        "version": version,
        "provider": {
            "organization": "SprklAI",
            "url": "https://zenii.sprklai.com"
        },
        "capabilities": {
            "streaming": true,
            "pushNotifications": true,
            "stateTransitionHistory": false
        },
        "authentication": {
            "schemes": if has_auth { vec!["Bearer"] } else { vec![] },
            "credentials": if has_auth {
                "Bearer token required (set gateway_auth_token in config.toml)"
            } else {
                "None (local mode)"
            }
        },
        "defaultInputModes": ["text/plain", "application/json"],
        "defaultOutputModes": ["text/plain", "application/json"],
        "skills": [
            {
                "id": "memory-store",
                "name": "Store Memory",
                "description": "Persistently store a fact with a unique key. Survives restarts. Searchable via FTS5 + vector embeddings.",
                "tags": ["memory", "persistence", "knowledge"],
                "examples": ["Remember that prod DB is on port 5434", "Store the deploy checklist"]
            },
            {
                "id": "memory-recall",
                "name": "Recall Memory",
                "description": "Search stored memories using natural language. Returns semantically relevant matches.",
                "tags": ["memory", "search", "recall", "semantic"],
                "examples": ["What port is the production database on?", "Find all memories about deploy procedures"]
            },
            {
                "id": "chat",
                "name": "AI Chat with Tools",
                "description": "Send a prompt to Zenii's AI agent. Has access to all tools, persistent memory, and multiple AI providers.",
                "tags": ["chat", "reasoning", "delegation", "multi-provider"],
                "examples": ["Search the web for Rust async patterns and summarize", "What files changed in the last git commit?"]
            },
            {
                "id": "tool-execute",
                "name": "Execute Tool",
                "description": "Run any built-in tool directly: web_search, file_read, file_write, shell, process, patch, system_info, and more.",
                "tags": ["tools", "execution", "automation", "shell", "files"],
                "examples": ["Execute a shell command", "Read a file", "Search the web"]
            },
            {
                "id": "schedule-task",
                "name": "Schedule Autonomous Task",
                "description": "Create cron, interval, or one-time scheduled jobs that execute AI agent turns autonomously.",
                "tags": ["scheduling", "automation", "cron", "recurring"],
                "examples": ["Every morning at 9am, summarize system status", "In 30 minutes, check if the build passed"]
            },
            {
                "id": "channel-message",
                "name": "Send Channel Message",
                "description": "Send messages to Telegram, Slack, or Discord via configured channel integrations.",
                "tags": ["messaging", "channels", "notifications"],
                "examples": ["Send 'deploy complete' to Telegram", "Post build results to Slack"]
            }
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn agent_card_returns_valid_json() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let response = agent_card(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["name"], "Zenii");
        assert!(json["version"].as_str().is_some());
        assert!(json["skills"].as_array().unwrap().len() >= 5);
        assert_eq!(json["capabilities"]["streaming"], true);
    }
}
