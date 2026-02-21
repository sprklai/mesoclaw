/**
 * Task Classification Utilities
 *
 * Simple heuristic-based task classification for routing decisions.
 * This mirrors the backend classification logic for frontend use.
 */

import type { TaskType } from "@/stores/routerStore";

/**
 * Keywords that indicate each task type
 */
const TASK_KEYWORDS: Record<TaskType, string[]> = {
  code: [
    "code",
    "debug",
    "implement",
    "function",
    "class",
    "bug",
    "fix",
    "error",
    "compile",
    "syntax",
    "variable",
    "method",
    "refactor",
    "api",
    "script",
    "program",
    "algorithm",
  ],
  analysis: [
    "analyze",
    "compare",
    "summarize",
    "explain",
    "why",
    "how does",
    "what is",
    "difference",
    "pros and cons",
    "evaluate",
    "review",
    "investigate",
    "research",
  ],
  creative: [
    "write",
    "create",
    "design",
    "brainstorm",
    "idea",
    "story",
    "poem",
    "blog",
    "article",
    "content",
    "marketing",
    "creative",
    "imagine",
  ],
  fast: [], // Fast is determined by message length, not keywords
  general: [], // Default fallback
  other: [], // Explicitly marked as other
};

/**
 * Classify a message into a task type
 *
 * @param input The user message to classify
 * @returns The classified task type
 */
export function classifyTask(input: string): TaskType {
  const lower = input.toLowerCase().trim();

  // Check for code indicators
  if (TASK_KEYWORDS.code.some((keyword) => lower.includes(keyword))) {
    return "code";
  }

  // Check for analysis indicators
  if (TASK_KEYWORDS.analysis.some((keyword) => lower.includes(keyword))) {
    return "analysis";
  }

  // Check for creative indicators
  if (TASK_KEYWORDS.creative.some((keyword) => lower.includes(keyword))) {
    return "creative";
  }

  // Fast indicators (short queries without complex questions)
  if (input.length < 50 && !lower.includes("?")) {
    return "fast";
  }

  // Check for questions that might be general
  if (
    lower.includes("?") ||
    lower.includes("help") ||
    lower.includes("please")
  ) {
    return "general";
  }

  // Default to general
  return "general";
}

/**
 * Get a human-readable description of why a task was classified
 *
 * @param input The user message
 * @param task The classified task type
 * @returns A description of the classification reason
 */
export function getClassificationReason(input: string, task: TaskType): string {
  // Note: input is available for future enhancements
  void input;

  switch (task) {
    case "code":
      return "Detected programming-related keywords";
    case "analysis":
      return "Detected analysis or explanation request";
    case "creative":
      return "Detected creative writing request";
    case "fast":
      return "Short message without complex question";
    case "general":
      return "General conversation query";
    default:
      return "Default classification";
  }
}

/**
 * Task type display names
 */
export const TASK_DISPLAY_NAMES: Record<TaskType, string> = {
  code: "Code & Programming",
  general: "General Conversation",
  fast: "Quick Response",
  creative: "Creative Writing",
  analysis: "Analysis & Research",
  other: "Other",
};

/**
 * Task type icons (emoji)
 */
export const TASK_ICONS: Record<TaskType, string> = {
  code: "💻",
  general: "💬",
  fast: "⚡",
  creative: "🎨",
  analysis: "🔍",
  other: "📝",
};
