use serde::{Deserialize, Serialize};

use crate::ai::agent::TokenUsage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationTask {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub tool_allowlist: Option<Vec<String>>,
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

fn default_token_budget() -> usize {
    4000
}

fn default_timeout_secs() -> u64 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: String,
    pub usage: TokenUsage,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub tool_uses: u32,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    pub id: String,
    pub task_results: Vec<TaskResult>,
    pub aggregated_response: String,
    pub total_usage: TokenUsage,
    pub total_duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // 7.1
    #[test]
    fn delegation_task_defaults() {
        let task = DelegationTask {
            id: "t1".into(),
            description: "do something".into(),
            tool_allowlist: None,
            token_budget: 4000,
            timeout_secs: 120,
            depends_on: vec![],
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"id\":\"t1\""));
        assert!(json.contains("\"tool_allowlist\":null"));
        assert!(json.contains("\"depends_on\":[]"));
    }

    // 7.2
    #[test]
    fn delegation_task_serde_roundtrip() {
        let task = DelegationTask {
            id: "t1".into(),
            description: "search the web".into(),
            tool_allowlist: Some(vec!["web_search".into()]),
            token_budget: 2000,
            timeout_secs: 60,
            depends_on: vec!["t0".into()],
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: DelegationTask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "t1");
        assert_eq!(back.tool_allowlist.unwrap(), vec!["web_search"]);
        assert_eq!(back.token_budget, 2000);
        assert_eq!(back.timeout_secs, 60);
        assert_eq!(back.depends_on, vec!["t0"]);
    }

    // 7.3
    #[test]
    fn task_status_variants() {
        let variants = vec![
            (TaskStatus::Pending, "\"Pending\""),
            (TaskStatus::Running, "\"Running\""),
            (TaskStatus::Completed, "\"Completed\""),
            (TaskStatus::Failed, "\"Failed\""),
            (TaskStatus::Cancelled, "\"Cancelled\""),
            (TaskStatus::TimedOut, "\"TimedOut\""),
        ];
        for (status, expected) in variants {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
        }
    }

    // 7.4
    #[test]
    fn task_result_with_error() {
        let result = TaskResult {
            task_id: "t1".into(),
            status: TaskStatus::Failed,
            output: String::new(),
            usage: TokenUsage::default(),
            duration_ms: 500,
            error: Some("timeout".into()),
            session_id: "s1".into(),
            tool_uses: 0,
            description: "test task".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, TaskStatus::Failed);
        assert_eq!(back.error.as_deref(), Some("timeout"));
    }

    // 7.5
    #[test]
    fn delegation_result_total_usage() {
        let r1 = TaskResult {
            task_id: "t1".into(),
            status: TaskStatus::Completed,
            output: "a".into(),
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
                cached_input_tokens: 0,
            },
            duration_ms: 500,
            error: None,
            session_id: "s1".into(),
            tool_uses: 3,
            description: "test task".into(),
        };
        let r2 = TaskResult {
            task_id: "t2".into(),
            status: TaskStatus::Completed,
            output: "b".into(),
            usage: TokenUsage {
                input_tokens: 200,
                output_tokens: 100,
                total_tokens: 300,
                cached_input_tokens: 10,
            },
            duration_ms: 700,
            error: None,
            session_id: "s2".into(),
            tool_uses: 5,
            description: "test task".into(),
        };
        let total = r1.usage.clone() + r2.usage.clone();
        let result = DelegationResult {
            id: "d1".into(),
            task_results: vec![r1, r2],
            aggregated_response: "done".into(),
            total_usage: total,
            total_duration_ms: 1200,
        };
        assert_eq!(result.total_usage.input_tokens, 300);
        assert_eq!(result.total_usage.output_tokens, 150);
        assert_eq!(result.total_usage.total_tokens, 450);
        assert_eq!(result.total_usage.cached_input_tokens, 10);
    }
}
