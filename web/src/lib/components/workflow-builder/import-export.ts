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

/** Validate that the content looks like a workflow TOML (basic sanity check) */
export function validateWorkflowToml(content: string): {
  valid: boolean;
  error?: string;
} {
  // Basic checks - the backend does full validation
  if (!content.trim()) {
    return { valid: false, error: "wb_import_error_empty" };
  }
  // Must have at least an id and name field
  if (!content.includes("id =") && !content.includes("id=")) {
    return { valid: false, error: "wb_import_error_no_id" };
  }
  if (!content.includes("name =") && !content.includes("name=")) {
    return { valid: false, error: "wb_import_error_no_name" };
  }
  // Must have at least one step
  if (!content.includes("[[steps]]")) {
    return { valid: false, error: "wb_import_error_no_steps" };
  }
  return { valid: true };
}
