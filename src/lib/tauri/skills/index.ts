/**
 * Tauri command wrappers for the skill system.
 */

import { invoke } from "@tauri-apps/api/core";

import type { SkillDefinition, SkillInfo, SkillSettings } from "./types";

/**
 * List all available skills.
 */
export async function listAvailableSkills(): Promise<SkillInfo[]> {
  return invoke<SkillInfo[]>("list_available_skills_command");
}

/**
 * Get detailed information about a specific skill.
 */
export async function getSkillDetails(
  skillId: string
): Promise<SkillDefinition> {
  return invoke<SkillDefinition>("get_skill_details_command", {
    skillId,
  });
}

/**
 * Get skill settings (all templates are always enabled).
 */
export async function getSkillSettings(): Promise<SkillSettings> {
  return invoke<SkillSettings>("get_skill_settings_command");
}

/**
 * Set whether a skill is enabled (no-op in the template system).
 */
export async function setSkillEnabled(
  skillId: string,
  enabled: boolean
): Promise<void> {
  return invoke("set_skill_enabled_command", {
    skillId,
    enabled,
  });
}

/**
 * Update skill configuration (no-op in the template system).
 */
export async function updateSkillConfig(
  skillId: string,
  enabled: boolean,
  priorityOverride?: number
): Promise<void> {
  return invoke("update_skill_config_command", {
    skillId,
    enabled,
    priorityOverride,
  });
}

/**
 * Initialize default skill settings (no-op in the template system).
 */
export async function initializeSkillDefaults(): Promise<void> {
  return invoke("initialize_skill_defaults_command");
}

/**
 * Reload skills from all sources.
 */
export async function reloadSkills(): Promise<void> {
  return invoke("reload_skills_command");
}

/**
 * List skills grouped by category.
 */
export async function listSkillsByCategory(): Promise<
  Record<string, SkillInfo[]>
> {
  return invoke<Record<string, SkillInfo[]>>("list_skills_by_category_command");
}

/**
 * Toggle auto-select mode (no-op in the template system).
 */
export async function setSkillAutoSelect(autoSelect: boolean): Promise<void> {
  return invoke("set_skill_auto_select_command", {
    autoSelect,
  });
}

/**
 * Skill suggestion returned by suggestSkills.
 */
export interface SkillSuggestion {
  skillId: string;
  skillName: string;
  description: string;
  relevanceScore: number;
  matchedTriggers: string[];
}

/**
 * Get skill suggestions for a user request.
 *
 * Returns an empty list — the template system does not perform AI-based
 * selection.
 */
export async function suggestSkills(
  request: string
): Promise<SkillSuggestion[]> {
  return invoke<SkillSuggestion[]>("suggest_skills_command", {
    request,
  });
}

// Re-export types
export type {
  SkillDefinition,
  SkillInfo,
  SkillSettings,
  SkillUserConfig,
} from "./types";
