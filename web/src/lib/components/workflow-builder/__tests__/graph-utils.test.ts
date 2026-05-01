import { describe, it, expect } from "vitest";
import type { Node, Edge } from "@xyflow/svelte";

import {
  validateGraph,
  graphToWorkflow,
  workflowToGraph,
  type Workflow,
  type WorkflowMeta,
} from "../graph-utils";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeNode(
  id: string,
  definitionType: string,
  extras: Record<string, unknown> = {},
): Node {
  return {
    id,
    type: "standard",
    position: { x: 0, y: 0 },
    data: { definitionType, stepName: id, ...extras },
  };
}

function makeEdge(source: string, target: string, handle?: string): Edge {
  return {
    id: `e-${source}-${target}`,
    source,
    target,
    sourceHandle: handle,
  } as Edge;
}

const META: WorkflowMeta = {
  id: "test-wf",
  name: "Test Workflow",
  description: "desc",
  schedule: null,
};

// ---------------------------------------------------------------------------
// validateGraph — cycle detection
// ---------------------------------------------------------------------------

describe("validateGraph — cycle detection", () => {
  it("throws on a direct self-loop", () => {
    const nodes = [makeNode("a", "llm"), makeNode("b", "llm")];
    // a → b → a
    const edges = [makeEdge("a", "b"), makeEdge("b", "a")];
    expect(() => validateGraph(nodes, edges)).toThrow(
      "cycle detected in workflow graph",
    );
  });

  it("throws on a three-node cycle", () => {
    const nodes = [
      makeNode("a", "llm"),
      makeNode("b", "llm"),
      makeNode("c", "llm"),
    ];
    const edges = [makeEdge("a", "b"), makeEdge("b", "c"), makeEdge("c", "a")];
    expect(() => validateGraph(nodes, edges)).toThrow(
      "cycle detected in workflow graph",
    );
  });

  it("does not throw on a valid DAG", () => {
    const nodes = [makeNode("a", "llm"), makeNode("b", "llm")];
    const edges = [makeEdge("a", "b")];
    expect(() => validateGraph(nodes, edges)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// validateGraph — orphan detection
// ---------------------------------------------------------------------------

describe("validateGraph — orphan detection", () => {
  it("throws when a non-trigger node has no edges", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("step_a", "llm"),
      makeNode("orphan", "llm"), // no edges at all
    ];
    const edges = [makeEdge("trigger", "step_a")];
    expect(() => validateGraph(nodes, edges)).toThrow(
      "orphan node detected: orphan",
    );
  });

  it("does not throw for trigger nodes with no incoming edges", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("step_a", "llm"),
    ];
    const edges = [makeEdge("trigger", "step_a")];
    expect(() => validateGraph(nodes, edges)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// validateGraph — condition node branch validation
// ---------------------------------------------------------------------------

describe("validateGraph — condition branch validation", () => {
  it("throws when condition node has missing if_true", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("cond", "condition", { if_true: "", if_false: "step_b" }),
      makeNode("step_b", "llm"),
    ];
    const edges = [
      makeEdge("trigger", "cond"),
      makeEdge("cond", "step_b", "false"),
    ];
    expect(() => validateGraph(nodes, edges)).toThrow(
      'condition node "cond" requires both true and false branches',
    );
  });

  it("throws when condition node has missing if_false", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("cond", "condition", { if_true: "step_a", if_false: "" }),
      makeNode("step_a", "llm"),
    ];
    const edges = [
      makeEdge("trigger", "cond"),
      makeEdge("cond", "step_a", "true"),
    ];
    expect(() => validateGraph(nodes, edges)).toThrow(
      'condition node "cond" requires both true and false branches',
    );
  });

  it("does not throw when condition node has both branches set", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("cond", "condition", {
        if_true: "step_a",
        if_false: "step_b",
        expression: "x > 0",
      }),
      makeNode("step_a", "llm"),
      makeNode("step_b", "llm"),
    ];
    const edges = [
      makeEdge("trigger", "cond"),
      makeEdge("cond", "step_a", "true"),
      makeEdge("cond", "step_b", "false"),
    ];
    expect(() => validateGraph(nodes, edges)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// validateGraph — duplicate step names
// ---------------------------------------------------------------------------

describe("validateGraph — step name uniqueness", () => {
  it("throws on duplicate step names", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      // Two nodes with same stepName
      { ...makeNode("step_a", "llm"), data: { definitionType: "llm", stepName: "my_step" } } as Node,
      { ...makeNode("step_b", "llm"), data: { definitionType: "llm", stepName: "my_step" } } as Node,
    ];
    const edges = [makeEdge("trigger", "step_a"), makeEdge("trigger", "step_b")];
    expect(() => validateGraph(nodes, edges)).toThrow(
      "duplicate step name: my_step",
    );
  });

  it("does not throw when all step names are unique", () => {
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("step_a", "llm"),
      makeNode("step_b", "llm"),
    ];
    const edges = [makeEdge("trigger", "step_a"), makeEdge("trigger", "step_b")];
    expect(() => validateGraph(nodes, edges)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Reference cleanup on delete
// ---------------------------------------------------------------------------

describe("reference cleanup on delete — graphToWorkflow preserves cleaned state", () => {
  it("depends_on is empty after referenced step is removed", () => {
    // Simulate a graph where step_b depended on step_a, then step_a was removed
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("step_b", "llm"),
    ];
    const edges = [makeEdge("trigger", "step_b")];
    const wf = graphToWorkflow(nodes, edges, META);
    const stepB = wf.steps.find((s) => s.name === "step_b");
    expect(stepB?.depends_on).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Reference cleanup on rename
// ---------------------------------------------------------------------------

describe("reference cleanup on rename", () => {
  it("condition if_true references are updated after rename", () => {
    // After renaming step_a → renamed_step, condition.if_true should be "renamed_step"
    const nodes = [
      makeNode("trigger", "trigger_manual"),
      makeNode("cond", "condition", {
        if_true: "renamed_step",
        if_false: "step_b",
        expression: "x > 0",
      }),
      makeNode("renamed_step", "llm"),
      makeNode("step_b", "llm"),
    ];
    const edges = [
      makeEdge("trigger", "cond"),
      makeEdge("cond", "renamed_step", "true"),
      makeEdge("cond", "step_b", "false"),
    ];
    const wf = graphToWorkflow(nodes, edges, META);
    const cond = wf.steps.find((s) => s.name === "cond");
    expect(cond?.if_true).toBe("renamed_step");
    expect(cond?.if_false).toBe("step_b");
  });
});

// ---------------------------------------------------------------------------
// Wiki node round-trip
// ---------------------------------------------------------------------------

describe("wiki node round-trip", () => {
  const wikiWorkflow: Workflow = {
    id: "wiki-test",
    name: "Wiki Test",
    description: "test",
    schedule: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    steps: [
      {
        name: "search_wiki",
        type: "tool",
        tool: "wiki",
        depends_on: [],
        args: {
          action: "search",
          query: "typescript generics",
          limit: 5,
        },
      },
    ],
  };

  it("workflowToGraph preserves wiki fields in node data", () => {
    const { nodes } = workflowToGraph(wikiWorkflow);
    const wikiNode = nodes.find((n) => n.id === "search_wiki");
    expect(wikiNode).toBeDefined();
    const data = wikiNode!.data as Record<string, unknown>;
    expect(data.action).toBe("search");
    expect(data.query).toBe("typescript generics");
    expect(data.limit).toBe(5);
    expect(data.definitionType).toBe("wiki");
  });

  it("graphToWorkflow round-trips wiki fields correctly", () => {
    const { nodes, edges } = workflowToGraph(wikiWorkflow);
    const meta: WorkflowMeta = {
      id: "wiki-test",
      name: "Wiki Test",
      description: "test",
      schedule: null,
    };
    const result = graphToWorkflow(nodes, edges, meta);
    const step = result.steps.find((s) => s.name === "search_wiki");
    expect(step).toBeDefined();
    expect(step!.type).toBe("tool");
    expect(step!.tool).toBe("wiki");
    expect(step!.args).toEqual({
      action: "search",
      query: "typescript generics",
      limit: 5,
    });
  });

  it("deep equality: wiki workflow survives workflowToGraph → graphToWorkflow round-trip", () => {
    const { nodes, edges } = workflowToGraph(wikiWorkflow);
    const meta: WorkflowMeta = {
      id: wikiWorkflow.id,
      name: wikiWorkflow.name,
      description: wikiWorkflow.description,
      schedule: wikiWorkflow.schedule,
    };
    const result = graphToWorkflow(nodes, edges, meta);
    expect(result.steps).toHaveLength(1);
    const orig = wikiWorkflow.steps[0];
    const rt = result.steps[0];
    expect(rt.name).toBe(orig.name);
    expect(rt.type).toBe(orig.type);
    expect(rt.tool).toBe(orig.tool);
    expect(rt.args).toEqual(orig.args);
    expect(rt.depends_on).toEqual(orig.depends_on);
  });

  it("workflowToGraph handles wiki get action with slug field", () => {
    const wf: Workflow = {
      ...wikiWorkflow,
      steps: [
        {
          name: "get_page",
          type: "tool",
          tool: "wiki",
          depends_on: [],
          args: { action: "get", slug: "my-page" },
        },
      ],
    };
    const { nodes } = workflowToGraph(wf);
    const node = nodes.find((n) => n.id === "get_page");
    const data = node!.data as Record<string, unknown>;
    expect(data.action).toBe("get");
    expect(data.slug).toBe("my-page");
    expect(data.query).toBeUndefined();
  });

  it("workflowToGraph handles wiki query action with question field", () => {
    const wf: Workflow = {
      ...wikiWorkflow,
      steps: [
        {
          name: "query_wiki",
          type: "tool",
          tool: "wiki",
          depends_on: [],
          args: { action: "query", question: "What is X?", limit: 3 },
        },
      ],
    };
    const { nodes } = workflowToGraph(wf);
    const node = nodes.find((n) => n.id === "query_wiki");
    const data = node!.data as Record<string, unknown>;
    expect(data.action).toBe("query");
    expect(data.question).toBe("What is X?");
    expect(data.limit).toBe(3);
  });
});
