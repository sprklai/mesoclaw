use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use crate::MesoError;
use crate::gateway::state::AppState;
use crate::user::UserObservation;

#[derive(Deserialize)]
pub struct ObservationsQuery {
    pub category: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ObservationsListResponse {
    pub observations: Vec<UserObservation>,
}

#[derive(Serialize, Deserialize)]
pub struct AddObservationRequest {
    pub category: String,
    pub key: String,
    pub value: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 {
    0.5
}

#[derive(Serialize, Deserialize)]
pub struct UserProfileResponse {
    pub context: String,
}

/// GET /user/observations — list (optional ?category= filter)
pub async fn list_observations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ObservationsQuery>,
) -> Result<Json<ObservationsListResponse>, MesoError> {
    let observations = state
        .user_learner
        .get_observations(query.category.as_deref())
        .await?;
    Ok(Json(ObservationsListResponse { observations }))
}

/// POST /user/observations — add observation
pub async fn add_observation(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddObservationRequest>,
) -> Result<Json<serde_json::Value>, MesoError> {
    state
        .user_learner
        .observe(&body.category, &body.key, &body.value, body.confidence)
        .await?;
    Ok(Json(serde_json::json!({"status": "observed"})))
}

/// GET /user/observations/{key} — get by key
pub async fn get_observation_by_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<UserObservation>, MesoError> {
    let obs = state.user_learner.get_by_key(&key).await?;
    match obs {
        Some(o) => Ok(Json(o)),
        None => Err(MesoError::NotFound(format!(
            "observation '{key}' not found"
        ))),
    }
}

/// DELETE /user/observations/{key} — delete by key
pub async fn delete_observation(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, MesoError> {
    state.user_learner.forget(&key).await?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// DELETE /user/observations — clear all or by category
pub async fn clear_observations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ObservationsQuery>,
) -> Result<Json<serde_json::Value>, MesoError> {
    if let Some(ref category) = query.category {
        state.user_learner.forget_category(category).await?;
    } else {
        state.user_learner.forget_category("%").await?;
        // Use a direct approach for clearing all
        crate::db::with_db(&state.db, |conn| {
            conn.execute("DELETE FROM user_observations", [])
                .map_err(MesoError::from)?;
            Ok(())
        })
        .await?;
    }
    Ok(Json(serde_json::json!({"status": "cleared"})))
}

/// GET /user/profile — get computed user context string
pub async fn get_user_profile(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserProfileResponse>, MesoError> {
    let context = state.user_learner.build_context().await?;
    Ok(Json(UserProfileResponse { context }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::routes::build_router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    #[tokio::test]
    async fn list_observations() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/user/observations")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ObservationsListResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.observations.is_empty());
    }

    #[tokio::test]
    async fn add_observation() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/user/observations")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&AddObservationRequest {
                    category: "preference".into(),
                    key: "editor".into(),
                    value: "vim".into(),
                    confidence: 0.9,
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_observation_by_key() {
        let (_dir, state) = test_state().await;

        // Add observation directly
        state
            .user_learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();

        let app = build_router(state);

        let req = Request::builder()
            .uri("/user/observations/editor")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: UserObservation = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.value, "vim");
    }

    #[tokio::test]
    async fn delete_observation() {
        let (_dir, state) = test_state().await;

        state
            .user_learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();

        let app = build_router(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/user/observations/editor")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_user_profile() {
        let (_dir, state) = test_state().await;

        state
            .user_learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();

        let app = build_router(state);

        let req = Request::builder()
            .uri("/user/profile")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: UserProfileResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.context.contains("editor: vim"));
    }
}
