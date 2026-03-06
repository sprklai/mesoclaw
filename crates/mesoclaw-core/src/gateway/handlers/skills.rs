use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use crate::MesoError;
use crate::gateway::state::AppState;
use crate::skills::{Skill, SkillInfo};

#[derive(Deserialize)]
pub struct SkillsQuery {
    pub category: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SkillsListResponse {
    pub skills: Vec<SkillInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub id: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateSkillRequest {
    pub content: String,
}

/// GET /skills — list skills (optional ?category= filter)
pub async fn list_skills(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SkillsQuery>,
) -> Result<Json<SkillsListResponse>, MesoError> {
    let skills = if let Some(ref category) = query.category {
        state.skill_registry.by_category(category).await
    } else {
        state.skill_registry.list().await
    };
    Ok(Json(SkillsListResponse { skills }))
}

/// GET /skills/{id} — get full skill definition
pub async fn get_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Skill>, MesoError> {
    let skill = state.skill_registry.get(&id).await?;
    Ok(Json(skill))
}

/// POST /skills — create user skill
pub async fn create_skill(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateSkillRequest>,
) -> Result<Json<Skill>, MesoError> {
    let skill = state.skill_registry.create(body.id, body.content).await?;
    Ok(Json(skill))
}

/// PUT /skills/{id} — update skill content
pub async fn update_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSkillRequest>,
) -> Result<Json<Skill>, MesoError> {
    let skill = state.skill_registry.update(&id, body.content).await?;
    Ok(Json(skill))
}

/// DELETE /skills/{id} — delete user skill
pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, MesoError> {
    state.skill_registry.delete(&id).await?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// POST /skills/reload — force reload
pub async fn reload_skills(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, MesoError> {
    state.skill_registry.reload().await?;
    Ok(Json(serde_json::json!({"status": "reloaded"})))
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
    async fn list_skills() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/skills")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: SkillsListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.skills.len(), 2);
    }

    #[tokio::test]
    async fn list_skills_by_category() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/skills?category=meta")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: SkillsListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.skills.len(), 1);
        assert_eq!(json.skills[0].id, "system-prompt");
    }

    #[tokio::test]
    async fn get_skill() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/skills/system-prompt")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Skill = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.id, "system-prompt");
    }

    #[tokio::test]
    async fn create_skill() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/skills")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&CreateSkillRequest {
                    id: "test-skill".into(),
                    content: "---\nname: test-skill\ndescription: Test\ncategory: test\n---\nBody."
                        .into(),
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn delete_skill() {
        let (_dir, state) = test_state().await;

        // Create a user skill first
        state
            .skill_registry
            .create(
                "deletable".into(),
                "---\nname: deletable\ndescription: Del\ncategory: test\n---\nBody.".into(),
            )
            .await
            .unwrap();

        let app = build_router(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/skills/deletable")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
