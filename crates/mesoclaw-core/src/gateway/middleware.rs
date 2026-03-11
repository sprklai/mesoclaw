use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::MesoError;

/// Bearer token authentication middleware.
///
/// If `auth_token` state is `None`, all requests pass through (no auth configured).
/// If `auth_token` is `Some(token)`, validates the `Authorization: Bearer <token>` header.
/// Skips auth for `GET /health`.
/// For WebSocket endpoints (paths starting with `/ws`), also accepts `?token=<token>` query param.
pub async fn auth_middleware(
    State(auth_token): State<Option<String>>,
    request: Request,
    next: Next,
) -> Result<Response, MesoError> {
    let auth_token = match auth_token {
        Some(token) => token,
        None => return Ok(next.run(request).await),
    };

    // Skip auth for GET /health, /api-docs/*, and GET /setup/status
    if request.method() == axum::http::Method::GET && request.uri().path() == "/health"
        || request.uri().path().starts_with("/api-docs")
        || request.method() == axum::http::Method::GET && request.uri().path() == "/setup/status"
    {
        return Ok(next.run(request).await);
    }

    // Check Authorization header
    if let Some(header_value) = request.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(header_str) = header_value.to_str()
            && let Some(bearer_token) = header_str.strip_prefix("Bearer ")
            && bearer_token == auth_token
        {
            return Ok(next.run(request).await);
        }
        return Err(MesoError::Auth("invalid bearer token".into()));
    }

    // For WS endpoints, check ?token= query param
    if request.uri().path().starts_with("/ws")
        && let Some(query) = request.uri().query()
    {
        for pair in query.split('&') {
            if let Some(value) = pair.strip_prefix("token=") {
                if value == auth_token {
                    return Ok(next.run(request).await);
                }
                return Err(MesoError::Auth("invalid query token".into()));
            }
        }
    }

    Err(MesoError::Auth("missing authorization".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request as HttpRequest, StatusCode};
    use axum::middleware;
    use axum::routing::get;
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn app_with_auth(token: Option<String>) -> Router {
        Router::new()
            .route("/health", get(ok_handler))
            .route("/api/test", get(ok_handler))
            .route("/ws/chat", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                token.clone(),
                auth_middleware,
            ))
            .with_state(token)
    }

    #[tokio::test]
    async fn valid_bearer_token_passes() {
        let app = app_with_auth(Some("secret123".into()));
        let req = HttpRequest::builder()
            .uri("/api/test")
            .header("Authorization", "Bearer secret123")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_token_returns_401() {
        let app = app_with_auth(Some("secret123".into()));
        let req = HttpRequest::builder()
            .uri("/api/test")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_token_returns_401() {
        let app = app_with_auth(Some("secret123".into()));
        let req = HttpRequest::builder()
            .uri("/api/test")
            .header("Authorization", "Bearer wrong_token")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn no_auth_configured_passes_all() {
        let app = app_with_auth(None);
        let req = HttpRequest::builder()
            .uri("/api/test")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_bypasses_auth() {
        let app = app_with_auth(Some("secret123".into()));
        let req = HttpRequest::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ws_token_in_query_param() {
        let app = app_with_auth(Some("secret123".into()));
        let req = HttpRequest::builder()
            .uri("/ws/chat?token=secret123")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
