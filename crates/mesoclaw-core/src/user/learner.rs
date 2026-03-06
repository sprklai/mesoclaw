use crate::config::AppConfig;
use crate::db::{self, DbPool};
use crate::{MesoError, Result};

use super::types::UserObservation;

/// Manages user observations for progressive learning.
pub struct UserLearner {
    db: DbPool,
    learning_enabled: bool,
    denied_categories: Vec<String>,
    max_observations: usize,
    ttl_days: u32,
    min_confidence: f32,
}

impl UserLearner {
    pub fn new(db: DbPool, config: &AppConfig) -> Self {
        Self {
            db,
            learning_enabled: config.learning_enabled,
            denied_categories: config.learning_denied_categories.clone(),
            max_observations: config.learning_max_observations,
            ttl_days: config.learning_observation_ttl_days,
            min_confidence: config.learning_min_confidence,
        }
    }

    /// Add or update an observation.
    pub async fn observe(
        &self,
        category: &str,
        key: &str,
        value: &str,
        confidence: f32,
    ) -> Result<()> {
        if !self.learning_enabled {
            return Err(MesoError::User("learning is disabled".into()));
        }

        if self.denied_categories.iter().any(|c| c == category) {
            return Err(MesoError::User(format!(
                "category '{category}' is denied by configuration"
            )));
        }

        // Check max observations
        let current_count = self.count().await?;
        if current_count >= self.max_observations {
            return Err(MesoError::User(format!(
                "max observations reached ({}/{})",
                current_count, self.max_observations
            )));
        }

        let category = category.to_string();
        let key = key.to_string();
        let value = value.to_string();
        let id = uuid::Uuid::new_v4().to_string();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO user_observations (id, category, key, value, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    confidence = excluded.confidence,
                    updated_at = datetime('now')",
                rusqlite::params![id, category, key, value, confidence],
            )
            .map_err(MesoError::from)?;
            Ok(())
        })
        .await
    }

    /// Get all observations, optionally filtered by category.
    pub async fn get_observations(&self, category: Option<&str>) -> Result<Vec<UserObservation>> {
        let category = category.map(|s| s.to_string());
        db::with_db(&self.db, move |conn| {
            let mut stmt;
            let rows: Vec<UserObservation> = if let Some(ref cat) = category {
                stmt = conn
                    .prepare(
                        "SELECT id, category, key, value, confidence, created_at, updated_at
                         FROM user_observations WHERE category = ?1
                         ORDER BY updated_at DESC",
                    )
                    .map_err(MesoError::from)?;
                stmt.query_map(rusqlite::params![cat], map_row)
                    .map_err(MesoError::from)?
                    .filter_map(|r| r.ok())
                    .collect()
            } else {
                stmt = conn
                    .prepare(
                        "SELECT id, category, key, value, confidence, created_at, updated_at
                         FROM user_observations ORDER BY updated_at DESC",
                    )
                    .map_err(MesoError::from)?;
                stmt.query_map([], map_row)
                    .map_err(MesoError::from)?
                    .filter_map(|r| r.ok())
                    .collect()
            };
            Ok(rows)
        })
        .await
    }

    /// Get a specific observation by key.
    pub async fn get_by_key(&self, key: &str) -> Result<Option<UserObservation>> {
        let key = key.to_string();
        db::with_db(&self.db, move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, category, key, value, confidence, created_at, updated_at
                     FROM user_observations WHERE key = ?1",
                )
                .map_err(MesoError::from)?;
            let obs = stmt
                .query_map(rusqlite::params![key], map_row)
                .map_err(MesoError::from)?
                .filter_map(|r| r.ok())
                .next();
            Ok(obs)
        })
        .await
    }

    /// Delete an observation by key.
    pub async fn forget(&self, key: &str) -> Result<()> {
        let key = key.to_string();
        db::with_db(&self.db, move |conn| {
            conn.execute(
                "DELETE FROM user_observations WHERE key = ?1",
                rusqlite::params![key],
            )
            .map_err(MesoError::from)?;
            Ok(())
        })
        .await
    }

    /// Delete all observations in a category.
    pub async fn forget_category(&self, category: &str) -> Result<()> {
        let category = category.to_string();
        db::with_db(&self.db, move |conn| {
            conn.execute(
                "DELETE FROM user_observations WHERE category = ?1",
                rusqlite::params![category],
            )
            .map_err(MesoError::from)?;
            Ok(())
        })
        .await
    }

    /// Remove observations older than the configured TTL.
    pub async fn prune_expired(&self) -> Result<usize> {
        let ttl_days = self.ttl_days;
        db::with_db(&self.db, move |conn| {
            let deleted = conn
                .execute(
                    "DELETE FROM user_observations
                     WHERE updated_at < datetime('now', ?1)",
                    rusqlite::params![format!("-{ttl_days} days")],
                )
                .map_err(MesoError::from)?;
            Ok(deleted)
        })
        .await
    }

    /// Count total observations.
    pub async fn count(&self) -> Result<usize> {
        db::with_db(&self.db, |conn| {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM user_observations", [], |r| r.get(0))
                .map_err(MesoError::from)?;
            Ok(count as usize)
        })
        .await
    }

    /// Build a formatted context string from observations for prompt composition.
    pub async fn build_context(&self) -> Result<String> {
        let min_confidence = self.min_confidence;
        db::with_db(&self.db, move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, category, key, value, confidence, created_at, updated_at
                     FROM user_observations
                     WHERE confidence >= ?1
                     ORDER BY category, key",
                )
                .map_err(MesoError::from)?;
            let observations: Vec<UserObservation> = stmt
                .query_map(rusqlite::params![min_confidence], map_row)
                .map_err(MesoError::from)?
                .filter_map(|r| r.ok())
                .collect();

            if observations.is_empty() {
                return Ok(String::new());
            }

            let mut lines = Vec::new();
            for obs in &observations {
                lines.push(format!(
                    "- {}: {} (confidence: {:.1})",
                    obs.key, obs.value, obs.confidence
                ));
            }
            Ok(lines.join("\n"))
        })
        .await
    }
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<UserObservation> {
    Ok(UserObservation {
        id: row.get(0)?,
        category: row.get(1)?,
        key: row.get(2)?,
        value: row.get(3)?,
        confidence: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, UserLearner) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig::default();
        let learner = UserLearner::new(pool, &config);
        (dir, learner)
    }

    #[tokio::test]
    async fn observe_inserts_observation() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        let count = learner.count().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn observe_upserts_existing_key() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        learner
            .observe("preference", "editor", "neovim", 0.95)
            .await
            .unwrap();
        let count = learner.count().await.unwrap();
        assert_eq!(count, 1);
        let obs = learner.get_by_key("editor").await.unwrap().unwrap();
        assert_eq!(obs.value, "neovim");
    }

    #[tokio::test]
    async fn observe_respects_denied_categories() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig {
            learning_denied_categories: vec!["personal".into()],
            ..Default::default()
        };
        let learner = UserLearner::new(pool, &config);
        let result = learner.observe("personal", "age", "30", 0.9).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn observe_respects_learning_disabled() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig {
            learning_enabled: false,
            ..Default::default()
        };
        let learner = UserLearner::new(pool, &config);
        let result = learner.observe("preference", "editor", "vim", 0.9).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_observations_returns_all() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        learner
            .observe("workflow", "shell", "bash", 0.8)
            .await
            .unwrap();

        let all = learner.get_observations(None).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn get_observations_by_category() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        learner
            .observe("workflow", "shell", "bash", 0.8)
            .await
            .unwrap();

        let prefs = learner.get_observations(Some("preference")).await.unwrap();
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0].key, "editor");
    }

    #[tokio::test]
    async fn get_by_key_found() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        let obs = learner.get_by_key("editor").await.unwrap();
        assert!(obs.is_some());
        assert_eq!(obs.unwrap().value, "vim");
    }

    #[tokio::test]
    async fn get_by_key_not_found() {
        let (_dir, learner) = setup().await;
        let obs = learner.get_by_key("nonexistent").await.unwrap();
        assert!(obs.is_none());
    }

    #[tokio::test]
    async fn forget_deletes_by_key() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        learner.forget("editor").await.unwrap();
        let count = learner.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn forget_category_deletes_all() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        learner
            .observe("preference", "theme", "dark", 0.8)
            .await
            .unwrap();
        learner
            .observe("workflow", "shell", "bash", 0.7)
            .await
            .unwrap();

        learner.forget_category("preference").await.unwrap();
        let count = learner.count().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn prune_expired_removes_old() {
        let (_dir, learner) = setup().await;
        // Insert with an old timestamp
        db::with_db(&learner.db, |conn| {
            conn.execute(
                "INSERT INTO user_observations (id, category, key, value, confidence, updated_at)
                 VALUES ('old', 'preference', 'old-key', 'old-val', 0.5, datetime('now', '-400 days'))",
                [],
            )
            .map_err(MesoError::from)?;
            Ok(())
        })
        .await
        .unwrap();

        let deleted = learner.prune_expired().await.unwrap();
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn prune_expired_keeps_fresh() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        let deleted = learner.prune_expired().await.unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(learner.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn count_returns_correct() {
        let (_dir, learner) = setup().await;
        assert_eq!(learner.count().await.unwrap(), 0);
        learner.observe("preference", "a", "1", 0.5).await.unwrap();
        learner.observe("preference", "b", "2", 0.5).await.unwrap();
        assert_eq!(learner.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn build_context_formats_observations() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "editor", "vim", 0.9)
            .await
            .unwrap();
        learner
            .observe("workflow", "shell", "bash", 0.8)
            .await
            .unwrap();

        let context = learner.build_context().await.unwrap();
        assert!(context.contains("editor: vim"));
        assert!(context.contains("shell: bash"));
    }

    #[tokio::test]
    async fn build_context_respects_min_confidence() {
        let (_dir, learner) = setup().await;
        learner
            .observe("preference", "high", "yes", 0.9)
            .await
            .unwrap();
        learner
            .observe("preference", "low", "no", 0.1)
            .await
            .unwrap();

        let context = learner.build_context().await.unwrap();
        assert!(context.contains("high: yes"));
        assert!(!context.contains("low: no"));
    }
}
