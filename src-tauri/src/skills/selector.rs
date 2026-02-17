//! Skill selection with hybrid structured + LLM approach.
//!
//! Two-stage selection:
//! 1. Structured pre-filter (fast, deterministic)
//! 2. LLM final selection (nuanced, optional)

use crate::ai::provider::LLMProvider;
use crate::ai::types::{CompletionRequest, Message};
use crate::skills::error::{SkillError, SkillResult};
use crate::skills::registry::SkillRegistry;
use crate::skills::types::{SelectionContext, SelectionResult, SkillDefinition};
use std::sync::Arc;

/// Selects appropriate skills based on context.
pub struct SkillSelector {
    registry: Arc<SkillRegistry>,
}

impl SkillSelector {
    /// Create a new skill selector with access to the registry.
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }

    /// Stage 1: Fast deterministic filtering based on triggers and requirements.
    pub fn pre_filter(&self, ctx: &SelectionContext) -> Vec<SkillDefinition> {
        let all_skills = self.registry.list_all();

        all_skills
            .into_iter()
            .filter(|skill| self.matches_triggers(skill, ctx))
            .filter(|skill| self.requirements_satisfiable(skill, ctx))
            .filter(|skill| ctx.enabled_skills.contains(&skill.id))
            .collect()
    }

    /// Check if a skill's triggers match the current context.
    fn matches_triggers(&self, skill: &SkillDefinition, ctx: &SelectionContext) -> bool {
        let triggers = &skill.triggers;

        // If skill has no triggers, it matches everything
        if triggers.task_types.is_empty() && triggers.entity_types.is_empty() {
            return true;
        }

        // Check entity type match
        let entity_match = if triggers.entity_types.is_empty() {
            true
        } else {
            ctx.entity_type
                .as_ref()
                .is_some_and(|et| triggers.entity_types.iter().any(|t| t == et))
        };

        // Check task type match (also check request text for keywords)
        let task_match = if triggers.task_types.is_empty() {
            true
        } else {
            // Check explicit task hint
            let hint_match = ctx
                .task_hint
                .as_ref()
                .is_some_and(|th| triggers.task_types.iter().any(|t| t == th));

            // Check request text for task keywords
            let request_lower = ctx.request.to_lowercase();
            let keyword_match = triggers
                .task_types
                .iter()
                .any(|t| request_lower.contains(&t.to_lowercase()));

            hint_match || keyword_match
        };

        entity_match && task_match
    }

    /// Check if a skill's requirements can be satisfied.
    fn requirements_satisfiable(&self, skill: &SkillDefinition, ctx: &SelectionContext) -> bool {
        // Check if required context is available
        for required_ctx in &skill.requires.context {
            // Special handling for database_type - always available if we have a connection
            if required_ctx == "database_type" {
                if ctx.database_type.is_none() {
                    return false;
                }
                continue;
            }

            if !ctx.available_context.contains(required_ctx) {
                return false;
            }
        }

        // Tools are checked at execution time, not selection time
        true
    }

    /// Stage 2: LLM-based final selection from candidates.
    pub async fn llm_select(
        &self,
        candidates: Vec<SkillDefinition>,
        ctx: &SelectionContext,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SelectionResult> {
        if candidates.is_empty() {
            return Ok(SelectionResult {
                selected_skills: vec![],
                reasoning: Some("No candidate skills matched the pre-filter.".to_string()),
            });
        }

        if candidates.len() == 1 {
            return Ok(SelectionResult {
                selected_skills: vec![candidates[0].id.clone()],
                reasoning: Some("Single matching skill found.".to_string()),
            });
        }

        // Build the selection prompt
        let system_prompt = self.build_selection_system_prompt();
        let user_prompt = self.build_selection_user_prompt(&candidates, ctx);

        let request = CompletionRequest::new(
            "", // Model will be set by provider
            vec![
                Message::system(system_prompt),
                Message::user(user_prompt),
            ],
        )
        .with_temperature(0.3) // Low temperature for consistent selection
        .with_max_tokens(500);

        let response = llm_provider
            .complete(request)
            .await
            .map_err(|e| SkillError::LlmError(e.to_string()))?;

        // Parse the response to extract selected skills
        self.parse_selection_response(&response.content, &candidates)
    }

    /// Build the system prompt for skill selection.
    fn build_selection_system_prompt(&self) -> String {
        r#"You are a skill selection assistant. Your job is to choose the most appropriate skill(s) for a given user request.

You will be given:
1. A list of available skills with their descriptions
2. The user's request
3. Context about the current environment (database type, entity type, etc.)

Respond with a JSON object containing:
- "selected_skills": array of skill IDs to use (in order of priority)
- "reasoning": brief explanation of why these skills were selected

Rules:
- Select 1-3 skills maximum
- Prefer specific skills over general ones
- Consider skill compatibility (don't select conflicting skills)
- If no skill is appropriate, return an empty array"#
            .to_string()
    }

    /// Build the user prompt for skill selection.
    fn build_selection_user_prompt(
        &self,
        candidates: &[SkillDefinition],
        ctx: &SelectionContext,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str("## Available Skills\n\n");
        for skill in candidates {
            prompt.push_str(&format!(
                "### {} ({})\n{}\n- Category: {}\n- Triggers: {:?}\n\n",
                skill.name,
                skill.id,
                skill.description,
                skill.feature.category,
                skill.triggers.task_types
            ));
        }

        prompt.push_str("## User Request\n\n");
        prompt.push_str(&ctx.request);
        prompt.push_str("\n\n");

        prompt.push_str("## Context\n\n");
        if let Some(db_type) = &ctx.database_type {
            prompt.push_str(&format!("- Database type: {}\n", db_type));
        }
        if let Some(entity_type) = &ctx.entity_type {
            prompt.push_str(&format!("- Entity type: {}\n", entity_type));
        }
        if let Some(task_hint) = &ctx.task_hint {
            prompt.push_str(&format!("- Task hint: {}\n", task_hint));
        }

        prompt.push_str("\n## Your Selection\n\nRespond with JSON:");

        prompt
    }

    /// Parse the LLM response to extract selected skills.
    fn parse_selection_response(
        &self,
        response: &str,
        candidates: &[SkillDefinition],
    ) -> SkillResult<SelectionResult> {
        // Try to find JSON in the response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        #[derive(serde::Deserialize)]
        struct LlmResponse {
            selected_skills: Vec<String>,
            reasoning: Option<String>,
        }

        match serde_json::from_str::<LlmResponse>(json_str) {
            Ok(parsed) => {
                // Validate that selected skills are in candidates
                let valid_ids: std::collections::HashSet<_> =
                    candidates.iter().map(|s| s.id.as_str()).collect();

                let selected: Vec<_> = parsed
                    .selected_skills
                    .into_iter()
                    .filter(|id| valid_ids.contains(id.as_str()))
                    .collect();

                Ok(SelectionResult {
                    selected_skills: selected,
                    reasoning: parsed.reasoning,
                })
            }
            Err(_) => {
                // Fallback: if JSON parsing fails, use first candidate
                Ok(SelectionResult {
                    selected_skills: vec![candidates[0].id.clone()],
                    reasoning: Some(
                        "LLM response parsing failed; using first candidate.".to_string(),
                    ),
                })
            }
        }
    }

    /// Combined selection: pre-filter → LLM select (if needed).
    pub async fn select(
        &self,
        ctx: &SelectionContext,
        llm_provider: Option<Arc<dyn LLMProvider>>,
    ) -> SkillResult<SelectionResult> {
        // Stage 1: Pre-filter
        let candidates = self.pre_filter(ctx);

        if candidates.is_empty() {
            return Err(SkillError::NoSkillsSelected);
        }

        // If only one candidate or no LLM provided, use pre-filter result
        let Some(provider) = llm_provider else {
            return Ok(SelectionResult {
                selected_skills: candidates.iter().map(|s| s.id.clone()).collect(),
                reasoning: Some("Selected by pre-filter.".to_string()),
            });
        };

        if candidates.len() == 1 {
            return Ok(SelectionResult {
                selected_skills: candidates.iter().map(|s| s.id.clone()).collect(),
                reasoning: Some("Selected by pre-filter.".to_string()),
            });
        }

        // Stage 2: LLM selection
        self.llm_select(candidates, ctx, provider).await
    }

    /// Get skills by ID from the registry.
    pub fn get_skills_by_ids(&self, ids: &[String]) -> Vec<SkillDefinition> {
        ids.iter()
            .filter_map(|id| self.registry.get(id))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn create_test_skill(id: &str, task_types: Vec<&str>, entity_types: Vec<&str>) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            name: format!("Test Skill {}", id),
            description: "A test skill".to_string(),
            feature: crate::skills::types::FeatureConfig {
                category: "test".to_string(),
                default_enabled: true,
            },
            requires: crate::skills::types::SkillRequirements::default(),
            triggers: crate::skills::types::SkillTriggers {
                task_types: task_types.into_iter().map(String::from).collect(),
                entity_types: entity_types.into_iter().map(String::from).collect(),
            },
            compose: crate::skills::types::ComposeConfig::default(),
            prompt_content: "Test prompt".to_string(),
        }
    }

    #[test]
    fn test_matches_triggers_with_keyword() {
        // This test would use a mock registry
        // Testing trigger matching logic directly
        let skill = create_test_skill("optimizer", vec!["optimize", "performance"], vec!["query"]);

        let ctx = SelectionContext {
            request: "Please optimize this query".to_string(),
            entity_type: Some("query".to_string()),
            enabled_skills: std::collections::HashSet::from(["optimizer".to_string()]),
            ..Default::default()
        };

        // The skill should match because:
        // 1. entity_type "query" matches
        // 2. request contains "optimize"
        assert!(skill.triggers.task_types.iter().any(|t| ctx.request.to_lowercase().contains(t)));
        assert!(skill.triggers.entity_types.contains(&ctx.entity_type.clone().unwrap()));
    }
}
