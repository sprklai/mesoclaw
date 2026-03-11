use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;

/// GET /health -- returns 200 {"status": "ok"}
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/health", tag = "System",
    security(()),
    responses((status = 200, description = "Health check", body = Object))
))]
pub async fn health() -> impl IntoResponse {
    Json(json!({"status": "ok"}))
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_returns_200() {
        let app = Router::new().route("/health", get(health));
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 1024)
            .await
            .expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn health_no_auth_required() {
        // Health endpoint works without any auth headers or state
        let app = Router::new().route("/health", get(health));
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
