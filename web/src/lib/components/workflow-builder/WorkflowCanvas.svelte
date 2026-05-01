<script lang="ts">
	import { SvelteFlow, Controls, Background, type NodeTypes, type Connection, type Edge } from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import StandardNode from './nodes/StandardNode.svelte';
	import ConditionNode from './nodes/ConditionNode.svelte';
	import TriggerNode from './nodes/TriggerNode.svelte';
	import { builderStore } from '$lib/stores/workflow-builder.svelte';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { nodeRegistry } from './node-registry';
	import { generateStepName } from './graph-utils';

	const nodeTypes: NodeTypes = {
		standard: StandardNode as unknown as NodeTypes[string],
		condition: ConditionNode as unknown as NodeTypes[string],
		trigger: TriggerNode as unknown as NodeTypes[string]
	};

	function handleConnect(connection: Connection) {
		if (!connection.source || !connection.target) return;
		if (connection.source === connection.target) return;

		const edge: Edge = {
			id: `e-${connection.source}-${connection.target}`,
			source: connection.source,
			target: connection.target,
			sourceHandle: connection.sourceHandle ?? undefined,
			targetHandle: connection.targetHandle ?? undefined
		};
		builderStore.addEdge(edge);

		// Sync condition node if_true / if_false when branch edges are connected
		const sourceNode = builderStore.nodes.find((n) => n.id === connection.source);
		if (
			sourceNode &&
			(sourceNode.data as Record<string, unknown>).definitionType === 'condition'
		) {
			const targetNode = builderStore.nodes.find((n) => n.id === connection.target);
			const targetStepName = targetNode
				? ((targetNode.data as Record<string, unknown>).stepName as string) || connection.target
				: connection.target;

			if (connection.sourceHandle === 'true') {
				builderStore.updateNodeData(connection.source, { if_true: targetStepName });
			} else if (connection.sourceHandle === 'false') {
				builderStore.updateNodeData(connection.source, { if_false: targetStepName });
			}
		}
	}

	function handleNodeClick({ node }: { node: { id: string }; event: MouseEvent | TouchEvent }) {
		builderStore.selectNode(node.id);
	}

	function handlePaneClick() {
		builderStore.selectNode(null);
	}

	function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: { id: string }[]; edges: { id: string }[] }) {
		for (const n of deletedNodes) {
			builderStore.removeNode(n.id);
		}
		for (const e of deletedEdges) {
			// When a condition branch edge is removed, clear the corresponding field
			const fullEdge = builderStore.edges.find((edge) => edge.id === e.id);
			if (fullEdge) {
				const sourceNode = builderStore.nodes.find((n) => n.id === fullEdge.source);
				if (
					sourceNode &&
					(sourceNode.data as Record<string, unknown>).definitionType === 'condition'
				) {
					if (fullEdge.sourceHandle === 'true') {
						builderStore.updateNodeData(fullEdge.source, { if_true: '' });
					} else if (fullEdge.sourceHandle === 'false') {
						builderStore.updateNodeData(fullEdge.source, { if_false: '' });
					}
				}
			}
			builderStore.removeEdge(e.id);
		}
	}

	function handleDragOver(e: DragEvent) {
		if (!e.dataTransfer?.types.includes('application/workflow-node')) return;
		e.preventDefault();
		e.dataTransfer.dropEffect = 'move';
	}

	function handleDrop(e: DragEvent) {
		if (!e.dataTransfer) return;
		const defType = e.dataTransfer.getData('application/workflow-node');
		if (!defType) return;
		e.preventDefault();

		const def = nodeRegistry.get(defType);
		if (!def) return;

		const existingNames = builderStore.nodes.map(n => (n.data.stepName as string) || n.id);
		const stepName = generateStepName(defType, existingNames);

		// Build default data from definition fields
		const data: Record<string, unknown> = {
			definitionType: def.type,
			stepName
		};
		for (const field of def.fields) {
			if (field.default !== undefined) {
				data[field.key] = field.default;
			}
		}

		// Approximate canvas position from drop coordinates
		const bounds = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const position = {
			x: e.clientX - bounds.left,
			y: e.clientY - bounds.top
		};

		builderStore.addNode({
			id: stepName,
			type: def.visual,
			position,
			data
		});

		builderStore.selectNode(stepName);
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Delete' || e.key === 'Backspace') {
			const selected = builderStore.selectedNodeId;
			if (selected && document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
				builderStore.removeNode(selected);
			}
		}
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<div
	class="flex-1 h-full"
	role="application"
	ondragover={handleDragOver}
	ondrop={handleDrop}
>
	<SvelteFlow
		bind:nodes={builderStore.nodes}
		bind:edges={builderStore.edges}
		{nodeTypes}
		onconnect={handleConnect}
		onnodeclick={handleNodeClick}
		onpaneclick={handlePaneClick}
		ondelete={handleDelete}
		fitView
		colorMode={themeStore.isDark ? 'dark' : 'light'}
		deleteKey={[]}
	>
		<Controls />
		<Background />
	</SvelteFlow>
</div>

<style>
	:global(.svelte-flow__edge-path) {
		stroke: #6366f1;
		stroke-width: 2.5px;
	}
	:global(.svelte-flow__edge.selected .svelte-flow__edge-path),
	:global(.svelte-flow__edge:hover .svelte-flow__edge-path) {
		stroke: #a855f7;
		stroke-width: 3px;
	}
	:global(.svelte-flow__connection-path) {
		stroke: #6366f1;
		stroke-width: 2.5px;
	}
</style>
