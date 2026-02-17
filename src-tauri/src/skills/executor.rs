//! Skill executor for running composed skills.
//!
//! Handles the execution of skills through the LLM, including
//! context gathering and output processing.

use std::sync::Arc;

use crate::adapters::{ApplicationAdapter, ContextBag};
use crate::ai::provider::LLMProvider;
use crate::ai::types::{CompletionRequest, Message};
use crate::skills::error::{SkillError, SkillResult};
use crate::skills::types::{ComposedSkill, ExecutionPlan, SkillOutput, SkillRequirements};

/// Executes composed skills through the LLM.
pub struct SkillExecutor<A: ApplicationAdapter> {
    adapter: Arc<A>,
}

/// Request for skill execution.
pub struct ExecutionRequest {
    /// The original user request
    pub user_request: String,

    /// The composed skill to execute
    pub composed_skill: ComposedSkill,

    /// Additional context to merge
    pub context_overrides: Option<serde_json::Value>,
}

impl<A: ApplicationAdapter> SkillExecutor<A> {
    /// Create a new skill executor with the given adapter.
    pub fn new(adapter: Arc<A>) -> Self {
        Self { adapter }
    }

    /// Execute a skill request.
    pub async fn execute(
        &self,
        request: ExecutionRequest,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        // Gather required context
        let context = self
            .gather_context(&request.composed_skill.combined_requirements)
            .await?;

        // Execute based on execution plan
        match &request.composed_skill.execution_plan {
            ExecutionPlan::Single(id) => {
                self.execute_single(&request, id, &context, llm_provider).await
            }
            ExecutionPlan::Merged(ids) => {
                self.execute_merged(&request, ids, &context, llm_provider).await
            }
            ExecutionPlan::Chained(ids) => {
                self.execute_chained(&request, ids, &context, llm_provider).await
            }
            ExecutionPlan::Parallel(ids) => {
                self.execute_parallel(&request, ids, &context, llm_provider).await
            }
        }
    }

    /// Gather context required by the skill.
    async fn gather_context(&self, requirements: &SkillRequirements) -> SkillResult<ContextBag> {
        let keys: Vec<&str> = requirements.context.iter().map(|s| s.as_str()).collect();

        self.adapter
            .get_context(&keys)
            .await
            .map_err(|e| SkillError::MissingContext(e.to_string()))
    }

    /// Execute a single skill.
    async fn execute_single(
        &self,
        request: &ExecutionRequest,
        skill_id: &str,
        context: &ContextBag,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        let skill = request
            .composed_skill
            .skills
            .iter()
            .find(|s| s.id == skill_id)
            .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))?;

        // Build the prompt with context
        let system_prompt = self.build_system_prompt(&skill.prompt_content, context);
        let user_prompt = self.build_user_prompt(&request.user_request, &request.context_overrides);

        // Execute
        let content = self
            .execute_completion(&system_prompt, &user_prompt, llm_provider)
            .await?;

        Ok(SkillOutput {
            skill_id: skill_id.to_string(),
            content,
            structured_data: None,
            tool_calls_made: Vec::new(),
            from_cache: false,
        })
    }

    /// Execute merged skills (combined into one prompt).
    async fn execute_merged(
        &self,
        request: &ExecutionRequest,
        skill_ids: &[String],
        context: &ContextBag,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        // Use the merged prompt from the composed skill
        let system_prompt =
            self.build_system_prompt(&request.composed_skill.merged_prompt, context);
        let user_prompt = self.build_user_prompt(&request.user_request, &request.context_overrides);

        let content = self
            .execute_completion(&system_prompt, &user_prompt, llm_provider)
            .await?;

        Ok(SkillOutput {
            skill_id: skill_ids.join("+"),
            content,
            structured_data: None,
            tool_calls_made: Vec::new(),
            from_cache: false,
        })
    }

    /// Execute chained skills (sequential, passing output forward).
    async fn execute_chained(
        &self,
        request: &ExecutionRequest,
        skill_ids: &[String],
        context: &ContextBag,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        let mut accumulated_output = String::new();

        for (i, skill_id) in skill_ids.iter().enumerate() {
            let skill = request
                .composed_skill
                .skills
                .iter()
                .find(|s| &s.id == skill_id)
                .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))?;

            // Build prompt with previous output as additional context
            let mut enhanced_prompt = skill.prompt_content.clone();
            if !accumulated_output.is_empty() {
                enhanced_prompt = format!(
                    "{}\n\n## Previous Analysis\n\n{}",
                    enhanced_prompt, accumulated_output
                );
            }

            let system_prompt = self.build_system_prompt(&enhanced_prompt, context);

            // For chained execution, only the first skill uses the original user request
            let user_prompt = if i == 0 {
                self.build_user_prompt(&request.user_request, &request.context_overrides)
            } else {
                "Continue the analysis based on the previous results.".to_string()
            };

            accumulated_output = self
                .execute_completion(&system_prompt, &user_prompt, llm_provider.clone())
                .await?;
        }

        Ok(SkillOutput {
            skill_id: skill_ids.join("->"),
            content: accumulated_output,
            structured_data: None,
            tool_calls_made: Vec::new(),
            from_cache: false,
        })
    }

    /// Execute parallel skills (concurrent, aggregated results).
    async fn execute_parallel(
        &self,
        request: &ExecutionRequest,
        skill_ids: &[String],
        context: &ContextBag,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();

        for skill_id in skill_ids {
            let skill = request
                .composed_skill
                .skills
                .iter()
                .find(|s| &s.id == skill_id)
                .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))?
                .clone();

            let context = context.clone();
            let user_request = request.user_request.clone();
            let context_overrides = request.context_overrides.clone();
            let provider = llm_provider.clone();

            let skill_id_clone = skill_id.clone();
            join_set.spawn(async move {
                let system_prompt = build_system_prompt_static(&skill.prompt_content, &context);
                let user_prompt = build_user_prompt_static(&user_request, &context_overrides);

                let request = CompletionRequest::new("", vec![
                    Message::system(&system_prompt),
                    Message::user(&user_prompt),
                ]).with_temperature(0.7).with_max_tokens(4096);

                let result = provider.complete(request).await;
                (skill_id_clone, result)
            });
        }

        let mut all_outputs = Vec::new();

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((skill_id, Ok(response))) => {
                    all_outputs.push(format!("## {} Results\n\n{}", skill_id, response.content));
                }
                Ok((skill_id, Err(e))) => {
                    all_outputs.push(format!("## {} Error\n\n{}", skill_id, e));
                }
                Err(e) => {
                    tracing::error!("Parallel skill task failed: {}", e);
                }
            }
        }

        Ok(SkillOutput {
            skill_id: skill_ids.join("|"),
            content: all_outputs.join("\n\n---\n\n"),
            structured_data: None,
            tool_calls_made: Vec::new(),
            from_cache: false,
        })
    }

    /// Build the system prompt with context injected.
    fn build_system_prompt(&self, skill_prompt: &str, context: &ContextBag) -> String {
        build_system_prompt_static(skill_prompt, context)
    }

    /// Build the user prompt.
    fn build_user_prompt(
        &self,
        user_request: &str,
        context_overrides: &Option<serde_json::Value>,
    ) -> String {
        build_user_prompt_static(user_request, context_overrides)
    }

    /// Execute a simple completion (no tool calling).
    async fn execute_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<String> {
        let request = CompletionRequest::new(
            "",  // Model will be set by provider
            vec![
                Message::system(system_prompt),
                Message::user(user_prompt),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(4096);

        let response = llm_provider
            .complete(request)
            .await
            .map_err(|e| SkillError::LlmError(e.to_string()))?;

        Ok(response.content)
    }
}

/// Build the system prompt with context injected (static version for parallel execution).
fn build_system_prompt_static(skill_prompt: &str, context: &ContextBag) -> String {
    let mut prompt = String::new();

    // Add skill prompt
    prompt.push_str(skill_prompt);
    prompt.push_str("\n\n");

    // Add context section
    if !context.values.is_empty() {
        prompt.push_str("## Available Context\n\n");

        for (key, value) in &context.values {
            // Format the value nicely
            let formatted = if value.is_string() {
                value.as_str().unwrap_or("").to_string()
            } else {
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            };

            prompt.push_str(&format!("### {}\n\n```json\n{}\n```\n\n", key, formatted));
        }
    }

    prompt
}

/// Build the user prompt (static version for parallel execution).
fn build_user_prompt_static(
    user_request: &str,
    context_overrides: &Option<serde_json::Value>,
) -> String {
    let mut prompt = user_request.to_string();

    if let Some(overrides) = context_overrides
        && let Ok(formatted) = serde_json::to_string_pretty(overrides)
    {
        prompt.push_str("\n\nAdditional context:\n```json\n");
        prompt.push_str(&formatted);
        prompt.push_str("\n```");
    }

    prompt
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::adapters::{AdapterError, ContextType, ToolCall, ToolDefinition, ToolResult};
    use crate::skills::types::{ComposeConfig, FeatureConfig, SkillDefinition, SkillTriggers};

    struct MockAdapter;

    #[async_trait::async_trait]
    impl ApplicationAdapter for MockAdapter {
        fn app_id(&self) -> &str {
            "test"
        }

        fn available_context(&self) -> Vec<ContextType> {
            vec![ContextType::new("test", "Test context", "string")]
        }

        fn available_tools(&self) -> Vec<ToolDefinition> {
            vec![]
        }

        async fn get_context(&self, _keys: &[&str]) -> Result<ContextBag, AdapterError> {
            Ok(ContextBag::new())
        }

        async fn execute_tool(&self, _call: ToolCall) -> Result<ToolResult, AdapterError> {
            Ok(ToolResult::failure("Not implemented"))
        }
    }

    #[allow(dead_code)]
    fn make_test_skill(id: &str) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            name: format!("Test {}", id),
            description: "Test skill".to_string(),
            feature: FeatureConfig {
                category: "test".to_string(),
                default_enabled: true,
            },
            requires: SkillRequirements::default(),
            triggers: SkillTriggers::default(),
            compose: ComposeConfig::default(),
            prompt_content: "Test prompt".to_string(),
        }
    }

    #[test]
    fn test_executor_creation() {
        let adapter = Arc::new(MockAdapter);
        let executor = SkillExecutor::new(adapter);

        // Just verify it compiles and can be created
        assert!(std::mem::size_of_val(&executor) > 0);
    }

    #[test]
    fn test_build_system_prompt() {
        let adapter = Arc::new(MockAdapter);
        let executor = SkillExecutor::new(adapter);

        let mut context = ContextBag::new();
        context.insert("test", "test_value").unwrap();

        let prompt = executor.build_system_prompt("Base prompt", &context);

        assert!(prompt.contains("Base prompt"));
        assert!(prompt.contains("test"));
        assert!(prompt.contains("test_value"));
    }
}
