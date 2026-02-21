/**
 * Agent configuration types for multi-agent system.
 *
 * These types match the backend AgentConfig and related structures
 * from src-tauri/src/agent/loop_.rs
 */

/**
 * Tool access profile for agents.
 * Controls which tools are available to the agent.
 */
export type ToolProfile = "minimal" | "coding" | "messaging" | "full";

/**
 * Agent status for UI display.
 */
export type AgentStatus = "idle" | "running" | "paused" | "error" | "completed";

/**
 * Runtime configuration for an agent.
 * Mirrors the Rust AgentConfig struct.
 */
export interface AgentConfig {
  /** Unique identifier for this agent configuration */
  id: string;
  /** Display name for the agent */
  name: string;
  /** Role description (e.g., "frontend-developer", "architect") */
  role: string;
  /** System prompt / instructions for the agent */
  systemPrompt: string;
  /** LLM provider ID */
  providerId: string;
  /** LLM model identifier (e.g., "openai/gpt-4o") */
  modelId: string;
  /** Sampling temperature (0.0 - 2.0) */
  temperature: number;
  /** Maximum tokens per LLM response */
  maxTokens: number;
  /** Maximum number of tool-call iterations before aborting */
  maxIterations: number;
  /** Maximum number of messages to keep in context */
  maxHistory: number;
  /** Tool access profile */
  toolProfile: ToolProfile;
  /** Whether this agent is enabled */
  isEnabled: boolean;
  /** Creation timestamp */
  createdAt: number;
  /** Last update timestamp */
  updatedAt: number;
}

/**
 * Default values for agent configuration.
 */
export const DEFAULT_AGENT_CONFIG: Partial<AgentConfig> = {
  temperature: 0.7,
  maxTokens: 4096,
  maxIterations: 20,
  maxHistory: 50,
  toolProfile: "full",
  isEnabled: true,
};

/**
 * Agent workspace file types.
 */
export type WorkspaceFileType = "soul" | "agents" | "scratchpad" | "instructions";

/**
 * Workspace file metadata.
 */
export interface WorkspaceFile {
  type: WorkspaceFileType;
  filename: string;
  path: string;
  content: string;
  lastModified: number;
}

/**
 * Agent session summary for history.
 */
export interface AgentSessionSummary {
  id: string;
  agentId: string;
  agentName: string;
  status: AgentStatus;
  startedAt: number;
  completedAt?: number;
  messageCount: number;
  tokenUsage?: number;
  finalMessage?: string;
}

/**
 * Agent run details for execution monitoring.
 */
export interface AgentRun {
  id: string;
  sessionId: string;
  agentId: string;
  agentName: string;
  status: AgentStatus;
  startedAt: number;
  completedAt?: number;
  iterations: number;
  currentTool?: string;
  toolCalls: ToolCallRecord[];
}

/**
 * Tool call record for execution history.
 */
export interface ToolCallRecord {
  id: string;
  toolName: string;
  args: Record<string, unknown>;
  result?: string;
  status: "pending" | "running" | "success" | "error";
  startedAt: number;
  completedAt?: number;
}

/**
 * Create agent request payload.
 */
export interface CreateAgentRequest {
  name: string;
  role: string;
  systemPrompt: string;
  providerId: string;
  modelId: string;
  temperature?: number;
  maxTokens?: number;
  maxIterations?: number;
  maxHistory?: number;
  toolProfile?: ToolProfile;
}

/**
 * Update agent request payload.
 */
export interface UpdateAgentRequest {
  id: string;
  name?: string;
  role?: string;
  systemPrompt?: string;
  providerId?: string;
  modelId?: string;
  temperature?: number;
  maxTokens?: number;
  maxIterations?: number;
  maxHistory?: number;
  toolProfile?: ToolProfile;
  isEnabled?: boolean;
}
