use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;
use sysinfo::System;

/// GET /system/info -- returns host system information.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/system/info", tag = "System",
    responses((status = 200, description = "System information", body = Object))
))]
pub async fn system_info() -> impl IntoResponse {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    Json(json!({
        "os": System::name().unwrap_or_default(),
        "os_version": System::os_version().unwrap_or_default(),
        "hostname": System::host_name().unwrap_or_default(),
        "cpu_count": sys.cpus().len(),
        "total_memory_bytes": sys.total_memory(),
        "used_memory_bytes": sys.used_memory(),
    }))
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
    async fn system_info_returns_200() {
        let app = Router::new().route("/system/info", get(system_info));
        let req = Request::builder()
            .uri("/system/info")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");

        // All expected fields must be present
        assert!(json.get("os").is_some());
        assert!(json.get("os_version").is_some());
        assert!(json.get("hostname").is_some());
        assert!(json.get("cpu_count").is_some());
        assert!(json.get("total_memory_bytes").is_some());
        assert!(json.get("used_memory_bytes").is_some());

        // total_memory_bytes should be a positive number on any real system
        assert!(json["total_memory_bytes"].as_u64().unwrap_or(0) > 0);
    }
}
