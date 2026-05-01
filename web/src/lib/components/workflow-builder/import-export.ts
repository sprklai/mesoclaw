import { parse as parseToml } from "smol-toml";

/** Download a workflow as a .toml file */
export function exportWorkflowToml(
  rawToml: string,
  workflowName: string,
): void {
  const blob = new Blob([rawToml], { type: "application/toml" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${workflowName.replace(/[^a-zA-Z0-9_-]/g, "_")}.toml`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/** Read a .toml file from a File input, return the raw text */
export async function readWorkflowFile(file: File): Promise<string> {
  return file.text();
}

/**
 * Validate that the content is a well-formed workflow TOML.
 *
 * Issue 5: Uses smol-toml to parse the TOML rather than naive string matching,
 * so structurally broken TOML is caught early rather than silently passed to
 * the backend. Validates:
 * - Parseable as valid TOML
 * - `id` is a non-empty string
 * - `name` is a non-empty string
 * - `steps` is a non-empty array
 */
export function validateWorkflowToml(content: string): {
  valid: boolean;
  error?: string;
} {
  if (!content.trim()) {
    return { valid: false, error: "wb_import_error_empty" };
  }

  let parsed: Record<string, unknown>;
  try {
    parsed = parseToml(content) as Record<string, unknown>;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { valid: false, error: `wb_import_error_parse: ${msg}` };
  }

  if (typeof parsed.id !== "string" || parsed.id.trim() === "") {
    return { valid: false, error: "wb_import_error_no_id" };
  }

  if (typeof parsed.name !== "string" || parsed.name.trim() === "") {
    return { valid: false, error: "wb_import_error_no_name" };
  }

  if (!Array.isArray(parsed.steps) || parsed.steps.length === 0) {
    return { valid: false, error: "wb_import_error_no_steps" };
  }

  return { valid: true };
}
