use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::{agent_runs, agent_sessions, agents};
use crate::database::utils::{bool_to_int, int_to_bool};

/// Agent database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = agents)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub provider_id: String,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub tools_enabled: i32,
    pub memory_enabled: i32,
    pub workspace_path: Option<String>,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Typed Agent with boolean conversions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub provider_id: String,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub tools_enabled: bool,
    pub memory_enabled: bool,
    pub workspace_path: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Agent> for AgentData {
    fn from(agent: Agent) -> Self {
        Self {
            id: agent.id,
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
            model_id: agent.model_id,
            provider_id: agent.provider_id,
            temperature: agent.temperature,
            max_tokens: agent.max_tokens,
            tools_enabled: int_to_bool(agent.tools_enabled),
            memory_enabled: int_to_bool(agent.memory_enabled),
            workspace_path: agent.workspace_path,
            is_active: int_to_bool(agent.is_active),
            created_at: agent.created_at,
            updated_at: agent.updated_at,
        }
    }
}

/// New agent for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agents)]
pub struct NewAgent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub provider_id: String,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub tools_enabled: i32,
    pub memory_enabled: i32,
    pub workspace_path: Option<String>,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NewAgent {
    /// Create a new agent for insertion
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: Option<String>,
        system_prompt: impl Into<String>,
        model_id: impl Into<String>,
        provider_id: impl Into<String>,
        temperature: f32,
        max_tokens: Option<i32>,
        tools_enabled: bool,
        memory_enabled: bool,
        workspace_path: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            name: name.into(),
            description,
            system_prompt: system_prompt.into(),
            model_id: model_id.into(),
            provider_id: provider_id.into(),
            temperature,
            max_tokens,
            tools_enabled: bool_to_int(tools_enabled),
            memory_enabled: bool_to_int(memory_enabled),
            workspace_path,
            is_active: 1,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Agent Session database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = agent_sessions)]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Typed Agent Session with status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionData {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub status: SessionStatus,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Session status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl From<String> for SessionStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => SessionStatus::Pending,
            "running" => SessionStatus::Running,
            "completed" => SessionStatus::Completed,
            "failed" => SessionStatus::Failed,
            "cancelled" => SessionStatus::Cancelled,
            _ => SessionStatus::Pending,
        }
    }
}

impl From<SessionStatus> for String {
    fn from(status: SessionStatus) -> Self {
        match status {
            SessionStatus::Pending => "pending".to_string(),
            SessionStatus::Running => "running".to_string(),
            SessionStatus::Completed => "completed".to_string(),
            SessionStatus::Failed => "failed".to_string(),
            SessionStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

impl From<AgentSession> for AgentSessionData {
    fn from(session: AgentSession) -> Self {
        Self {
            id: session.id,
            agent_id: session.agent_id,
            name: session.name,
            status: SessionStatus::from(session.status),
            created_at: session.created_at,
            updated_at: session.updated_at,
            completed_at: session.completed_at,
        }
    }
}

/// New agent session for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agent_sessions)]
pub struct NewAgentSession {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

impl NewAgentSession {
    /// Create a new agent session for insertion
    pub fn new(
        id: impl Into<String>,
        agent_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            agent_id: agent_id.into(),
            name: name.into(),
            status: String::from(SessionStatus::Pending),
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
        }
    }
}

/// Agent Run database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = agent_runs)]
pub struct AgentRun {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub parent_run_id: Option<String>,
    pub status: String,
    pub input_message: String,
    pub output_message: Option<String>,
    pub error_message: Option<String>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

/// Typed Agent Run with status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunData {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub parent_run_id: Option<String>,
    pub status: RunStatus,
    pub input_message: String,
    pub output_message: Option<String>,
    pub error_message: Option<String>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

/// Run status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl From<String> for RunStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => RunStatus::Pending,
            "running" => RunStatus::Running,
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            "cancelled" => RunStatus::Cancelled,
            _ => RunStatus::Pending,
        }
    }
}

impl From<RunStatus> for String {
    fn from(status: RunStatus) -> Self {
        match status {
            RunStatus::Pending => "pending".to_string(),
            RunStatus::Running => "running".to_string(),
            RunStatus::Completed => "completed".to_string(),
            RunStatus::Failed => "failed".to_string(),
            RunStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

impl From<AgentRun> for AgentRunData {
    fn from(run: AgentRun) -> Self {
        Self {
            id: run.id,
            session_id: run.session_id,
            agent_id: run.agent_id,
            parent_run_id: run.parent_run_id,
            status: RunStatus::from(run.status),
            input_message: run.input_message,
            output_message: run.output_message,
            error_message: run.error_message,
            tokens_used: run.tokens_used,
            duration_ms: run.duration_ms,
            started_at: run.started_at,
            completed_at: run.completed_at,
            created_at: run.created_at,
        }
    }
}

/// New agent run for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agent_runs)]
pub struct NewAgentRun {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub parent_run_id: Option<String>,
    pub status: String,
    pub input_message: String,
    pub output_message: Option<String>,
    pub error_message: Option<String>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

impl NewAgentRun {
    /// Create a new agent run for insertion
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
        parent_run_id: Option<String>,
        input_message: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            parent_run_id,
            status: String::from(RunStatus::Pending),
            input_message: input_message.into(),
            output_message: None,
            error_message: None,
            tokens_used: None,
            duration_ms: None,
            started_at: None,
            completed_at: None,
            created_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_agent_creation() {
        let agent = NewAgent::new(
            "test-agent",
            "Test Agent",
            Some("Test description".to_string()),
            "You are a helpful assistant",
            "gpt-4o",
            "openai",
            0.7,
            Some(4000),
            true,
            true,
            Some("/workspace/test".to_string()),
        );

        assert_eq!(agent.id, "test-agent");
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.description, Some("Test description".to_string()));
        assert_eq!(agent.system_prompt, "You are a helpful assistant");
        assert_eq!(agent.model_id, "gpt-4o");
        assert_eq!(agent.provider_id, "openai");
        assert_eq!(agent.temperature, 0.7);
        assert_eq!(agent.max_tokens, Some(4000));
        assert_eq!(agent.tools_enabled, 1);
        assert_eq!(agent.memory_enabled, 1);
        assert_eq!(agent.workspace_path, Some("/workspace/test".to_string()));
        assert_eq!(agent.is_active, 1);
    }

    #[test]
    fn test_agent_data_conversion() {
        let agent = Agent {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: Some("Test agent".to_string()),
            system_prompt: "Prompt".to_string(),
            model_id: "model".to_string(),
            provider_id: "provider".to_string(),
            temperature: 0.7,
            max_tokens: Some(4000),
            tools_enabled: 1,
            memory_enabled: 0,
            workspace_path: None,
            is_active: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let data = AgentData::from(agent);
        assert!(data.tools_enabled);
        assert!(!data.memory_enabled);
        assert!(data.is_active);
    }

    #[test]
    fn test_session_status_conversion() {
        assert_eq!(
            SessionStatus::from("pending".to_string()),
            SessionStatus::Pending
        );
        assert_eq!(
            SessionStatus::from("running".to_string()),
            SessionStatus::Running
        );
        assert_eq!(
            SessionStatus::from("completed".to_string()),
            SessionStatus::Completed
        );
        assert_eq!(
            SessionStatus::from("failed".to_string()),
            SessionStatus::Failed
        );
        assert_eq!(
            SessionStatus::from("cancelled".to_string()),
            SessionStatus::Cancelled
        );
        assert_eq!(
            SessionStatus::from("invalid".to_string()),
            SessionStatus::Pending
        );
    }

    #[test]
    fn test_session_status_to_string() {
        assert_eq!(String::from(SessionStatus::Pending), "pending");
        assert_eq!(String::from(SessionStatus::Running), "running");
        assert_eq!(String::from(SessionStatus::Completed), "completed");
        assert_eq!(String::from(SessionStatus::Failed), "failed");
        assert_eq!(String::from(SessionStatus::Cancelled), "cancelled");
    }

    #[test]
    fn test_new_session_creation() {
        let session = NewAgentSession::new("session-1", "agent-1", "Test Session");

        assert_eq!(session.id, "session-1");
        assert_eq!(session.agent_id, "agent-1");
        assert_eq!(session.name, "Test Session");
        assert_eq!(session.status, "pending");
        assert!(session.completed_at.is_none());
    }

    #[test]
    fn test_run_status_conversion() {
        assert_eq!(RunStatus::from("pending".to_string()), RunStatus::Pending);
        assert_eq!(RunStatus::from("running".to_string()), RunStatus::Running);
        assert_eq!(
            RunStatus::from("completed".to_string()),
            RunStatus::Completed
        );
        assert_eq!(RunStatus::from("failed".to_string()), RunStatus::Failed);
        assert_eq!(
            RunStatus::from("cancelled".to_string()),
            RunStatus::Cancelled
        );
    }

    #[test]
    fn test_run_status_to_string() {
        assert_eq!(String::from(RunStatus::Pending), "pending");
        assert_eq!(String::from(RunStatus::Running), "running");
        assert_eq!(String::from(RunStatus::Completed), "completed");
        assert_eq!(String::from(RunStatus::Failed), "failed");
        assert_eq!(String::from(RunStatus::Cancelled), "cancelled");
    }

    #[test]
    fn test_new_run_creation() {
        let run = NewAgentRun::new(
            "run-1",
            "session-1",
            "agent-1",
            Some("parent-run-1".to_string()),
            "Test input message",
        );

        assert_eq!(run.id, "run-1");
        assert_eq!(run.session_id, "session-1");
        assert_eq!(run.agent_id, "agent-1");
        assert_eq!(run.parent_run_id, Some("parent-run-1".to_string()));
        assert_eq!(run.status, "pending");
        assert_eq!(run.input_message, "Test input message");
        assert!(run.output_message.is_none());
        assert!(run.error_message.is_none());
        assert!(run.tokens_used.is_none());
        assert!(run.duration_ms.is_none());
        assert!(run.started_at.is_none());
        assert!(run.completed_at.is_none());
    }

    #[test]
    fn test_run_data_conversion() {
        let run = AgentRun {
            id: "run-1".to_string(),
            session_id: "session-1".to_string(),
            agent_id: "agent-1".to_string(),
            parent_run_id: None,
            status: "completed".to_string(),
            input_message: "Input".to_string(),
            output_message: Some("Output".to_string()),
            error_message: None,
            tokens_used: Some(100),
            duration_ms: Some(5000),
            started_at: Some("2024-01-01T00:00:00Z".to_string()),
            completed_at: Some("2024-01-01T00:00:05Z".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let data = AgentRunData::from(run);
        assert_eq!(data.status, RunStatus::Completed);
        assert_eq!(data.tokens_used, Some(100));
        assert_eq!(data.duration_ms, Some(5000));
    }
}
