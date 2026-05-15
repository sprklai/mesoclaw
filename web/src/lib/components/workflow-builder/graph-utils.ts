import type { Node, Edge } from "@xyflow/svelte";

import { nodeRegistry } from "./node-registry";

export interface WorkflowMeta {
  id?: string;
  name: string;
  description: string;
  schedule: string | null;
  /** Preserved from the original workflow — only set on first creation if absent (Issue 4). */
  created_at?: string;
}

export interface NodePosition {
  x: number;
  y: number;
}

export interface WorkflowLayout {
  [stepName: string]: NodePosition;
}

export interface RetryConfig {
  max_retries: number;
  retry_delay_ms: number;
}

export interface WorkflowStep {
  name: string;
  type: string;
  depends_on: string[];
  tool?: string;
  args?: Record<string, unknown>;
  prompt?: string;
  model?: string;
  seconds?: number;
  expression?: string;
  if_true?: string;
  if_false?: string;
  steps?: string[];
  timeout_secs?: number;
  retry?: RetryConfig;
  failure_policy?: string | { fallback: { step: string } };
}

export interface Workflow {
  id: string;
  name: string;
  description: string;
  schedule: string | null;
  steps: WorkflowStep[];
  layout?: WorkflowLayout;
  created_at: string;
  updated_at: string;
  schema_version?: number;
}

/**
 * Convert a backend Workflow to @xyflow/svelte nodes and edges.
 */
/** Resolve a workflow step to its node-registry type key.
 *  Action-dispatched tools (memory, config) are mapped by tool+action pair. */
function resolveNodeType(step: WorkflowStep): string {
  if (step.type !== "tool") return step.type;
  const tool = step.tool ?? step.type;
  const action = step.args?.action as string | undefined;
  if (tool === "memory" && action) {
    if (action === "store" || action === "update") return "memory_store";
    if (action === "recall") return "memory_recall";
    if (action === "forget") return "memory_forget";
  }
  if (tool === "config" && action) {
    if (action === "get") return "config_read";
    if (action === "update") return "config_update";
  }
  return tool;
}

export function workflowToGraph(workflow: Workflow): {
  nodes: Node[];
  edges: Edge[];
} {
  const nodes: Node[] = workflow.steps.map((step) => {
    const defType = resolveNodeType(step);
    const def = nodeRegistry.get(defType);
    const nodeData = def?.fromStep
      ? def.fromStep(step as unknown as Record<string, unknown>)
      : { ...step };

    nodeData.definitionType = defType;
    nodeData.stepName = step.name;

    if (step.timeout_secs !== undefined)
      nodeData.timeout_secs = step.timeout_secs;
    if (step.retry !== undefined) nodeData.retry = step.retry;
    if (step.failure_policy !== undefined)
      nodeData.failure_policy = step.failure_policy;

    return {
      id: step.name,
      type: def?.visual ?? "standard",
      position: workflow.layout?.[step.name] ?? { x: 0, y: 0 },
      data: nodeData,
    } satisfies Node;
  });

  const edges = deriveEdges(workflow.steps);

  if (!workflow.layout) {
    const laid = autoLayout(nodes, edges);
    for (let i = 0; i < nodes.length; i++) {
      nodes[i].position = laid[i].position;
    }
  }

  return { nodes, edges };
}

/**
 * Validate the graph before converting to a workflow.
 * Throws a descriptive Error if validation fails.
 */
export function validateGraph(nodes: Node[], edges: Edge[]): void {
  // Identify trigger nodes (excluded from step-level validation)
  const triggerNodeIds = new Set(
    nodes
      .filter((n) => {
        const dt = (n.data as Record<string, unknown>).definitionType as
          | string
          | undefined;
        return dt === "trigger_manual" || dt === "trigger_cron";
      })
      .map((n) => n.id),
  );

  const nonTriggerNodes = nodes.filter((n) => !triggerNodeIds.has(n.id));

  // Step name uniqueness
  const stepNames: string[] = nonTriggerNodes.map(
    (n) => ((n.data as Record<string, unknown>).stepName as string) || n.id,
  );
  const seenNames = new Set<string>();
  for (const name of stepNames) {
    if (seenNames.has(name)) {
      throw new Error(`duplicate step name: ${name}`);
    }
    seenNames.add(name);
  }

  // Build adjacency from edges (excluding condition branch handle edges)
  const conditionNodeIds = new Set(
    nodes
      .filter(
        (n) =>
          (n.data as Record<string, unknown>).definitionType === "condition",
      )
      .map((n) => n.id),
  );

  const outgoing = new Map<string, Set<string>>();
  const incoming = new Map<string, Set<string>>();
  for (const n of nodes) {
    outgoing.set(n.id, new Set());
    incoming.set(n.id, new Set());
  }
  for (const edge of edges) {
    outgoing.get(edge.source)?.add(edge.target);
    incoming.get(edge.target)?.add(edge.source);
  }

  // Cycle detection via DFS — detect back edges
  const WHITE = 0,
    GRAY = 1,
    BLACK = 2;
  const color = new Map<string, number>();
  for (const n of nodes) color.set(n.id, WHITE);

  function dfs(id: string): boolean {
    color.set(id, GRAY);
    for (const neighbor of outgoing.get(id) ?? []) {
      if (color.get(neighbor) === GRAY) return true; // back edge = cycle
      if (color.get(neighbor) === WHITE && dfs(neighbor)) return true;
    }
    color.set(id, BLACK);
    return false;
  }

  for (const n of nodes) {
    if (color.get(n.id) === WHITE && dfs(n.id)) {
      throw new Error("cycle detected in workflow graph");
    }
  }

  // Orphan detection: non-trigger nodes with zero incoming AND zero outgoing edges,
  // Condition node branch validation
  for (const n of nodes) {
    if (!conditionNodeIds.has(n.id)) continue;
    const data = n.data as Record<string, unknown>;
    const name = (data.stepName as string) || n.id;
    const ifTrue = data.if_true as string | undefined;
    const ifFalse = data.if_false as string | undefined;
    if (!ifTrue || !ifFalse) {
      throw new Error(
        `condition node "${name}" requires both true and false branches`,
      );
    }
  }
}

/**
 * Convert @xyflow/svelte nodes and edges back to a backend Workflow.
 * Runs graph validation before building — throws Error on invalid graph.
 */
export function graphToWorkflow(
  nodes: Node[],
  edges: Edge[],
  meta: WorkflowMeta,
): Workflow {
  validateGraph(nodes, edges);

  // Build a set of condition node IDs so we can filter their handle edges from depends_on
  const conditionNodeIds = new Set(
    nodes
      .filter(
        (n) =>
          (n.data as Record<string, unknown>).definitionType === "condition",
      )
      .map((n) => n.id),
  );

  // Build incoming dependency edges, excluding condition branch edges (true/false handles)
  const incomingEdges = new Map<string, string[]>();
  for (const edge of edges) {
    // Skip edges from condition node's true/false handles — those are represented by if_true/if_false fields
    if (
      conditionNodeIds.has(edge.source) &&
      (edge.sourceHandle === "true" || edge.sourceHandle === "false")
    ) {
      continue;
    }
    const existing = incomingEdges.get(edge.target) ?? [];
    existing.push(edge.source);
    incomingEdges.set(edge.target, existing);
  }

  // Identify trigger nodes (visual-only, no backend StepType equivalent)
  const triggerNodeIds = new Set(
    nodes
      .filter((n) => {
        const dt = (n.data as Record<string, unknown>).definitionType as
          | string
          | undefined;
        return dt === "trigger_manual" || dt === "trigger_cron";
      })
      .map((n) => n.id),
  );

  const steps: WorkflowStep[] = nodes
    .filter((node) => !triggerNodeIds.has(node.id))
    .map((node) => {
      const data = node.data as Record<string, unknown>;
      const defType = data.definitionType as string | undefined;
      const def = defType ? nodeRegistry.get(defType) : undefined;
      const stepFields: Record<string, unknown> = def?.toStep
        ? def.toStep(data)
        : { ...data };

      const step: WorkflowStep = {
        ...(stepFields as Partial<WorkflowStep>),
        type:
          ((stepFields as Record<string, unknown>).type as string) ?? "tool",
        name: (data.stepName as string) || node.id,
        depends_on: (incomingEdges.get(node.id) ?? []).filter(
          (id) => !triggerNodeIds.has(id),
        ),
      };

      if (data.timeout_secs !== undefined)
        step.timeout_secs = data.timeout_secs as number;

      // Retry: accept RetryConfig object from node data
      if (data.retry !== undefined && data.retry !== null) {
        const r = data.retry as Record<string, unknown>;
        if (typeof r === "object" && r.max_retries !== undefined) {
          step.retry = {
            max_retries: Number(r.max_retries),
            retry_delay_ms: Number(r.retry_delay_ms ?? 1000),
          };
        }
      }

      // Failure policy: accept string or fallback object
      if (data.failure_policy !== undefined) {
        const fp = data.failure_policy;
        if (typeof fp === "string") {
          step.failure_policy = fp === "stop" ? undefined : fp;
        } else {
          step.failure_policy = fp as { fallback: { step: string } };
        }
      }

      return step;
    });

  const layout: WorkflowLayout = {};
  for (const node of nodes) {
    const stepName =
      ((node.data as Record<string, unknown>).stepName as string) || node.id;
    layout[stepName] = { x: node.position.x, y: node.position.y };
  }

  const id = meta.id ?? slugify(meta.name);
  const now = new Date().toISOString();

  return {
    id,
    name: meta.name,
    description: meta.description,
    schedule: meta.schedule,
    steps,
    layout,
    // Preserve existing created_at; only set on first creation (Issue 4)
    created_at: meta.created_at ?? now,
    updated_at: now,
    schema_version: 1,
  };
}

/**
 * Simple topological-sort-based auto-layout for workflow nodes.
 *
 * Assigns nodes to columns based on dependency depth, then spaces
 * them evenly within each column.
 */
export function autoLayout(nodes: Node[], edges: Edge[]): Node[] {
  if (nodes.length === 0) return [];

  const nodeIds = new Set(nodes.map((n) => n.id));
  const incomingMap = new Map<string, Set<string>>();
  const outgoingMap = new Map<string, Set<string>>();

  for (const id of nodeIds) {
    incomingMap.set(id, new Set());
    outgoingMap.set(id, new Set());
  }

  for (const edge of edges) {
    if (nodeIds.has(edge.source) && nodeIds.has(edge.target)) {
      incomingMap.get(edge.target)!.add(edge.source);
      outgoingMap.get(edge.source)!.add(edge.target);
    }
  }

  // Kahn's algorithm for topological ordering with column assignment
  const columnOf = new Map<string, number>();
  const queue: string[] = [];

  for (const id of nodeIds) {
    if (incomingMap.get(id)!.size === 0) {
      queue.push(id);
      columnOf.set(id, 0);
    }
  }

  const sorted: string[] = [];
  while (queue.length > 0) {
    const current = queue.shift()!;
    sorted.push(current);
    const col = columnOf.get(current) ?? 0;

    for (const neighbor of outgoingMap.get(current) ?? []) {
      const incoming = incomingMap.get(neighbor)!;
      incoming.delete(current);

      // Assign the maximum column among all dependencies
      const existingCol = columnOf.get(neighbor) ?? 0;
      columnOf.set(neighbor, Math.max(existingCol, col + 1));

      if (incoming.size === 0) {
        queue.push(neighbor);
      }
    }
  }

  // Any nodes not reached (cycles or isolated) get placed in column 0
  for (const id of nodeIds) {
    if (!columnOf.has(id)) {
      columnOf.set(id, 0);
      sorted.push(id);
    }
  }

  // Group nodes by column
  const columns = new Map<number, string[]>();
  for (const id of sorted) {
    const col = columnOf.get(id) ?? 0;
    const group = columns.get(col) ?? [];
    group.push(id);
    columns.set(col, group);
  }

  // Assign positions
  const X_START = 100;
  const X_GAP = 300;
  const Y_GAP = 150;

  const positionMap = new Map<string, NodePosition>();
  for (const [col, ids] of columns) {
    const x = X_START + col * X_GAP;
    for (let i = 0; i < ids.length; i++) {
      positionMap.set(ids[i], { x, y: i * Y_GAP });
    }
  }

  return nodes.map((node) => ({
    ...node,
    position: positionMap.get(node.id) ?? node.position,
  }));
}

/**
 * Derive edges from workflow step dependency declarations and condition branches.
 */
export function deriveEdges(steps: WorkflowStep[]): Edge[] {
  const edges: Edge[] = [];

  for (const step of steps) {
    // Dependency edges
    for (const dep of step.depends_on) {
      edges.push({
        id: `e-${dep}-${step.name}`,
        source: dep,
        target: step.name,
        animated: false,
        type: "default",
      });
    }

    // Condition branch edges — reconstruct from if_true/if_false fields
    if (step.type === "condition") {
      if (step.if_true) {
        edges.push({
          id: `e-${step.name}-true-${step.if_true}`,
          source: step.name,
          target: step.if_true,
          sourceHandle: "true",
          animated: false,
          type: "default",
        });
      }
      if (step.if_false) {
        edges.push({
          id: `e-${step.name}-false-${step.if_false}`,
          source: step.name,
          target: step.if_false,
          sourceHandle: "false",
          animated: false,
          type: "default",
        });
      }
    }
  }

  return edges;
}

/**
 * Generate a unique step name, appending _1, _2, etc. if the base name is taken.
 */
export function generateStepName(
  baseName: string,
  existingNames: string[],
): string {
  const nameSet = new Set(existingNames);

  if (!nameSet.has(baseName)) {
    return baseName;
  }

  let counter = 1;
  while (nameSet.has(`${baseName}_${counter}`)) {
    counter++;
  }

  return `${baseName}_${counter}`;
}

/**
 * Serialize a Workflow object to TOML string suitable for the backend API.
 *
 * Constraints on step args:
 * - Tool step `args` must be flat key-value pairs only (string, number, boolean,
 *   simple arrays). Nested objects are not supported in TOML inline tables and
 *   will throw at call time (Issue 1).
 */
export function workflowToToml(wf: Workflow): string {
  const lines: string[] = [];
  lines.push(`id = ${tomlStr(wf.id)}`);
  lines.push(`name = ${tomlStr(wf.name)}`);
  lines.push(`description = ${tomlStr(wf.description)}`);
  // Always write schema_version so future schema changes can be detected (Issue 6)
  lines.push(`schema_version = ${wf.schema_version ?? 1}`);
  if (wf.schedule) {
    const parts = wf.schedule.trim().split(/\s+/);
    const normalized = parts.length === 5 ? `0 ${wf.schedule.trim()}` : wf.schedule.trim();
    lines.push(`schedule = ${tomlStr(normalized)}`);
  }
  lines.push("");

  for (const step of wf.steps) {
    lines.push("[[steps]]");
    lines.push(`name = ${tomlStr(step.name)}`);
    lines.push(`type = ${tomlStr(step.type)}`);
    if (step.tool) lines.push(`tool = ${tomlStr(step.tool)}`);
    if (step.prompt) lines.push(`prompt = ${tomlStr(step.prompt)}`);
    if (step.model) lines.push(`model = ${tomlStr(step.model)}`);
    if (step.seconds !== undefined) lines.push(`seconds = ${step.seconds}`);
    if (step.expression) lines.push(`expression = ${tomlStr(step.expression)}`);
    if (step.if_true) lines.push(`if_true = ${tomlStr(step.if_true)}`);
    if (step.if_false) lines.push(`if_false = ${tomlStr(step.if_false)}`);
    if (step.steps && step.steps.length > 0) {
      lines.push(`steps = [${step.steps.map(tomlStr).join(", ")}]`);
    }
    if (step.depends_on.length > 0) {
      lines.push(`depends_on = [${step.depends_on.map(tomlStr).join(", ")}]`);
    }
    // Issue 2: use strict undefined/null check so timeout_secs = 0 is preserved
    if (step.timeout_secs !== undefined && step.timeout_secs !== null) {
      lines.push(`timeout_secs = ${step.timeout_secs}`);
    }
    // Issue 3: only write retry when both sub-fields are defined numbers
    if (
      step.retry !== undefined &&
      typeof step.retry.max_retries === "number" &&
      typeof step.retry.retry_delay_ms === "number"
    ) {
      lines.push(
        `retry = { max_retries = ${step.retry.max_retries}, retry_delay_ms = ${step.retry.retry_delay_ms} }`,
      );
    }
    if (step.failure_policy !== undefined) {
      if (typeof step.failure_policy === "string") {
        lines.push(`failure_policy = ${tomlStr(step.failure_policy)}`);
      } else if (step.failure_policy.fallback) {
        lines.push(
          `failure_policy = { fallback = { step = ${tomlStr(step.failure_policy.fallback.step)} } }`,
        );
      }
    }
    // Issue 1: validate args are flat before serializing
    if (step.args && Object.keys(step.args).length > 0) {
      lines.push(`args = ${tomlInlineTable(step.args)}`);
    }
    lines.push("");
  }

  if (wf.layout && Object.keys(wf.layout).length > 0) {
    lines.push("[layout]");
    for (const [name, pos] of Object.entries(wf.layout)) {
      lines.push(
        `${name} = { x = ${pos.x.toFixed(1)}, y = ${pos.y.toFixed(1)} }`,
      );
    }
    lines.push("");
  }

  return lines.join("\n");
}

/** Escape a string for TOML (double-quoted). */
function tomlStr(s: string): string {
  return `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t")}"`;
}

/**
 * Serialize a flat object as a TOML inline table: { key = "val", num = 5 }
 *
 * Constraint (Issue 1): tool step args must be flat key-value pairs only.
 * Nested objects (typeof v === "object" and not Array) are not valid in TOML
 * inline tables produced here. Throws if a nested object is detected so the
 * caller gets a clear error rather than silently producing invalid TOML.
 */
function tomlInlineTable(obj: Record<string, unknown>): string {
  const pairs: string[] = [];
  for (const [k, v] of Object.entries(obj)) {
    if (typeof v === "string") {
      pairs.push(`${k} = ${tomlStr(v)}`);
    } else if (typeof v === "number" || typeof v === "boolean") {
      pairs.push(`${k} = ${v}`);
    } else if (Array.isArray(v)) {
      pairs.push(
        `${k} = [${v.map((item) => (typeof item === "string" ? tomlStr(item) : String(item))).join(", ")}]`,
      );
    } else if (v !== null && typeof v === "object") {
      // Issue 1: nested objects produce invalid TOML inline table syntax
      throw new Error(
        `tool step args must be flat key-value pairs — nested objects not supported (key: "${k}")`,
      );
    }
    // null / undefined values are silently skipped (no TOML representation)
  }
  return `{ ${pairs.join(", ")} }`;
}

/**
 * Create a URL-friendly slug from a workflow name.
 */
function slugify(name: string): string {
  return (
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "workflow"
  );
}
