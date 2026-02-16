/**
 * Tauri command wrappers for the skill system.
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  SkillDefinition,
  SkillInfo,
  SkillOutput,
  SkillSettings,
} from "./types";

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
 * Get skill settings for a workspace.
 */
export async function getSkillSettings(
  workspaceId: string
): Promise<SkillSettings> {
  return invoke<SkillSettings>("get_skill_settings_command", {
    workspaceId,
  });
}

/**
 * Set whether a skill is enabled for a workspace.
 */
export async function setSkillEnabled(
  workspaceId: string,
  skillId: string,
  enabled: boolean
): Promise<void> {
  return invoke("set_skill_enabled_command", {
    workspaceId,
    skillId,
    enabled,
  });
}

/**
 * Update skill configuration for a workspace.
 */
export async function updateSkillConfig(
  workspaceId: string,
  skillId: string,
  enabled: boolean,
  priorityOverride?: number
): Promise<void> {
  return invoke("update_skill_config_command", {
    workspaceId,
    skillId,
    enabled,
    priorityOverride,
  });
}

/**
 * Initialize default skill settings for a workspace.
 */
export async function initializeSkillDefaults(
  workspaceId: string
): Promise<void> {
  return invoke("initialize_skill_defaults_command", {
    workspaceId,
  });
}

/**
 * Execute a request using the skill system with automatic skill selection.
 */
export async function executeWithSkills(
  workspaceId: string,
  request: string,
  providerId: string,
  apiKey: string,
  options?: {
    entityType?: string;
    taskHint?: string;
  }
): Promise<SkillOutput> {
  return invoke<SkillOutput>("execute_with_skills_command", {
    workspaceId,
    request,
    entityType: options?.entityType,
    taskHint: options?.taskHint,
    providerId,
    apiKey,
  });
}

/**
 * Execute a specific skill by ID.
 */
export async function executeSkill(
  workspaceId: string,
  skillId: string,
  request: string,
  providerId: string,
  apiKey: string
): Promise<SkillOutput> {
  return invoke<SkillOutput>("execute_skill_command", {
    workspaceId,
    skillId,
    request,
    providerId,
    apiKey,
  });
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
 * Toggle auto-select mode for a workspace.
 */
export async function setSkillAutoSelect(
  workspaceId: string,
  autoSelect: boolean
): Promise<void> {
  return invoke("set_skill_auto_select_command", {
    workspaceId,
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
 * Returns skills that might be relevant based on trigger matching.
 * Useful for showing skill chips in the chat UI.
 */
export async function suggestSkills(
  workspaceId: string,
  request: string,
  databaseType?: string
): Promise<SkillSuggestion[]> {
  return invoke<SkillSuggestion[]>("suggest_skills_command", {
    workspaceId,
    request,
    databaseType,
  });
}

// Re-export types
export type {
  SkillDefinition,
  SkillInfo,
  SkillOutput,
  SkillSettings,
  SkillSource,
  SkillUserConfig,
} from "./types";
