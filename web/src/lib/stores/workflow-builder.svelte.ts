import type { Node, Edge } from "@xyflow/svelte";
import { nodeRegistry } from "$lib/components/workflow-builder/node-registry";

function createWorkflowBuilderStore() {
  // Canvas state
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let selectedNodeId = $state<string | null>(null);

  // Workflow metadata
  let workflowId = $state<string | null>(null); // null = new workflow
  let workflowName = $state("");
  let workflowDescription = $state("");
  let workflowSchedule = $state<string | null>(null);

  // UI state
  let isDirty = $state(false);
  let suppressDirty = $state(false); // suppress dirty marking after load
  let viewMode = $state<"visual" | "code">("visual");
  let isRunning = $state(false); // whether this workflow is currently executing

  return {
    // Getters
    get nodes() {
      return nodes;
    },
    get edges() {
      return edges;
    },
    get selectedNodeId() {
      return selectedNodeId;
    },
    get workflowId() {
      return workflowId;
    },
    get workflowName() {
      return workflowName;
    },
    get workflowDescription() {
      return workflowDescription;
    },
    get workflowSchedule() {
      return workflowSchedule;
    },
    get isDirty() {
      return isDirty;
    },
    get viewMode() {
      return viewMode;
    },
    get isRunning() {
      return isRunning;
    },

    // Get the currently selected node object
    get selectedNode(): Node | undefined {
      return selectedNodeId
        ? nodes.find((n) => n.id === selectedNodeId)
        : undefined;
    },

    // Returns true if any node has an invalid step name or empty fallback step
    get hasFormErrors(): boolean {
      const STEP_NAME_RE = /^[a-z0-9_]+$/;
      for (const n of nodes) {
        const data = n.data as Record<string, unknown>;
        const stepName = (data.stepName as string) || "";
        if (!stepName || !STEP_NAME_RE.test(stepName)) return true;
        // Fallback step must be non-empty when failure_policy is Fallback
        const fp = data.failure_policy;
        if (
          fp !== null &&
          typeof fp === "object" &&
          (fp as Record<string, unknown>).Fallback
        ) {
          const fallbackStep = (
            (fp as Record<string, unknown>).Fallback as Record<string, string>
          ).step;
          if (!fallbackStep) return true;
        }
      }
      return false;
    },

    // Setters — SvelteFlow bind:nodes/bind:edges triggers these on every internal update.
    // suppressDirty prevents false dirty state after loadWorkflow/reset.
    set nodes(v: Node[]) {
      nodes = v;
      if (!suppressDirty) isDirty = true;
    },
    set edges(v: Edge[]) {
      edges = v;
      if (!suppressDirty) isDirty = true;
    },

    selectNode(id: string | null) {
      selectedNodeId = id;
    },

    setViewMode(mode: "visual" | "code") {
      viewMode = mode;
    },

    setRunning(running: boolean) {
      isRunning = running;
    },

    updateMeta(meta: {
      name?: string;
      description?: string;
      schedule?: string | null;
    }) {
      if (meta.name !== undefined) workflowName = meta.name;
      if (meta.description !== undefined)
        workflowDescription = meta.description;
      if (meta.schedule !== undefined) workflowSchedule = meta.schedule;
      isDirty = true;
    },

    // Add a new node at a position (from palette drag-drop)
    addNode(node: Node) {
      nodes = [...nodes, node];
      isDirty = true;
    },

    // Remove a node and its connected edges, cleaning up references in other nodes
    removeNode(nodeId: string) {
      // Get the step name of the node being removed (for reference cleanup)
      const removedNode = nodes.find((n) => n.id === nodeId);
      const removedStepName = removedNode
        ? (
            (removedNode.data as Record<string, unknown>).stepName as
              | string
              | undefined
          ) ?? nodeId
        : nodeId;

      nodes = nodes
        .filter((n) => n.id !== nodeId)
        .map((n) => {
          const defType = (n.data as Record<string, unknown>)
            .definitionType as string | undefined;
          const def = defType ? nodeRegistry.get(defType) : undefined;
          let changed = false;
          const newData = { ...n.data } as Record<string, unknown>;

          // Clean up step-ref / step-refs fields defined in the registry
          if (def) {
            for (const field of def.fields) {
              if (
                field.type === "step-ref" &&
                newData[field.key] === removedStepName
              ) {
                newData[field.key] = "";
                changed = true;
              } else if (
                field.type === "step-refs" &&
                Array.isArray(newData[field.key])
              ) {
                const arr = newData[field.key] as string[];
                if (arr.includes(removedStepName)) {
                  newData[field.key] = arr.filter((s) => s !== removedStepName);
                  changed = true;
                }
              }
            }
          }

          // Clean up condition if_true / if_false
          if (newData.if_true === removedStepName) {
            newData.if_true = "";
            changed = true;
          }
          if (newData.if_false === removedStepName) {
            newData.if_false = "";
            changed = true;
          }

          // Clean up fallback policy
          const fp = newData.failure_policy;
          if (
            fp !== null &&
            typeof fp === "object" &&
            (fp as Record<string, unknown>).Fallback
          ) {
            const fallbackStep = (
              (fp as Record<string, unknown>).Fallback as Record<string, string>
            ).step;
            if (fallbackStep === removedStepName) {
              newData.failure_policy = { Fallback: { step: "" } };
              changed = true;
            }
          }

          return changed ? { ...n, data: newData } : n;
        });

      edges = edges.filter((e) => e.source !== nodeId && e.target !== nodeId);
      if (selectedNodeId === nodeId) selectedNodeId = null;
      isDirty = true;
    },

    // Add an edge (connection)
    addEdge(edge: Edge) {
      // Prevent duplicates (same source, target, and sourceHandle)
      if (
        edges.some(
          (e) =>
            e.source === edge.source &&
            e.target === edge.target &&
            e.sourceHandle === edge.sourceHandle,
        )
      )
        return;
      edges = [...edges, edge];
      isDirty = true;
    },

    // Remove an edge
    removeEdge(edgeId: string) {
      edges = edges.filter((e) => e.id !== edgeId);
      isDirty = true;
    },

    // Update a specific node's data (from config panel)
    updateNodeData(nodeId: string, data: Record<string, unknown>) {
      nodes = nodes.map((n) =>
        n.id === nodeId ? { ...n, data: { ...n.data, ...data } } : n,
      );
      isDirty = true;
    },

    // Rename a node: update its id, data.stepName, edges, and all step references in other nodes
    renameNode(oldId: string, newId: string) {
      if (oldId === newId) return;
      nodes = nodes.map((n) => {
        if (n.id === oldId) {
          return { ...n, id: newId, data: { ...n.data, stepName: newId } };
        }
        // Rewrite step-ref/step-refs/if_true/if_false/fallback in other nodes
        const defType = (n.data as Record<string, unknown>).definitionType as
          | string
          | undefined;
        const def = defType ? nodeRegistry.get(defType) : undefined;
        let changed = false;
        const newData = { ...n.data } as Record<string, unknown>;

        if (def) {
          for (const field of def.fields) {
            if (field.type === "step-ref" && newData[field.key] === oldId) {
              newData[field.key] = newId;
              changed = true;
            } else if (
              field.type === "step-refs" &&
              Array.isArray(newData[field.key])
            ) {
              const arr = newData[field.key] as string[];
              if (arr.includes(oldId)) {
                newData[field.key] = arr.map((s) => (s === oldId ? newId : s));
                changed = true;
              }
            }
          }
        }

        // Condition if_true / if_false
        if (newData.if_true === oldId) {
          newData.if_true = newId;
          changed = true;
        }
        if (newData.if_false === oldId) {
          newData.if_false = newId;
          changed = true;
        }

        // Fallback policy
        const fp = newData.failure_policy;
        if (
          fp !== null &&
          typeof fp === "object" &&
          (fp as Record<string, unknown>).Fallback
        ) {
          const fallback = (fp as Record<string, unknown>)
            .Fallback as Record<string, string>;
          if (fallback.step === oldId) {
            newData.failure_policy = { Fallback: { step: newId } };
            changed = true;
          }
        }

        return changed ? { ...n, data: newData } : n;
      });
      edges = edges.map((e) => ({
        ...e,
        source: e.source === oldId ? newId : e.source,
        target: e.target === oldId ? newId : e.target,
        id:
          e.source === oldId || e.target === oldId
            ? `e-${e.source === oldId ? newId : e.source}-${e.target === oldId ? newId : e.target}`
            : e.id,
      }));
      if (selectedNodeId === oldId) selectedNodeId = newId;
      isDirty = true;
    },

    // Load a workflow into the builder
    loadWorkflow(
      wf: {
        id: string;
        name: string;
        description: string;
        schedule: string | null;
      },
      graphNodes: Node[],
      graphEdges: Edge[],
    ) {
      suppressDirty = true;
      workflowId = wf.id;
      workflowName = wf.name;
      workflowDescription = wf.description;
      workflowSchedule = wf.schedule;
      nodes = graphNodes;
      edges = graphEdges;
      selectedNodeId = null;
      isDirty = false;
      viewMode = "visual";
      isRunning = false;
      // Release after SvelteFlow fully processes bind:nodes/bind:edges + fitView
      setTimeout(() => {
        suppressDirty = false;
      }, 150);
    },

    // Reset for a new workflow
    reset() {
      suppressDirty = true;
      workflowId = null;
      workflowName = "";
      workflowDescription = "";
      workflowSchedule = null;
      nodes = [];
      edges = [];
      selectedNodeId = null;
      isDirty = false;
      viewMode = "visual";
      isRunning = false;
      setTimeout(() => {
        suppressDirty = false;
      }, 150);
    },

    // Mark as saved (clears dirty flag)
    // Must suppress SvelteFlow's reactive bind:nodes/bind:edges writeback
    // which otherwise immediately re-sets isDirty via the setters.
    markSaved(id?: string) {
      suppressDirty = true;
      if (id) workflowId = id;
      isDirty = false;
      setTimeout(() => {
        suppressDirty = false;
      }, 150);
    },
  };
}

export const builderStore = createWorkflowBuilderStore();
