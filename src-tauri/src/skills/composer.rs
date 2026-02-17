//! Skill composition for combining multiple skills.
//!
//! Handles merging, chaining, and parallel execution of skills.

use crate::skills::error::{SkillError, SkillResult};
use crate::skills::types::{
    ComposeMode, ComposedSkill, ExecutionPlan, PromptPosition, SkillDefinition, SkillRequirements,
};

/// Composes multiple skills into a single executable unit.
pub struct SkillComposer;

impl SkillComposer {
    /// Create a new skill composer.
    pub fn new() -> Self {
        Self
    }

    /// Compose multiple skills into a ComposedSkill.
    pub fn compose(&self, skills: Vec<SkillDefinition>) -> SkillResult<ComposedSkill> {
        if skills.is_empty() {
            return Err(SkillError::NoSkillsSelected);
        }

        // Check compatibility
        self.check_compatibility(&skills)?;

        // Sort by priority (highest first)
        let sorted_skills = self.sort_by_priority(skills);

        // Determine execution mode
        let execution_plan = self.build_execution_plan(&sorted_skills);

        // Merge prompts based on mode
        let merged_prompt = self.merge_prompts(&sorted_skills, &execution_plan);

        // Combine requirements
        let combined_requirements = self.combine_requirements(&sorted_skills);

        Ok(ComposedSkill {
            skills: sorted_skills,
            merged_prompt,
            execution_plan,
            combined_requirements,
        })
    }

    /// Check if all skills are compatible with each other.
    fn check_compatibility(&self, skills: &[SkillDefinition]) -> SkillResult<()> {
        for (i, skill_a) in skills.iter().enumerate() {
            for skill_b in skills.iter().skip(i + 1) {
                // Check if skill_a conflicts with skill_b
                if skill_a.compose.conflicts_with.contains(&skill_b.id) {
                    return Err(SkillError::IncompatibleSkills(
                        skill_a.id.clone(),
                        skill_b.id.clone(),
                    ));
                }

                // Check if skill_b conflicts with skill_a
                if skill_b.compose.conflicts_with.contains(&skill_a.id) {
                    return Err(SkillError::IncompatibleSkills(
                        skill_b.id.clone(),
                        skill_a.id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Sort skills by priority (highest first).
    fn sort_by_priority(&self, mut skills: Vec<SkillDefinition>) -> Vec<SkillDefinition> {
        skills.sort_by(|a, b| b.compose.priority.cmp(&a.compose.priority));
        skills
    }

    /// Determine the execution plan based on skill compose modes.
    fn build_execution_plan(&self, skills: &[SkillDefinition]) -> ExecutionPlan {
        if skills.len() == 1 {
            return ExecutionPlan::Single(skills[0].id.clone());
        }

        // Check if any skill wants chain or parallel mode
        let has_chain = skills.iter().any(|s| s.compose.mode == ComposeMode::Chain);
        let has_parallel = skills
            .iter()
            .any(|s| s.compose.mode == ComposeMode::Parallel);

        let skill_ids: Vec<String> = skills.iter().map(|s| s.id.clone()).collect();

        if has_parallel {
            ExecutionPlan::Parallel(skill_ids)
        } else if has_chain {
            ExecutionPlan::Chained(skill_ids)
        } else {
            ExecutionPlan::Merged(skill_ids)
        }
    }

    /// Merge prompts from multiple skills.
    fn merge_prompts(&self, skills: &[SkillDefinition], plan: &ExecutionPlan) -> String {
        match plan {
            ExecutionPlan::Single(id) => skills
                .iter()
                .find(|s| &s.id == id)
                .map(|s| s.prompt_content.clone())
                .unwrap_or_default(),

            ExecutionPlan::Merged(_) => {
                let mut prepend_parts = Vec::new();
                let mut main_parts = Vec::new();
                let mut append_parts = Vec::new();

                for skill in skills {
                    match skill.compose.prompt_position {
                        PromptPosition::Prepend => {
                            prepend_parts.push(self.format_skill_section(skill));
                        }
                        PromptPosition::Append => {
                            append_parts.push(self.format_skill_section(skill));
                        }
                        PromptPosition::Replace => {
                            // Replace mode: only use this skill's prompt
                            return skill.prompt_content.clone();
                        }
                    }
                }

                // Default position is append if not specified
                if prepend_parts.is_empty() && append_parts.is_empty() {
                    main_parts = skills
                        .iter()
                        .map(|s| self.format_skill_section(s))
                        .collect();
                }

                let mut result = Vec::new();
                result.extend(prepend_parts);
                result.extend(main_parts);
                result.extend(append_parts);

                result.join("\n\n---\n\n")
            }

            ExecutionPlan::Chained(_) | ExecutionPlan::Parallel(_) => {
                // For chain/parallel, each skill keeps its own prompt
                // Return the first skill's prompt as the "main" prompt
                // The executor will handle running each skill separately
                skills
                    .first()
                    .map(|s| s.prompt_content.clone())
                    .unwrap_or_default()
            }
        }
    }

    /// Format a skill's prompt as a labeled section.
    fn format_skill_section(&self, skill: &SkillDefinition) -> String {
        format!(
            "## {} ({})\n\n{}",
            skill.name, skill.id, skill.prompt_content
        )
    }

    /// Combine requirements from all skills.
    fn combine_requirements(&self, skills: &[SkillDefinition]) -> SkillRequirements {
        skills
            .iter()
            .fold(SkillRequirements::default(), |acc, skill| {
                acc.merge(&skill.requires)
            })
    }
}

impl Default for SkillComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::skills::types::{ComposeConfig, FeatureConfig, SkillTriggers};

    fn make_skill(id: &str, priority: i32, conflicts: Vec<&str>) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            name: format!("Skill {}", id),
            description: "Test skill".to_string(),
            feature: FeatureConfig {
                category: "test".to_string(),
                default_enabled: true,
            },
            requires: SkillRequirements::default(),
            triggers: SkillTriggers::default(),
            compose: ComposeConfig {
                priority,
                conflicts_with: conflicts.into_iter().map(String::from).collect(),
                ..Default::default()
            },
            prompt_content: format!("Prompt for {}", id),
        }
    }

    #[test]
    fn test_compose_single_skill() {
        let composer = SkillComposer::new();
        let skills = vec![make_skill("test", 100, vec![])];

        let composed = composer.compose(skills).unwrap();

        assert_eq!(composed.skills.len(), 1);
        assert!(matches!(composed.execution_plan, ExecutionPlan::Single(_)));
    }

    #[test]
    fn test_compose_sorts_by_priority() {
        let composer = SkillComposer::new();
        let skills = vec![
            make_skill("low", 50, vec![]),
            make_skill("high", 150, vec![]),
            make_skill("medium", 100, vec![]),
        ];

        let composed = composer.compose(skills).unwrap();

        assert_eq!(composed.skills[0].id, "high");
        assert_eq!(composed.skills[1].id, "medium");
        assert_eq!(composed.skills[2].id, "low");
    }

    #[test]
    fn test_compose_detects_conflicts() {
        let composer = SkillComposer::new();
        let skills = vec![
            make_skill("a", 100, vec!["b"]),
            make_skill("b", 100, vec![]),
        ];

        let result = composer.compose(skills);
        assert!(matches!(result, Err(SkillError::IncompatibleSkills(_, _))));
    }

    #[test]
    fn test_compose_merges_prompts() {
        let composer = SkillComposer::new();
        let skills = vec![make_skill("a", 100, vec![]), make_skill("b", 50, vec![])];

        let composed = composer.compose(skills).unwrap();

        assert!(composed.merged_prompt.contains("Prompt for a"));
        assert!(composed.merged_prompt.contains("Prompt for b"));
    }
}
