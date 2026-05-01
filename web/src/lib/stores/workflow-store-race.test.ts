/**
 * Tests for UI state race condition fixes in workflow stores.
 *
 * Issue 1: suppressDirty synchronous flag — isDirty is false during backend load, true after
 * Issue 2: isSaving guard — double-calling update() concurrently rejects second call
 * Issue 3: codeContent derived value — updates when nodes change (logic tested via graph-utils)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// ---------------------------------------------------------------------------
// Issue 1: _loadingFromBackend synchronous suppression
// ---------------------------------------------------------------------------
describe("workflow-builder: _loadingFromBackend synchronous flag", () => {
  it("isDirty is false immediately after loadWorkflow", async () => {
    // Import fresh instance for each test by testing the logic directly.
    // The store's loadWorkflow sets _loadingFromBackend = true, updates state,
    // then sets _loadingFromBackend = false — all synchronously.
    // We verify there is no setTimeout that could race.

    const { builderStore } = await import("./workflow-builder.svelte");

    builderStore.loadWorkflow(
      { id: "wf-1", name: "Test", description: "", schedule: null },
      [],
      [],
    );

    // isDirty must be false synchronously — no timer needed
    expect(builderStore.isDirty).toBe(false);
  });

  it("isDirty is false immediately after reset", async () => {
    const { builderStore } = await import("./workflow-builder.svelte");

    builderStore.reset();

    expect(builderStore.isDirty).toBe(false);
  });

  it("isDirty is false immediately after markSaved", async () => {
    const { builderStore } = await import("./workflow-builder.svelte");

    // First make it dirty
    builderStore.reset();
    builderStore.updateMeta({ name: "changed" });
    expect(builderStore.isDirty).toBe(true);

    builderStore.markSaved("wf-2");

    // Must be cleared synchronously — no 150ms timer
    expect(builderStore.isDirty).toBe(false);
  });

  it("isDirty becomes true after a user edit following loadWorkflow", async () => {
    const { builderStore } = await import("./workflow-builder.svelte");

    builderStore.loadWorkflow(
      { id: "wf-3", name: "Test", description: "", schedule: null },
      [],
      [],
    );
    expect(builderStore.isDirty).toBe(false);

    // User edits metadata
    builderStore.updateMeta({ name: "New name" });
    expect(builderStore.isDirty).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Issue 2: isSaving guard in workflows store
// ---------------------------------------------------------------------------

// We must mock the API client before importing the store
vi.mock("$lib/api/client", () => ({
  apiGet: vi.fn(),
  apiGetText: vi.fn(),
  apiPost: vi.fn(),
  apiPut: vi.fn(),
  apiDelete: vi.fn(),
}));

describe("workflows store: isSaving concurrent save guard", () => {
  beforeEach(async () => {
    const client = await import("$lib/api/client");
    vi.mocked(client.apiPost).mockReset();
    vi.mocked(client.apiPut).mockReset();
    vi.mocked(client.apiGet).mockReset();
  });

  it("second call to create() while first is in flight is rejected", async () => {
    const client = await import("$lib/api/client");
    const { workflowsStore } = await import("./workflows.svelte");

    // Make apiPost never resolve so first call stays in flight
    let resolveFirst!: (v: unknown) => void;
    vi.mocked(client.apiPost).mockReturnValueOnce(
      new Promise((res) => {
        resolveFirst = res;
      }),
    );
    vi.mocked(client.apiGet).mockResolvedValue([]);

    // Start first create — do NOT await
    const first = workflowsStore.create("[name]\nfoo = 1");

    // Immediately attempt second create while first is in flight
    const secondResult = workflowsStore.create("[name]\nbar = 2");

    // Second call must reject synchronously (isSaving = true)
    await expect(secondResult).rejects.toThrow("Save already in progress");

    // Resolve first so test cleanup is clean
    resolveFirst({ id: "wf-1", name: "T", description: "", schedule: null, steps: [], layout: {}, created_at: "", updated_at: "" });
    await expect(first).resolves.toBeDefined();
  });

  it("second call to update() while first is in flight is rejected", async () => {
    const client = await import("$lib/api/client");
    const { workflowsStore } = await import("./workflows.svelte");

    let resolveFirst!: (v: unknown) => void;
    vi.mocked(client.apiPut).mockReturnValueOnce(
      new Promise((res) => {
        resolveFirst = res;
      }),
    );
    vi.mocked(client.apiGet).mockResolvedValue([]);

    const first = workflowsStore.update("wf-1", "[name]\nfoo = 1");

    // Second update while first is in flight
    const secondResult = workflowsStore.update("wf-1", "[name]\nbar = 2");

    await expect(secondResult).rejects.toThrow("Save already in progress");

    resolveFirst({ id: "wf-1", name: "T", description: "", schedule: null, steps: [], layout: {}, created_at: "", updated_at: "" });
    await expect(first).resolves.toBeDefined();
  });

  it("isSaving is false after create() completes", async () => {
    const client = await import("$lib/api/client");
    const { workflowsStore } = await import("./workflows.svelte");

    const wf = { id: "wf-2", name: "T", description: "", schedule: null, steps: [], layout: {}, created_at: "", updated_at: "" };
    vi.mocked(client.apiPost).mockResolvedValueOnce(wf);
    vi.mocked(client.apiGet).mockResolvedValue([wf]);

    await workflowsStore.create("[name]\nfoo = 1");

    expect(workflowsStore.isSaving).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Issue 3: codeContent $derived — logic verification via graph-utils
// ---------------------------------------------------------------------------
describe("graphToWorkflow + workflowToToml: live TOML generation", () => {
  it("produces valid TOML from a single-node graph", async () => {
    const { graphToWorkflow, workflowToToml } = await import(
      "$lib/components/workflow-builder/graph-utils"
    );

    const nodes = [
      {
        id: "step-1",
        type: "standard",
        position: { x: 0, y: 0 },
        data: {
          definitionType: "shell",
          stepName: "step-1",
          command: "echo hello",
        },
      },
    ];

    const wf = graphToWorkflow(nodes as never, [], {
      name: "Test WF",
      description: "desc",
      schedule: null,
    });

    const toml = workflowToToml(wf);
    expect(toml).toContain('name = "Test WF"');
    expect(toml).toContain("[[steps]]");
    expect(toml).toContain('name = "step-1"');
  });

  it("re-generating TOML after adding a node reflects the new node", async () => {
    const { graphToWorkflow, workflowToToml } = await import(
      "$lib/components/workflow-builder/graph-utils"
    );

    const meta = { name: "WF", description: "", schedule: null };

    // Start with one node
    const nodes1 = [
      {
        id: "a",
        type: "standard",
        position: { x: 0, y: 0 },
        data: { definitionType: "shell", stepName: "a", command: "ls" },
      },
    ];
    const toml1 = workflowToToml(graphToWorkflow(nodes1 as never, [], meta));
    expect(toml1).toContain('"a"');

    // Add a second node
    const nodes2 = [
      ...nodes1,
      {
        id: "b",
        type: "standard",
        position: { x: 200, y: 0 },
        data: { definitionType: "shell", stepName: "b", command: "pwd" },
      },
    ];
    const toml2 = workflowToToml(graphToWorkflow(nodes2 as never, [], meta));
    expect(toml2).toContain('"a"');
    expect(toml2).toContain('"b"');
    // Second node not present in first TOML
    expect(toml1).not.toContain('"b"');
  });
});
