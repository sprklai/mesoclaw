/**
 * Extracts a human-readable error message from any error type.
 *
 * Handles:
 * - Standard Error instances
 * - Plain strings (common from Tauri commands)
 * - Objects with `message` or `error` properties
 * - Arbitrary objects (JSON stringified)
 */
export function extractErrorMessage(error: unknown): string {
  // Handle standard Error instances
  if (error instanceof Error) {
    return error.message;
  }

  // Handle plain strings (most common from Tauri commands)
  if (typeof error === "string") {
    return error;
  }

  // Handle error objects with message property
  if (error && typeof error === "object") {
    const obj = error as Record<string, unknown>;
    if ("message" in obj && typeof obj.message === "string") {
      return obj.message;
    }
    if ("error" in obj && typeof obj.error === "string") {
      return obj.error;
    }
    // Try JSON stringify for other objects
    try {
      return JSON.stringify(error);
    } catch {
      return "Unknown error occurred";
    }
  }

  return "Unknown error occurred";
}
