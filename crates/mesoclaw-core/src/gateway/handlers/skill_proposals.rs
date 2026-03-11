use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::db;
use crate::gateway::state::AppState;
use crate::{MesoError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SkillProposal {
    pub id: String,
    pub action: String,
    pub skill_name: String,
    pub content: Option<String>,
    pub rationale: String,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

/// GET /skills/proposals — list pending proposals
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/skills/proposals", tag = "Skill Proposals",
    responses((status = 200, description = "List of pending skill proposals", body = Vec<SkillProposal>))
))]
pub async fn list_proposals(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse> {
    let proposals = db::with_db(&state.db, |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, action, skill_name, content, rationale, status, created_at, resolved_at
             FROM skill_proposals
             WHERE status = 'pending'
             ORDER BY created_at DESC",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(SkillProposal {
                    id: row.get(0)?,
                    action: row.get(1)?,
                    skill_name: row.get(2)?,
                    content: row.get(3)?,
                    rationale: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    resolved_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    })
    .await?;

    Ok(Json(proposals))
}

/// POST /skills/proposals/:id/approve — approve and execute a proposal
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/skills/proposals/{id}/approve", tag = "Skill Proposals",
    params(("id" = String, Path, description = "Proposal ID")),
    responses(
        (status = 200, description = "Proposal approved and executed"),
        (status = 404, description = "Proposal not found"),
        (status = 400, description = "Proposal not in pending state")
    )
))]
pub async fn approve_proposal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    // Get the proposal
    let proposal_id = id.clone();
    let proposal = db::with_db(&state.db, move |conn| {
        conn.query_row(
            "SELECT id, action, skill_name, content, rationale, status, created_at, resolved_at
             FROM skill_proposals WHERE id = ?1",
            rusqlite::params![proposal_id],
            |row| {
                Ok(SkillProposal {
                    id: row.get(0)?,
                    action: row.get(1)?,
                    skill_name: row.get(2)?,
                    content: row.get(3)?,
                    rationale: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    resolved_at: row.get(7)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                MesoError::NotFound(format!("proposal not found: {proposal_id}"))
            }
            other => MesoError::Sqlite(other),
        })
    })
    .await?;

    if proposal.status != "pending" {
        return Err(MesoError::Validation(format!(
            "proposal is already '{}'",
            proposal.status
        )));
    }

    // Execute the action
    match proposal.action.as_str() {
        "create" => {
            if let Some(ref content) = proposal.content {
                state
                    .skill_registry
                    .create(proposal.skill_name.clone(), content.clone())
                    .await?;
            }
        }
        "update" => {
            if let Some(ref content) = proposal.content {
                state
                    .skill_registry
                    .update(&proposal.skill_name, content.clone())
                    .await?;
            }
        }
        "delete" => {
            state.skill_registry.delete(&proposal.skill_name).await?;
        }
        _ => {}
    }

    // Mark as approved
    let approve_id = id.clone();
    db::with_db(&state.db, move |conn| {
        conn.execute(
            "UPDATE skill_proposals SET status = 'approved', resolved_at = datetime('now')
             WHERE id = ?1",
            rusqlite::params![approve_id],
        )
        .map_err(crate::MesoError::from)?;
        Ok(())
    })
    .await?;

    Ok(Json(serde_json::json!({"status": "approved", "id": id})))
}

/// POST /skills/proposals/:id/reject — reject a proposal
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/skills/proposals/{id}/reject", tag = "Skill Proposals",
    params(("id" = String, Path, description = "Proposal ID")),
    responses(
        (status = 200, description = "Proposal rejected"),
        (status = 404, description = "Pending proposal not found")
    )
))]
pub async fn reject_proposal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let reject_id = id.clone();
    let rows = db::with_db(&state.db, move |conn| {
        let rows = conn
            .execute(
                "UPDATE skill_proposals SET status = 'rejected', resolved_at = datetime('now')
                 WHERE id = ?1 AND status = 'pending'",
                rusqlite::params![reject_id],
            )
            .map_err(crate::MesoError::from)?;
        Ok(rows)
    })
    .await?;

    if rows == 0 {
        return Err(MesoError::NotFound(format!(
            "pending proposal not found: {id}"
        )));
    }

    Ok(Json(serde_json::json!({"status": "rejected", "id": id})))
}

/// DELETE /skills/proposals/:id — delete a proposal
#[cfg_attr(feature = "api-docs", utoipa::path(
    delete, path = "/skills/proposals/{id}", tag = "Skill Proposals",
    params(("id" = String, Path, description = "Proposal ID")),
    responses(
        (status = 200, description = "Proposal deleted"),
        (status = 404, description = "Proposal not found")
    )
))]
pub async fn delete_proposal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let delete_id = id.clone();
    let rows = db::with_db(&state.db, move |conn| {
        let rows = conn
            .execute(
                "DELETE FROM skill_proposals WHERE id = ?1",
                rusqlite::params![delete_id],
            )
            .map_err(crate::MesoError::from)?;
        Ok(rows)
    })
    .await?;

    if rows == 0 {
        return Err(MesoError::NotFound(format!("proposal not found: {id}")));
    }

    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{delete, get, post};
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/skills/proposals", get(list_proposals))
            .route("/skills/proposals/{id}/approve", post(approve_proposal))
            .route("/skills/proposals/{id}/reject", post(reject_proposal))
            .route("/skills/proposals/{id}", delete(delete_proposal))
            .with_state(state)
    }

    async fn insert_proposal(state: &AppState, id: &str, action: &str, skill_name: &str) {
        let id = id.to_string();
        let action = action.to_string();
        let skill_name = skill_name.to_string();
        db::with_db(&state.db, move |conn| {
            conn.execute(
                "INSERT INTO skill_proposals (id, action, skill_name, content, rationale)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    id,
                    action,
                    skill_name,
                    Some("# Test Skill\nContent"),
                    "Test rationale"
                ],
            )
            .map_err(crate::MesoError::from)?;
            Ok(())
        })
        .await
        .unwrap();
    }

    // 15.3.23 — list proposals returns pending
    #[tokio::test]
    async fn list_proposals_returns_pending() {
        let (_dir, state) = test_state().await;
        insert_proposal(&state, "p1", "create", "greeting").await;

        let req = Request::builder()
            .uri("/skills/proposals")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let proposals: Vec<SkillProposal> = serde_json::from_slice(&body).unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].skill_name, "greeting");
        assert_eq!(proposals[0].status, "pending");
    }

    // 15.3.24 — list proposals empty returns empty array
    #[tokio::test]
    async fn list_proposals_empty_returns_empty_array() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/skills/proposals")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let proposals: Vec<SkillProposal> = serde_json::from_slice(&body).unwrap();
        assert!(proposals.is_empty());
    }

    // 15.3.25 — approve proposal executes create
    #[tokio::test]
    async fn approve_proposal_executes_create() {
        let (_dir, state) = test_state().await;
        insert_proposal(&state, "p-create", "create", "test-skill").await;

        let req = Request::builder()
            .method("POST")
            .uri("/skills/proposals/p-create/approve")
            .body(Body::empty())
            .unwrap();

        let resp = app(state.clone()).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify skill was created
        let skill = state.skill_registry.get("test-skill").await;
        assert!(skill.is_ok());
    }

    // 15.3.26 — approve proposal executes delete
    #[tokio::test]
    async fn approve_proposal_executes_delete() {
        let (_dir, state) = test_state().await;

        // Create a skill first, then propose deletion
        state
            .skill_registry
            .create("to-delete".into(), "# To Delete\nContent".into())
            .await
            .unwrap();

        insert_proposal(&state, "p-delete", "delete", "to-delete").await;

        let req = Request::builder()
            .method("POST")
            .uri("/skills/proposals/p-delete/approve")
            .body(Body::empty())
            .unwrap();

        let resp = app(state.clone()).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify skill was deleted
        let skill = state.skill_registry.get("to-delete").await;
        assert!(skill.is_err());
    }

    // 15.3.27 — reject proposal updates status
    #[tokio::test]
    async fn reject_proposal_updates_status() {
        let (_dir, state) = test_state().await;
        insert_proposal(&state, "p-reject", "create", "rejected-skill").await;

        let req = Request::builder()
            .method("POST")
            .uri("/skills/proposals/p-reject/reject")
            .body(Body::empty())
            .unwrap();

        let resp = app(state.clone()).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify status changed — should not appear in pending list
        let proposals = db::with_db(&state.db, |conn| {
            let status: String = conn
                .query_row(
                    "SELECT status FROM skill_proposals WHERE id = 'p-reject'",
                    [],
                    |row| row.get(0),
                )
                .map_err(crate::MesoError::from)?;
            Ok(status)
        })
        .await
        .unwrap();
        assert_eq!(proposals, "rejected");
    }

    // 15.3.28 — approve nonexistent returns 404
    #[tokio::test]
    async fn approve_nonexistent_returns_404() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/skills/proposals/nonexistent/approve")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // 15.3.29 — delete proposal removes from db
    #[tokio::test]
    async fn delete_proposal_removes_from_db() {
        let (_dir, state) = test_state().await;
        insert_proposal(&state, "p-del", "create", "del-skill").await;

        let req = Request::builder()
            .method("DELETE")
            .uri("/skills/proposals/p-del")
            .body(Body::empty())
            .unwrap();

        let resp = app(state.clone()).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify removed
        let count = db::with_db(&state.db, |conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM skill_proposals WHERE id = 'p-del'",
                    [],
                    |row| row.get(0),
                )
                .map_err(crate::MesoError::from)?;
            Ok(count)
        })
        .await
        .unwrap();
        assert_eq!(count, 0);
    }
}
