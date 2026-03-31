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

    // Setters that mark dirty
    set nodes(v: Node[]) {
      nodes = v;
      isDirty = true;
    },
    set edges(v: Edge[]) {
      edges = v;
      isDirty = true;
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

    // Remove a node and its connected edges
    removeNode(nodeId: string) {
      nodes = nodes.filter((n) => n.id !== nodeId);
      edges = edges.filter(
        (e) => e.source !== nodeId && e.target !== nodeId,
      );
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

    // Rename a node: update its id, data.stepName, edges, and step-ref fields in other nodes
    renameNode(oldId: string, newId: string) {
      if (oldId === newId) return;
      nodes = nodes.map((n) => {
        if (n.id === oldId) {
          return { ...n, id: newId, data: { ...n.data, stepName: newId } };
        }
        // Rewrite step-ref/step-refs fields in other nodes that reference the old name
        const defType = (n.data as Record<string, unknown>).definitionType as string | undefined;
        const def = defType ? nodeRegistry.get(defType) : undefined;
        if (!def) return n;
        let changed = false;
        const newData = { ...n.data } as Record<string, unknown>;
        for (const field of def.fields) {
          if (field.type === 'step-ref' && newData[field.key] === oldId) {
            newData[field.key] = newId;
            changed = true;
          } else if (field.type === 'step-refs' && Array.isArray(newData[field.key])) {
            const arr = newData[field.key] as string[];
            if (arr.includes(oldId)) {
              newData[field.key] = arr.map((s) => (s === oldId ? newId : s));
              changed = true;
            }
          }
        }
        return changed ? { ...n, data: newData } : n;
      });
      edges = edges.map((e) => ({
        ...e,
        source: e.source === oldId ? newId : e.source,
        target: e.target === oldId ? newId : e.target,
        id: e.source === oldId || e.target === oldId
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
    },

    // Reset for a new workflow
    reset() {
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
    },

    // Mark as saved (clears dirty flag)
    markSaved(id?: string) {
      if (id) workflowId = id;
      isDirty = false;
    },
  };
}

export const builderStore = createWorkflowBuilderStore();
