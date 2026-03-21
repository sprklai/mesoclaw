use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::Result;
use crate::db::{self, DbPool};

/// Decision made by the user for a tool approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    ApproveAlways,
    Deny,
}

impl ApprovalDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::ApproveAlways => "approve_always",
            Self::Deny => "deny",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approve" => Self::Approve,
            "approve_always" => Self::ApproveAlways,
            "deny" => Self::Deny,
            _ => Self::Deny,
        }
    }
}

/// A request for user approval of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub approval_id: String,
    pub call_id: String,
    pub tool_name: String,
    pub args_summary: String,
    pub risk_level: String,
    pub reason: String,
    pub timeout_secs: u64,
}

/// A persistent approval rule saved to the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub id: String,
    pub tool_name: String,
    pub pattern: Option<String>,
    pub decision: String,
    pub surface: String,
    pub created_at: String,
}

/// Orchestrates tool approval: manages pending requests, session cache, and persistent rules.
pub struct ApprovalBroker {
    /// Pending approval requests waiting for a response.
    /// Key: approval_id, Value: oneshot sender for the decision.
    pending: DashMap<String, oneshot::Sender<ApprovalDecision>>,
    /// Session-scoped cache: key = "tool_name" or "tool_name:command_pattern", value = decision.
    session_cache: DashMap<String, ApprovalDecision>,
    db: DbPool,
}

impl ApprovalBroker {
    pub fn new(db: DbPool) -> Self {
        Self {
            pending: DashMap::new(),
            session_cache: DashMap::new(),
            db,
        }
    }

    /// Check if a tool call is pre-approved (session cache or persistent rule).
    /// Returns `Some(decision)` if found, `None` if the user must be prompted.
    pub async fn pre_check(
        &self,
        tool_name: &str,
        args_summary: &str,
        surface: &str,
    ) -> Option<ApprovalDecision> {
        // 1. Check session cache (exact tool name)
        if let Some(decision) = self.session_cache.get(tool_name) {
            debug!("approval: session cache hit for tool={tool_name}");
            return Some(*decision);
        }

        // 2. Check session cache (tool_name:pattern for shell commands)
        let cache_key = format!("{tool_name}:{args_summary}");
        if let Some(decision) = self.session_cache.get(&cache_key) {
            debug!("approval: session cache hit for {cache_key}");
            return Some(*decision);
        }

        // 3. Check persistent rules in DB
        match self
            .find_matching_rule(tool_name, args_summary, surface)
            .await
        {
            Ok(Some(rule)) => {
                let decision = if rule.decision == "approve" {
                    ApprovalDecision::Approve
                } else {
                    ApprovalDecision::Deny
                };
                debug!(
                    "approval: DB rule match for tool={tool_name}, rule_id={}",
                    rule.id
                );
                Some(decision)
            }
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("approval: DB rule lookup failed: {e}");
                None
            }
        }
    }

    /// Register a pending approval request. Returns a receiver to await the decision.
    pub fn register(&self, approval_id: &str) -> oneshot::Receiver<ApprovalDecision> {
        let (tx, rx) = oneshot::channel();
        self.pending.insert(approval_id.to_string(), tx);
        rx
    }

    /// Resolve a pending approval request with the user's decision.
    /// Returns `true` if the approval was found and resolved.
    pub fn resolve(&self, approval_id: &str, decision: ApprovalDecision) -> bool {
        if let Some((_, tx)) = self.pending.remove(approval_id) {
            let _ = tx.send(decision);
            info!("approval: resolved {approval_id} -> {:?}", decision);
            true
        } else {
            debug!("approval: no pending request for {approval_id}");
            false
        }
    }

    /// Cache a decision in the session cache.
    pub fn cache_session(&self, tool_name: &str, decision: ApprovalDecision) {
        self.session_cache.insert(tool_name.to_string(), decision);
    }

    /// Cache a decision with a specific pattern (e.g., shell command).
    pub fn cache_session_pattern(
        &self,
        tool_name: &str,
        pattern: &str,
        decision: ApprovalDecision,
    ) {
        let key = format!("{tool_name}:{pattern}");
        self.session_cache.insert(key, decision);
    }

    /// Save a persistent "always" rule to the database.
    pub async fn save_rule(
        &self,
        tool_name: &str,
        pattern: Option<&str>,
        decision: ApprovalDecision,
        surface: &str,
    ) -> Result<ApprovalRule> {
        let id = uuid::Uuid::new_v4().to_string();
        let decision_str = match decision {
            ApprovalDecision::Approve | ApprovalDecision::ApproveAlways => "approve",
            ApprovalDecision::Deny => "deny",
        };

        let rule = ApprovalRule {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            pattern: pattern.map(|p| p.to_string()),
            decision: decision_str.to_string(),
            surface: surface.to_string(),
            created_at: String::new(),
        };

        let db = self.db.clone();
        let tool_name = tool_name.to_string();
        let pattern = pattern.map(|p| p.to_string());
        let surface = surface.to_string();

        db::with_db(&db, move |conn| {
            conn.execute(
                "INSERT INTO approval_rules (id, tool_name, pattern, decision, surface) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, tool_name, pattern, decision_str, surface],
            )
            .map_err(crate::ZeniiError::from)?;
            Ok(())
        })
        .await?;

        info!(
            "approval: saved rule for tool={}, surface={}",
            rule.tool_name, rule.surface
        );
        Ok(rule)
    }

    /// List all persistent approval rules.
    pub async fn list_rules(&self) -> Result<Vec<ApprovalRule>> {
        let db = self.db.clone();
        db::with_db(&db, |conn| {
            let mut stmt = conn
                .prepare("SELECT id, tool_name, pattern, decision, surface, created_at FROM approval_rules ORDER BY created_at DESC")
                .map_err(crate::ZeniiError::from)?;

            let rules = stmt
                .query_map([], |row| {
                    Ok(ApprovalRule {
                        id: row.get(0)?,
                        tool_name: row.get(1)?,
                        pattern: row.get(2)?,
                        decision: row.get(3)?,
                        surface: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                })
                .map_err(crate::ZeniiError::from)?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rules)
        })
        .await
    }

    /// Delete a persistent approval rule by ID.
    pub async fn delete_rule(&self, rule_id: &str) -> Result<bool> {
        let db = self.db.clone();
        let rule_id = rule_id.to_string();
        db::with_db(&db, move |conn| {
            let affected = conn
                .execute("DELETE FROM approval_rules WHERE id = ?1", [&rule_id])
                .map_err(crate::ZeniiError::from)?;
            Ok(affected > 0)
        })
        .await
    }

    /// Find a matching persistent rule for a tool call.
    async fn find_matching_rule(
        &self,
        tool_name: &str,
        args_summary: &str,
        surface: &str,
    ) -> Result<Option<ApprovalRule>> {
        let db = self.db.clone();
        let tool_name = tool_name.to_string();
        let args_summary = args_summary.to_string();
        let surface = surface.to_string();

        db::with_db(&db, move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, tool_name, pattern, decision, surface, created_at \
                     FROM approval_rules \
                     WHERE tool_name = ?1 AND surface = ?2 \
                     ORDER BY created_at DESC",
                )
                .map_err(crate::ZeniiError::from)?;

            let rules: Vec<ApprovalRule> = stmt
                .query_map(rusqlite::params![tool_name, surface], |row| {
                    Ok(ApprovalRule {
                        id: row.get(0)?,
                        tool_name: row.get(1)?,
                        pattern: row.get(2)?,
                        decision: row.get(3)?,
                        surface: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                })
                .map_err(crate::ZeniiError::from)?
                .filter_map(|r| r.ok())
                .collect();

            for rule in rules {
                match &rule.pattern {
                    None => return Ok(Some(rule)), // Blanket rule for this tool
                    Some(pattern) => {
                        if glob_match(pattern, &args_summary) {
                            return Ok(Some(rule));
                        }
                    }
                }
            }

            Ok(None)
        })
        .await
    }

    /// Cancel a pending approval request, removing it from the pending map.
    /// Use this when an approval times out to avoid leaking entries.
    pub fn cancel(&self, approval_id: &str) {
        self.pending.remove(approval_id);
    }

    /// Number of pending approval requests (for testing/diagnostics).
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// Simple glob matching for approval patterns.
/// Supports `*` (match any) and exact match.
fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }
    pattern == value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn test_broker() -> (tempfile::TempDir, ApprovalBroker) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, |conn| db::run_migrations(conn))
            .await
            .unwrap();
        (dir, ApprovalBroker::new(pool))
    }

    #[test]
    fn approval_decision_round_trip() {
        let d = ApprovalDecision::ApproveAlways;
        assert_eq!(d.as_str(), "approve_always");
        assert_eq!(ApprovalDecision::from_str_lossy("approve_always"), d);
        assert_eq!(
            ApprovalDecision::from_str_lossy("unknown"),
            ApprovalDecision::Deny
        );
    }

    #[test]
    fn glob_match_wildcard() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("git *", "git push"));
        assert!(glob_match("*push", "git push"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "not_exact"));
    }

    #[tokio::test]
    async fn register_and_resolve() {
        let (_dir, broker) = test_broker().await;
        let rx = broker.register("req-1");
        assert_eq!(broker.pending_count(), 1);

        broker.resolve("req-1", ApprovalDecision::Approve);
        let decision = rx.await.unwrap();
        assert_eq!(decision, ApprovalDecision::Approve);
        assert_eq!(broker.pending_count(), 0);
    }

    #[tokio::test]
    async fn resolve_unknown_returns_false() {
        let (_dir, broker) = test_broker().await;
        assert!(!broker.resolve("nonexistent", ApprovalDecision::Deny));
    }

    #[tokio::test]
    async fn session_cache_hit() {
        let (_dir, broker) = test_broker().await;
        broker.cache_session("shell", ApprovalDecision::Approve);
        let result = broker.pre_check("shell", "echo hello", "desktop").await;
        assert_eq!(result, Some(ApprovalDecision::Approve));
    }

    #[tokio::test]
    async fn session_cache_pattern_hit() {
        let (_dir, broker) = test_broker().await;
        broker.cache_session_pattern("shell", "git push", ApprovalDecision::Approve);
        let result = broker.pre_check("shell", "git push", "desktop").await;
        assert_eq!(result, Some(ApprovalDecision::Approve));
    }

    #[tokio::test]
    async fn pre_check_miss() {
        let (_dir, broker) = test_broker().await;
        let result = broker.pre_check("shell", "rm -rf /", "desktop").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn save_and_list_rules() {
        let (_dir, broker) = test_broker().await;
        broker
            .save_rule("shell", Some("git *"), ApprovalDecision::Approve, "desktop")
            .await
            .unwrap();
        broker
            .save_rule("file_write", None, ApprovalDecision::Deny, "telegram")
            .await
            .unwrap();

        let rules = broker.list_rules().await.unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[tokio::test]
    async fn delete_rule() {
        let (_dir, broker) = test_broker().await;
        let rule = broker
            .save_rule("shell", None, ApprovalDecision::Approve, "desktop")
            .await
            .unwrap();

        assert!(broker.delete_rule(&rule.id).await.unwrap());
        assert!(!broker.delete_rule("nonexistent").await.unwrap());

        let rules = broker.list_rules().await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn persistent_rule_pre_check() {
        let (_dir, broker) = test_broker().await;
        broker
            .save_rule("shell", Some("git *"), ApprovalDecision::Approve, "desktop")
            .await
            .unwrap();

        let result = broker
            .pre_check("shell", "git push origin main", "desktop")
            .await;
        assert_eq!(result, Some(ApprovalDecision::Approve));

        // Different surface: no match
        let result = broker
            .pre_check("shell", "git push origin main", "telegram")
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn blanket_rule_matches_any_args() {
        let (_dir, broker) = test_broker().await;
        broker
            .save_rule("shell", None, ApprovalDecision::Approve, "desktop")
            .await
            .unwrap();

        let result = broker.pre_check("shell", "anything", "desktop").await;
        assert_eq!(result, Some(ApprovalDecision::Approve));
    }
}
