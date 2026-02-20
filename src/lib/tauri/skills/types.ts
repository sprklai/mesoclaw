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

/** Full skill definition with enhanced metadata */
export interface SkillDefinition {
  id: string;
  name: string;
  description: string;
  category: string;
  template: string;
  parameters: TemplateParameter[];
  metadata: SkillMetadata;
  enabled: boolean;
  source: SkillSource;
  filePath?: string;
}

/** Extended skill metadata */
export interface SkillMetadata {
  emoji?: string;
  requires?: SkillRequirements;
  toolSchemas?: ToolSchema[];
  [key: string]: unknown; // Extra metadata fields
}

/** Skill requirements */
export interface SkillRequirements {
  anyBins?: string[];
  allBins?: string[];
  apiKeys?: string[];
  env?: string[];
}

/** Tool schema definition */
export interface ToolSchema {
  name: string;
  description: string;
  parameters: unknown; // JSON Schema
}

/** Skill source location */
export type SkillSource = "workspace" | "global" | "bundled";

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
