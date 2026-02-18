/**
 * Skill system types matching the Rust backend definitions.
 */

/** Lightweight skill information for UI display */
export interface SkillInfo {
  id: string;
  name: string;
  description: string;
  category: string;
  defaultEnabled: boolean;
  source: string;
}

/** Full skill definition */
export interface SkillDefinition {
  id: string;
  name: string;
  description: string;
  category: string;
  template: string;
  parameters: TemplateParameter[];
}

/** A single parameter accepted by a template */
export interface TemplateParameter {
  name: string;
  description: string;
  required: boolean;
}

/** Per-skill user configuration */
export interface SkillUserConfig {
  skillId: string;
  enabled: boolean;
  priorityOverride?: number;
}

/** Complete skill settings for a workspace */
export interface SkillSettings {
  autoSelect: boolean;
  skills: SkillUserConfig[];
}
