/**
 * Tests for TOML serialization correctness and round-trip bugs fixed in:
 * - Issue 1: nested args throw at save time
 * - Issue 2: timeout_secs = 0 preserved in TOML output
 * - Issue 3: retry with one missing sub-field not written to TOML
 * - Issue 4: created_at preserved from meta; updated_at always refreshed
 * - Issue 6: schema_version = 1 written in TOML output
 */
import { describe, it, expect } from "vitest";
import {
  workflowToToml,
  graphToWorkflow,
  type Workflow,
  type WorkflowMeta,
  type WorkflowStep,
} from "./graph-utils";
import { validateWorkflowToml } from "./import-export";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeStep(overrides: Partial<WorkflowStep> = {}): WorkflowStep {
  return {
    name: "step1",
    type: "tool",
    tool: "shell",
    depends_on: [],
    args: { command: "echo hi" },
    ...overrides,
  };
}

function makeWorkflow(overrides: Partial<Workflow> = {}): Workflow {
  return {
    id: "wf-test",
    name: "Test Workflow",
    description: "A workflow for testing",
    schedule: null,
    steps: [makeStep()],
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
    schema_version: 1,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Issue 2: timeout_secs = 0 must NOT be dropped
// ---------------------------------------------------------------------------

describe("workflowToToml — timeout_secs = 0 preserved (Issue 2)", () => {
  it("emits timeout_secs when value is 0", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ timeout_secs: 0 })],
    });
    const toml = workflowToToml(wf);
    expect(toml).toContain("timeout_secs = 0");
  });

  it("emits timeout_secs when value is a positive number", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ timeout_secs: 30 })],
    });
    const toml = workflowToToml(wf);
    expect(toml).toContain("timeout_secs = 30");
  });

  it("does not emit timeout_secs when undefined", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ timeout_secs: undefined })],
    });
    const toml = workflowToToml(wf);
    expect(toml).not.toContain("timeout_secs");
  });
});

// ---------------------------------------------------------------------------
// Issue 1: nested args must throw
// ---------------------------------------------------------------------------

describe("workflowToToml — nested args throw (Issue 1)", () => {
  it("throws when args contain a nested object", () => {
    const wf = makeWorkflow({
      steps: [
        makeStep({
          args: { config: { nested: "value" } } as Record<string, unknown>,
        }),
      ],
    });
    expect(() => workflowToToml(wf)).toThrow(
      /tool step args must be flat key-value pairs/,
    );
  });

  it("throws and names the offending key", () => {
    const wf = makeWorkflow({
      steps: [
        makeStep({
          args: { options: { deep: true } } as Record<string, unknown>,
        }),
      ],
    });
    expect(() => workflowToToml(wf)).toThrow(/key: "options"/);
  });

  it("does not throw for flat string args", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ args: { command: "ls -la", flag: "true" } })],
    });
    expect(() => workflowToToml(wf)).not.toThrow();
  });

  it("does not throw for flat number and boolean args", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ args: { max_results: 5, recursive: true } })],
    });
    expect(() => workflowToToml(wf)).not.toThrow();
  });

  it("does not throw for simple array args", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ args: { tags: ["a", "b", "c"] } })],
    });
    expect(() => workflowToToml(wf)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Issue 3: retry with one missing sub-field must NOT emit undefined
// ---------------------------------------------------------------------------

describe("workflowToToml — retry with missing sub-field not written (Issue 3)", () => {
  it("does not emit retry when max_retries is undefined", () => {
    const wf = makeWorkflow({
      steps: [
        makeStep({
          retry: { max_retries: undefined as unknown as number, retry_delay_ms: 1000 },
        }),
      ],
    });
    const toml = workflowToToml(wf);
    expect(toml).not.toContain("retry");
    expect(toml).not.toContain("undefined");
  });

  it("does not emit retry when retry_delay_ms is undefined", () => {
    const wf = makeWorkflow({
      steps: [
        makeStep({
          retry: { max_retries: 3, retry_delay_ms: undefined as unknown as number },
        }),
      ],
    });
    const toml = workflowToToml(wf);
    expect(toml).not.toContain("retry");
    expect(toml).not.toContain("undefined");
  });

  it("emits retry when both sub-fields are defined numbers", () => {
    const wf = makeWorkflow({
      steps: [makeStep({ retry: { max_retries: 3, retry_delay_ms: 500 } })],
    });
    const toml = workflowToToml(wf);
    expect(toml).toContain("retry = { max_retries = 3, retry_delay_ms = 500 }");
  });
});

// ---------------------------------------------------------------------------
// Issue 4: created_at preserved; updated_at refreshed
// ---------------------------------------------------------------------------

describe("graphToWorkflow — created_at preserved (Issue 4)", () => {
  it("preserves created_at from meta when provided", () => {
    const original = "2025-06-15T12:00:00.000Z";
    const meta: WorkflowMeta = {
      id: "wf-1",
      name: "My Workflow",
      description: "desc",
      schedule: null,
      created_at: original,
    };
    const wf = graphToWorkflow([], [], meta);
    expect(wf.created_at).toBe(original);
  });

  it("sets created_at to now when absent from meta", () => {
    const before = new Date().toISOString();
    const meta: WorkflowMeta = {
      id: "wf-new",
      name: "New",
      description: "desc",
      schedule: null,
    };
    const wf = graphToWorkflow([], [], meta);
    const after = new Date().toISOString();
    expect(wf.created_at >= before).toBe(true);
    expect(wf.created_at <= after).toBe(true);
  });

  it("always updates updated_at to a current timestamp", () => {
    const original = "2025-06-15T12:00:00.000Z";
    const before = new Date().toISOString();
    const meta: WorkflowMeta = {
      id: "wf-1",
      name: "My Workflow",
      description: "desc",
      schedule: null,
      created_at: original,
    };
    const wf = graphToWorkflow([], [], meta);
    expect(wf.updated_at > original).toBe(true);
    expect(wf.updated_at >= before).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Issue 6: schema_version = 1 in TOML output
// ---------------------------------------------------------------------------

describe("workflowToToml — schema_version written (Issue 6)", () => {
  it("writes schema_version = 1 when set on workflow", () => {
    const wf = makeWorkflow({ schema_version: 1 });
    const toml = workflowToToml(wf);
    expect(toml).toContain("schema_version = 1");
  });

  it("defaults to schema_version = 1 when field is absent", () => {
    const wf = makeWorkflow({ schema_version: undefined });
    const toml = workflowToToml(wf);
    expect(toml).toContain("schema_version = 1");
  });
});

// ---------------------------------------------------------------------------
// Issue 5: validateWorkflowToml rejects empty steps array
// ---------------------------------------------------------------------------

describe("validateWorkflowToml — rejects empty steps (Issue 5)", () => {
  it("returns invalid when steps array is empty", () => {
    // smol-toml parses this; array is present but empty
    const toml = `id = "wf"\nname = "Test"\ndescription = "d"\n`;
    const result = validateWorkflowToml(toml);
    expect(result.valid).toBe(false);
    expect(result.error).toBe("wb_import_error_no_steps");
  });

  it("returns invalid for broken TOML syntax", () => {
    const toml = `id = "wf"\nname = [[[unclosed`;
    const result = validateWorkflowToml(toml);
    expect(result.valid).toBe(false);
    expect(result.error).toMatch(/wb_import_error_parse/);
  });

  it("returns invalid when id is missing", () => {
    const toml = `name = "Test"\n[[steps]]\nname = "s1"\ntype = "delay"\nseconds = 1\n`;
    const result = validateWorkflowToml(toml);
    expect(result.valid).toBe(false);
    expect(result.error).toBe("wb_import_error_no_id");
  });

  it("returns invalid when name is missing", () => {
    const toml = `id = "wf"\n[[steps]]\nname = "s1"\ntype = "delay"\nseconds = 1\n`;
    const result = validateWorkflowToml(toml);
    expect(result.valid).toBe(false);
    expect(result.error).toBe("wb_import_error_no_name");
  });

  it("returns valid for a well-formed workflow TOML", () => {
    const toml = [
      `id = "daily"`,
      `name = "Daily Report"`,
      `description = "desc"`,
      `[[steps]]`,
      `name = "fetch"`,
      `type = "tool"`,
      `tool = "web_search"`,
    ].join("\n");
    const result = validateWorkflowToml(toml);
    expect(result.valid).toBe(true);
  });
});
