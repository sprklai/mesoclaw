use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ChannelTestResult {
    pub channel: String,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// POST /channels/{name}/test -- test channel credentials by calling the provider API.
/// Works without the channels feature being enabled.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/channels/{name}/test", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    responses((status = 200, description = "Channel test result", body = ChannelTestResult))
))]
pub async fn test_channel_credentials(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> crate::Result<Json<ChannelTestResult>> {
    let result = match name.as_str() {
        "telegram" => test_telegram(&state).await,
        "slack" => test_slack(&state).await,
        "discord" => test_discord(&state).await,
        "matrix" => test_matrix(&state).await,
        _ => ChannelTestResult {
            channel: name.clone(),
            healthy: false,
            error: Some(format!("Unknown channel: {name}")),
            latency_ms: None,
        },
    };
    Ok(Json(result))
}

async fn test_telegram(state: &AppState) -> ChannelTestResult {
    let token = match state.credentials.get("channel:telegram:token").await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return ChannelTestResult {
                channel: "telegram".into(),
                healthy: false,
                error: Some("Bot token not configured".into()),
                latency_ms: None,
            };
        }
        Err(e) => {
            return ChannelTestResult {
                channel: "telegram".into(),
                healthy: false,
                error: Some(format!("Failed to read token: {e}")),
                latency_ms: None,
            };
        }
    };

    let url = format!("https://api.telegram.org/bot{token}/getMe");
    let start = std::time::Instant::now();
    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => ChannelTestResult {
            channel: "telegram".into(),
            healthy: true,
            error: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            ChannelTestResult {
                channel: "telegram".into(),
                healthy: false,
                error: Some(format!("HTTP {status}: {body}")),
                latency_ms: Some(start.elapsed().as_millis() as u64),
            }
        }
        Err(e) => ChannelTestResult {
            channel: "telegram".into(),
            healthy: false,
            error: Some(format!("Request failed: {e}")),
            latency_ms: None,
        },
    }
}

async fn test_slack(state: &AppState) -> ChannelTestResult {
    let token = match state.credentials.get("channel:slack:bot_token").await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return ChannelTestResult {
                channel: "slack".into(),
                healthy: false,
                error: Some("Bot token not configured".into()),
                latency_ms: None,
            };
        }
        Err(e) => {
            return ChannelTestResult {
                channel: "slack".into(),
                healthy: false,
                error: Some(format!("Failed to read token: {e}")),
                latency_ms: None,
            };
        }
    };

    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    match client
        .get("https://slack.com/api/auth.test")
        .bearer_auth(&token)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if body["ok"].as_bool() == Some(true) {
                ChannelTestResult {
                    channel: "slack".into(),
                    healthy: true,
                    error: None,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                }
            } else {
                ChannelTestResult {
                    channel: "slack".into(),
                    healthy: false,
                    error: Some(
                        body["error"]
                            .as_str()
                            .unwrap_or("Unknown error")
                            .to_string(),
                    ),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                }
            }
        }
        Ok(resp) => ChannelTestResult {
            channel: "slack".into(),
            healthy: false,
            error: Some(format!("HTTP {}", resp.status())),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => ChannelTestResult {
            channel: "slack".into(),
            healthy: false,
            error: Some(format!("Request failed: {e}")),
            latency_ms: None,
        },
    }
}

async fn test_discord(state: &AppState) -> ChannelTestResult {
    let token = match state.credentials.get("channel:discord:token").await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return ChannelTestResult {
                channel: "discord".into(),
                healthy: false,
                error: Some("Bot token not configured".into()),
                latency_ms: None,
            };
        }
        Err(e) => {
            return ChannelTestResult {
                channel: "discord".into(),
                healthy: false,
                error: Some(format!("Failed to read token: {e}")),
                latency_ms: None,
            };
        }
    };

    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    match client
        .get("https://discord.com/api/v10/users/@me")
        .header("Authorization", format!("Bot {token}"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => ChannelTestResult {
            channel: "discord".into(),
            healthy: true,
            error: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            ChannelTestResult {
                channel: "discord".into(),
                healthy: false,
                error: Some(format!("HTTP {status}: {body}")),
                latency_ms: Some(start.elapsed().as_millis() as u64),
            }
        }
        Err(e) => ChannelTestResult {
            channel: "discord".into(),
            healthy: false,
            error: Some(format!("Request failed: {e}")),
            latency_ms: None,
        },
    }
}

async fn test_matrix(state: &AppState) -> ChannelTestResult {
    let homeserver = match state.credentials.get("channel:matrix:homeserver_url").await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return ChannelTestResult {
                channel: "matrix".into(),
                healthy: false,
                error: Some("Homeserver URL not configured".into()),
                latency_ms: None,
            };
        }
        Err(e) => {
            return ChannelTestResult {
                channel: "matrix".into(),
                healthy: false,
                error: Some(format!("Failed to read homeserver URL: {e}")),
                latency_ms: None,
            };
        }
    };

    let token = match state.credentials.get("channel:matrix:access_token").await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return ChannelTestResult {
                channel: "matrix".into(),
                healthy: false,
                error: Some("Access token not configured".into()),
                latency_ms: None,
            };
        }
        Err(e) => {
            return ChannelTestResult {
                channel: "matrix".into(),
                healthy: false,
                error: Some(format!("Failed to read access token: {e}")),
                latency_ms: None,
            };
        }
    };

    let url = format!(
        "{}/_matrix/client/v3/account/whoami",
        homeserver.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    match client.get(&url).bearer_auth(&token).send().await {
        Ok(resp) if resp.status().is_success() => ChannelTestResult {
            channel: "matrix".into(),
            healthy: true,
            error: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            ChannelTestResult {
                channel: "matrix".into(),
                healthy: false,
                error: Some(format!("HTTP {status}: {body}")),
                latency_ms: Some(start.elapsed().as_millis() as u64),
            }
        }
        Err(e) => ChannelTestResult {
            channel: "matrix".into(),
            healthy: false,
            error: Some(format!("Request failed: {e}")),
            latency_ms: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    #[tokio::test]
    async fn test_unknown_channel() {
        let (_dir, state) = test_state().await;
        let app = Router::new()
            .route("/channels/{name}/test", post(test_channel_credentials))
            .with_state(state);

        let req = Request::post("/channels/unknown/test")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: ChannelTestResult = serde_json::from_slice(&body).unwrap();
        assert!(!result.healthy);
        assert!(result.error.unwrap().contains("Unknown channel"));
    }

    #[tokio::test]
    async fn test_telegram_no_token() {
        let (_dir, state) = test_state().await;
        let app = Router::new()
            .route("/channels/{name}/test", post(test_channel_credentials))
            .with_state(state);

        let req = Request::post("/channels/telegram/test")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: ChannelTestResult = serde_json::from_slice(&body).unwrap();
        assert!(!result.healthy);
        assert!(result.error.unwrap().contains("not configured"));
    }
}
