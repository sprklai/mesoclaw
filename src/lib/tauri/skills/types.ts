/**
 * Skill system types matching the Rust backend definitions.
 */

/** Source of a skill definition */
export type SkillSource = "Embedded" | "Local" | "Remote";

/** Lightweight skill information for UI display */
export interface SkillInfo {
  id: string;
  name: string;
  description: string;
  category: string;
  defaultEnabled: boolean;
  source: SkillSource;
}

/** Full skill definition */
export interface SkillDefinition {
  id: string;
  version: string;
  name: string;
  description: string;
  feature: FeatureConfig;
  requires: SkillRequirements;
  triggers: SkillTriggers;
  compose: ComposeConfig;
  promptContent: string;
}

/** Feature configuration */
export interface FeatureConfig {
  category: string;
  defaultEnabled: boolean;
}

/** Skill requirements */
export interface SkillRequirements {
  context: string[];
  tools: string[];
}

/** Skill triggers */
export interface SkillTriggers {
  taskTypes: string[];
  entityTypes: string[];
}

/** Composition configuration */
export interface ComposeConfig {
  extends?: string;
  compatibleWith: string[];
  conflictsWith: string[];
  priority: number;
  mode: ComposeMode;
  promptPosition: PromptPosition;
}

export type ComposeMode = "Merge" | "Chain" | "Parallel";
export type PromptPosition = "Prepend" | "Append" | "Replace";

/** Per-skill user configuration */
export interface SkillUserConfig {
  enabled: boolean;
  priorityOverride?: number;
}

/** Complete skill settings for a workspace */
export interface SkillSettings {
  enabledSkills: string[];
  skillConfigs: Record<string, SkillUserConfig>;
  autoSelect: boolean;
}

/** Skill execution output */
export interface SkillOutput {
  skillId: string;
  content: string;
  structuredData?: unknown;
  toolCallsMade: ToolCallRecord[];
  fromCache: boolean;
}

/** Record of a tool call made during execution */
export interface ToolCallRecord {
  toolName: string;
  parameters: unknown;
  result: string;
  success: boolean;
}
