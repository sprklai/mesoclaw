use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct CreateProviderRequest {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: bool,
    #[serde(default)]
    pub models: Vec<CreateModelEntry>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct CreateModelEntry {
    pub model_id: String,
    pub display_name: String,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct UpdateProviderRequest {
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct AddModelRequest {
    pub model_id: String,
    pub display_name: String,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SetDefaultModelRequest {
    pub provider_id: String,
    pub model_id: String,
}

/// GET /providers -- list all providers with models.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/providers", tag = "Providers",
    responses((status = 200, description = "List of providers with models", body = Vec<Object>))
))]
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> crate::Result<impl IntoResponse> {
    let providers = state.provider_registry.list_providers().await?;
    Ok(Json(providers))
}

/// GET /providers/with-key-status -- list providers with has_api_key boolean.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/providers/with-key-status", tag = "Providers",
    responses((status = 200, description = "Providers with API key status", body = Vec<Object>))
))]
pub async fn list_with_key_status(
    State(state): State<Arc<AppState>>,
) -> crate::Result<impl IntoResponse> {
    let providers = state
        .provider_registry
        .list_providers_with_key_status(state.credentials.as_ref())
        .await?;
    Ok(Json(providers))
}

/// GET /providers/default -- get global default model.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/providers/default", tag = "Providers",
    responses((status = 200, description = "Default model", body = Object))
))]
pub async fn get_default_model(
    State(state): State<Arc<AppState>>,
) -> crate::Result<impl IntoResponse> {
    let default = state.provider_registry.get_default_model().await?;
    match default {
        Some((provider_id, model_id)) => Ok(Json(serde_json::json!({
            "provider_id": provider_id,
            "model_id": model_id,
        }))),
        None => Ok(Json(serde_json::json!(null))),
    }
}

/// PUT /providers/default -- set global default model.
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/providers/default", tag = "Providers",
    request_body = SetDefaultModelRequest,
    responses((status = 200, description = "Default model set", body = Object))
))]
pub async fn set_default_model(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetDefaultModelRequest>,
) -> crate::Result<impl IntoResponse> {
    state
        .provider_registry
        .set_default_model(&req.provider_id, &req.model_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /providers/{id} -- get a specific provider by ID.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/providers/{id}", tag = "Providers",
    params(("id" = String, Path, description = "Provider ID")),
    responses((status = 200, description = "Provider details", body = Object))
))]
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let provider = state.provider_registry.get_provider(&id).await?;
    Ok(Json(provider))
}

/// POST /providers -- create a user-defined provider.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/providers", tag = "Providers",
    request_body = CreateProviderRequest,
    responses((status = 200, description = "Provider created", body = Object))
))]
pub async fn create_user_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProviderRequest>,
) -> crate::Result<impl IntoResponse> {
    let models: Vec<(String, String, bool)> = req
        .models
        .iter()
        .map(|m| (m.model_id.clone(), m.display_name.clone(), m.supports_tools))
        .collect();

    state
        .provider_registry
        .add_user_provider(
            &req.id,
            &req.name,
            &req.base_url,
            req.requires_api_key,
            &models,
        )
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// PUT /providers/{id} -- update provider base_url.
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/providers/{id}", tag = "Providers",
    params(("id" = String, Path, description = "Provider ID")),
    request_body = UpdateProviderRequest,
    responses((status = 200, description = "Provider updated", body = Object))
))]
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> crate::Result<impl IntoResponse> {
    state
        .provider_registry
        .update_provider(&id, &req.base_url)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /providers/{id} -- delete a user-defined provider.
#[cfg_attr(feature = "api-docs", utoipa::path(
    delete, path = "/providers/{id}", tag = "Providers",
    params(("id" = String, Path, description = "Provider ID")),
    responses((status = 200, description = "Provider deleted", body = Object))
))]
pub async fn delete_user_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> crate::Result<impl IntoResponse> {
    state.provider_registry.delete_user_provider(&id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /providers/{id}/models -- add a custom model.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/providers/{id}/models", tag = "Providers",
    params(("id" = String, Path, description = "Provider ID")),
    request_body = AddModelRequest,
    responses((status = 200, description = "Model added", body = Object))
))]
pub async fn add_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<AddModelRequest>,
) -> crate::Result<impl IntoResponse> {
    state
        .provider_registry
        .add_custom_model(
            &provider_id,
            &req.model_id,
            &req.display_name,
            req.supports_tools,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /providers/{id}/test -- test connection to a provider.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/providers/{id}/test", tag = "Providers",
    params(("id" = String, Path, description = "Provider ID")),
    responses((status = 200, description = "Connection test result", body = Object))
))]
pub async fn test_connection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let provider_with_models = state.provider_registry.get_provider(&id).await?;
    let provider = &provider_with_models.provider;

    let api_key = crate::ai::providers::resolve_api_key_for_provider(
        &id,
        provider.requires_api_key,
        state.credentials.as_ref(),
    )
    .await?;

    let url = format!("{}/models", provider.base_url.trim_end_matches('/'));

    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| crate::MesoError::Agent(format!("HTTP client error: {e}")))?;

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match resp {
        Ok(r) if r.status().is_success() => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Connected successfully",
            "latency_ms": latency_ms,
        }))),
        Ok(r) => {
            let status = r.status().as_u16();
            let body = r.text().await.unwrap_or_default();
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("HTTP {status}: {body}"),
                "latency_ms": latency_ms,
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "message": format!("Connection failed: {e}"),
            "latency_ms": latency_ms,
        }))),
    }
}

/// DELETE /providers/{id}/models/{model_id} -- delete a custom model.
#[cfg_attr(feature = "api-docs", utoipa::path(
    delete, path = "/providers/{id}/models/{model_id}", tag = "Providers",
    params(
        ("id" = String, Path, description = "Provider ID"),
        ("model_id" = String, Path, description = "Model ID"),
    ),
    responses((status = 200, description = "Model deleted", body = Object))
))]
pub async fn delete_model(
    State(state): State<Arc<AppState>>,
    Path((provider_id, model_id)): Path<(String, String)>,
) -> crate::Result<impl IntoResponse> {
    let composite_id = format!("{provider_id}:{model_id}");
    state
        .provider_registry
        .delete_custom_model(&composite_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{delete, get, post};
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/providers", get(list_providers).post(create_user_provider))
            .route("/providers/with-key-status", get(list_with_key_status))
            .route(
                "/providers/default",
                get(get_default_model).put(set_default_model),
            )
            .route(
                "/providers/{id}",
                get(get_provider)
                    .put(update_provider)
                    .delete(delete_user_provider),
            )
            .route("/providers/{id}/models", post(add_model))
            .route("/providers/{id}/models/{model_id}", delete(delete_model))
            .with_state(state)
    }

    #[tokio::test]
    async fn list_providers_test() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/providers")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 16384).await.unwrap();
        let providers: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(providers.len() >= 6);
    }

    #[tokio::test]
    async fn list_with_key_status_test() {
        let (_dir, state) = test_state().await;
        state
            .credentials
            .set("api_key:openai", "sk-test")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri("/providers/with-key-status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 16384).await.unwrap();
        let providers: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        let openai = providers.iter().find(|p| p["id"] == "openai").unwrap();
        assert_eq!(openai["has_api_key"], true);
    }

    #[tokio::test]
    async fn get_provider_test() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/providers/openai")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 16384).await.unwrap();
        let provider: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(provider["name"], "OpenAI");
    }

    #[tokio::test]
    async fn create_user_provider_test() {
        let (_dir, state) = test_state().await;
        let app = app(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/providers")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&CreateProviderRequest {
                    id: "my-api".into(),
                    name: "My API".into(),
                    base_url: "https://my.api/v1".into(),
                    requires_api_key: true,
                    models: vec![CreateModelEntry {
                        model_id: "model-1".into(),
                        display_name: "Model 1".into(),
                        supports_tools: true,
                    }],
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify it was created
        let p = state
            .provider_registry
            .get_provider("my-api")
            .await
            .unwrap();
        assert_eq!(p.provider.name, "My API");
        assert!(p.provider.is_user_defined);
    }

    #[tokio::test]
    async fn delete_user_provider_test() {
        let (_dir, state) = test_state().await;
        state
            .provider_registry
            .add_user_provider("temp", "Temp", "http://temp", false, &[])
            .await
            .unwrap();

        let app = app(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri("/providers/temp")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn delete_builtin_rejected() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/providers/openai")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        // Should return a validation error (400)
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn set_default_model_test() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("PUT")
            .uri("/providers/default")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&SetDefaultModelRequest {
                    provider_id: "openai".into(),
                    model_id: "gpt-4o".into(),
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_default_model_test() {
        let (_dir, state) = test_state().await;
        state
            .provider_registry
            .set_default_model("openai", "gpt-4o")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri("/providers/default")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["provider_id"], "openai");
        assert_eq!(result["model_id"], "gpt-4o");
    }
}
