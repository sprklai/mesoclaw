<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import {
		forceSimulation,
		forceLink,
		forceManyBody,
		forceCollide,
		forceX,
		forceY,
		type Simulation,
		type SimulationNodeDatum,
		type SimulationLinkDatum
	} from 'd3-force';
	import { Loader2 } from '@lucide/svelte';
	import type { WikiNode, WikiEdge, WikiPage } from '$lib/stores/wiki.svelte';
	import { configStore } from '$lib/stores/config.svelte';

	interface Props {
		nodes: WikiNode[];
		edges: WikiEdge[];
		pages?: WikiPage[];
		onnodeclick?: (e: CustomEvent<{ slug: string }>) => void;
	}

	let { nodes, edges, pages = [], onnodeclick }: Props = $props();

	// ── config ────────────────────────────────────────────────────────────────

	function num(key: string, fallback: number): number {
		const v = configStore.config[key];
		return typeof v === 'number' ? v : fallback;
	}

	const GC = $derived({
		preTicks:            num('wiki_graph_pre_ticks', 80),
		initialAlpha:        num('wiki_graph_initial_alpha', 0.5),
		alphaDecay:          num('wiki_graph_alpha_decay', 0.02),
		linkDistance:        num('wiki_graph_link_distance', 120),
		chargeStrength:      num('wiki_graph_charge_strength', -350),
		collideRadius:       num('wiki_graph_collide_radius', 40),
		centerForce:         num('wiki_graph_center_force_strength', 0.01),
		clusterForce:        num('wiki_graph_cluster_force_strength', 0.06),
		clusterRadiusFactor: num('wiki_graph_cluster_radius_factor', 0.40),
		clusterScatter:      num('wiki_graph_cluster_scatter', 120),
		dragAlphaTarget:     num('wiki_graph_drag_alpha_target', 0.3),
		labelThreshold:      num('wiki_graph_label_auto_hide_threshold', 200),
		zoomMin:             num('wiki_graph_zoom_min', 0.2),
		zoomMax:             num('wiki_graph_zoom_max', 3),
		zoomStep:            num('wiki_graph_zoom_step', 1.2),
		fitPadding:          num('wiki_graph_fit_padding', 30),
		fitScaleFactor:      num('wiki_graph_fit_scale_factor', 0.9),
		nodeRadius:          num('wiki_graph_node_radius', 6),
		nodeRadiusSelected:  num('wiki_graph_node_radius_selected', 8),
		nodeHitRadius:       num('wiki_graph_node_hit_radius', 20),
		selectionRingRadius: num('wiki_graph_selection_ring_radius', 12),
		labelFontSize:       num('wiki_graph_label_font_size', 10),
		labelYOffset:        num('wiki_graph_label_y_offset', 14),
		wheelZoomStep:       num('wiki_graph_wheel_zoom_step', 1.1),
	});

	// ── types ─────────────────────────────────────────────────────────────────

	interface SimNode extends SimulationNodeDatum {
		id: string;
		label: string;
		page_type: string;
	}

	interface SimLink extends SimulationLinkDatum<SimNode> {
		source: SimNode | string;
		target: SimNode | string;
	}

	// ── state ─────────────────────────────────────────────────────────────────

	let wrapEl = $state<HTMLDivElement | undefined>();
	let svgEl = $state<SVGSVGElement | undefined>();
	let width = $state(600);
	let height = $state(400);
	let simNodes = $state<SimNode[]>([]);
	let simLinks = $state<SimLink[]>([]);

	// pan / zoom
	let tx = $state(0);
	let ty = $state(0);
	let scale = $state(1);

	// drag state (plain vars — not reactive, only used in event handlers)
	let dragNode = $state<SimNode | null>(null);
	let dragMoved = false;
	let dragDownPos = { x: 0, y: 0 };
	let panning = $state(false);
	let panStart = { x: 0, y: 0, tx: 0, ty: 0 };


	// controls
	let locked = $state(false);
	let showLabels = $state(true);
	let hoveredNode = $state<SimNode | null>(null);

	// Reactive tick counter — incremented by d3 simulation each tick so that
	// inner template effects re-run and pick up d3's in-place x/y mutations.
	// (Svelte 5 fine-grained reactivity won't re-evaluate cx={node.x} when d3
	// mutates the underlying object directly; _tick forces the re-read.)
	let _tick = $state(0);

	function zoomIn() {
		const cx = width / 2;
		const cy = height / 2;
		const newScale = Math.min(GC.zoomMax, scale * GC.zoomStep);
		tx = cx - (cx - tx) * (newScale / scale);
		ty = cy - (cy - ty) * (newScale / scale);
		scale = newScale;
	}

	function zoomOut() {
		const cx = width / 2;
		const cy = height / 2;
		const newScale = Math.max(GC.zoomMin, scale * (1 / GC.zoomStep));
		tx = cx - (cx - tx) * (newScale / scale);
		ty = cy - (cy - ty) * (newScale / scale);
		scale = newScale;
	}

	function fitView() {
		if (simNodes.length === 0) return;
		const xs = simNodes.map((n) => n.x ?? 0);
		const ys = simNodes.map((n) => n.y ?? 0);
		const minX = Math.min(...xs) - GC.fitPadding;
		const maxX = Math.max(...xs) + GC.fitPadding;
		const minY = Math.min(...ys) - GC.fitPadding;
		const maxY = Math.max(...ys) + GC.fitPadding;
		const s = Math.min(GC.zoomMax, Math.max(GC.zoomMin, Math.min(width / (maxX - minX), height / (maxY - minY)) * GC.fitScaleFactor));
		scale = s;
		tx = (width - (maxX + minX) * s) / 2;
		ty = (height - (maxY + minY) * s) / 2;
	}

	// tooltip (pinned to bottom-left corner)
	let tooltipNode = $state<SimNode | null>(null);

	let simulation: Simulation<SimNode, SimLink> | null = null;
	let observer: ResizeObserver | null = null;
	let graphReady = $state(false);

	// ── topology guard (L5 fix) ───────────────────────────────────────────────
	// Build a stable key from node IDs and edge pairs. The simulation is only
	// rebuilt when this key changes — not when visual config props like colors or
	// label thresholds change. Without this guard, any configStore update that
	// touches GC (e.g. a theme change via wiki_graph_* keys) would tear down and
	// re-run the expensive D3 force simulation.
	const topoKey = $derived(
		nodes.map((n) => n.id).sort().join(',') +
		'|' +
		edges.map((e) => `${e.from}>${e.to}`).sort().join(',')
	);
	let lastTopoKey = '';

	// ── helpers ───────────────────────────────────────────────────────────────

	const pageMap = $derived(new Map(pages.map((p) => [p.slug, p])));

	// Set of IDs to highlight (selected node + direct neighbors). null = no filter.
	const highlightIds = $derived.by(() => {
		if (!tooltipNode) return null;
		const ids = new Set<string>([tooltipNode.id]);
		for (const link of simLinks) {
			const s = link.source as SimNode;
			const t = link.target as SimNode;
			if (s.id === tooltipNode.id) ids.add(t.id);
			if (t.id === tooltipNode.id) ids.add(s.id);
		}
		return ids;
	});

	function typeColor(type: string): string {
		switch (type) {
			case 'concept':    return '#3b82f6';
			case 'entity':     return '#22c55e';
			case 'topic':      return '#f97316';
			case 'comparison': return '#a855f7';
			case 'query':      return '#ec4899';
			default:           return '#6b7280';
		}
	}

	function typeBg(type: string): string {
		switch (type) {
			case 'concept':    return 'bg-blue-500/10 text-blue-400';
			case 'entity':     return 'bg-green-500/10 text-green-400';
			case 'topic':      return 'bg-orange-500/10 text-orange-400';
			case 'comparison': return 'bg-purple-500/10 text-purple-400';
			case 'query':      return 'bg-pink-500/10 text-pink-400';
			default:           return 'bg-muted text-muted-foreground';
		}
	}

	// ── position persistence ──────────────────────────────────────────────────
	// Node positions are saved to localStorage so the layout is stable across
	// sessions. Keyed by node ID (not topology hash) so adding a single new page
	// only displaces that one node — all existing nodes keep their saved positions.

	const POSITIONS_KEY = 'wiki-graph-node-positions';

	function savePositions(ns: SimNode[]) {
		const map: Record<string, { x: number; y: number }> = {};
		for (const n of ns) map[n.id] = { x: n.x ?? 0, y: n.y ?? 0 };
		try { localStorage.setItem(POSITIONS_KEY, JSON.stringify(map)); } catch {}
	}

	function loadSavedPositions(): Record<string, { x: number; y: number }> | null {
		try {
			const raw = localStorage.getItem(POSITIONS_KEY);
			return raw ? JSON.parse(raw) : null;
		} catch { return null; }
	}

	// ── simulation ────────────────────────────────────────────────────────────

	// Cluster centers: each page_type gets a fixed position on a pentagon
	// around the canvas center so same-type nodes naturally group together.
	const CLUSTER_ANGLE: Record<string, number> = {
		concept:    0,
		entity:     72,
		topic:      144,
		comparison: 216,
		query:      288,
	};

	function clusterCenter(w: number, h: number, type: string): { x: number; y: number } {
		const deg = CLUSTER_ANGLE[type] ?? 0;
		const rad = (deg - 90) * (Math.PI / 180);
		const r = Math.min(w, h) * GC.clusterRadiusFactor;
		return { x: w / 2 + r * Math.cos(rad), y: h / 2 + r * Math.sin(rad) };
	}

	function buildSimulation(w: number, h: number) {
		graphReady = false;
		// Guard: deduplicate by id in case backend returns duplicate slugs
		const uniqueNodes = [...new Map(nodes.map((n) => [n.id, n])).values()];
		// Restore saved positions when available so layout is stable across sessions.
		// Nodes without a saved position (newly added pages) fall back to cluster center.
		const savedPositions = loadSavedPositions();
		const sNodes: SimNode[] = uniqueNodes.map((n) => {
			const saved = savedPositions?.[n.id];
			if (saved) return { ...n, x: saved.x, y: saved.y };
			const center = clusterCenter(w, h, n.page_type);
			return {
				...n,
				x: center.x + (Math.random() - 0.5) * GC.clusterScatter,
				y: center.y + (Math.random() - 0.5) * GC.clusterScatter,
			};
		});
		const allRestored = !!savedPositions && uniqueNodes.every((n) => savedPositions[n.id] != null);

		const idToNode = new Map(sNodes.map((n) => [n.id, n]));
		const sLinks: SimLink[] = edges
			.filter((e) => idToNode.has(e.from) && idToNode.has(e.to))
			.map((e) => ({ source: e.from, target: e.to }));

		// Custom cluster force: gently pulls each node toward its type's center.
		// D3 expects a bare function (alpha) => void, not an object wrapper.
		function clusterForce(alpha: number) {
			for (const node of sNodes) {
				const center = clusterCenter(w, h, node.page_type);
				node.vx = (node.vx ?? 0) + (center.x - (node.x ?? 0)) * GC.clusterForce * alpha;
				node.vy = (node.vy ?? 0) + (center.y - (node.y ?? 0)) * GC.clusterForce * alpha;
			}
		}

		const sim = forceSimulation<SimNode>(sNodes)
			.force('link', forceLink<SimNode, SimLink>(sLinks).id((d) => d.id).distance(GC.linkDistance))
			.force('charge', forceManyBody<SimNode>().strength(GC.chargeStrength))
			.force('collide', forceCollide<SimNode>(GC.collideRadius))
			.force('cluster', clusterForce)
			// Weak global centering — cluster force handles spatial placement
			.force('x', forceX(w / 2).strength(GC.centerForce))
			.force('y', forceY(h / 2).strength(GC.centerForce))
			.alphaDecay(GC.alphaDecay)
			.on('tick', () => {
				// Increment reactive counter so template effects re-run and read
				// the current x/y values that d3 mutated in-place on the nodes.
				_tick++;
			})
			.on('end', () => {
				// Auto-center once the simulation has converged to its final layout.
				// fitView() called right after buildSimulation() only sees pre-tick
				// positions; this ensures the final bounding box is used.
				savePositions(sNodes);
				fitView();
				graphReady = true;
			});

		if (allRestored) {
			// All nodes have saved positions — skip simulation for instant display.
			sim.stop();
			fitView();
			graphReady = true;
		} else {
			// More pre-ticks for better initial cluster separation before user sees layout
			for (let i = 0; i < GC.preTicks; i++) sim.tick();
			sim.alpha(GC.initialAlpha).restart();
		}

		simNodes = sNodes;
		simLinks = sLinks as SimLink[];
		simulation = sim;
	}

	// ── lifecycle ─────────────────────────────────────────────────────────────

	onMount(() => {
		if (!svgEl) return;

		// Use ResizeObserver for initial setup so we get guaranteed non-zero
		// dimensions (getBoundingClientRect on mount can return 0 before layout).
		observer = new ResizeObserver((entries) => {
			const entry = entries[0];
			if (!entry) return;
			const newW = entry.contentRect.width;
			const newH = entry.contentRect.height;
			if (newW === 0 || newH === 0) return;

			if (!simulation) {
				// First valid measurement: build and fit
				width = newW;
				height = newH;
				lastTopoKey = topoKey;
				buildSimulation(width, height);
			} else {
				// Subsequent resize: re-center forces, keep user's pan/zoom
				width = newW;
				height = newH;
				simulation.force('x', forceX(width / 2).strength(GC.centerForce));
				simulation.force('y', forceY(height / 2).strength(GC.centerForce));
				simulation.alpha(GC.dragAlphaTarget).restart();
			}
		});
		observer.observe(svgEl);
	});

	// ── topology-driven simulation rebuild (L5 fix) ───────────────────────────
	// Re-run buildSimulation only when node/edge topology changes, not on every
	// reactive update of config values like colors or label thresholds.
	$effect(() => {
		const key = topoKey; // explicit dep on topology key only
		if (key === lastTopoKey || width === 0 || height === 0) return;
		lastTopoKey = key;
		simulation?.stop();
		simulation = null;
		buildSimulation(width, height);
	});

	onDestroy(() => {
		simulation?.stop();
		observer?.disconnect();
	});

	// ── interaction ────────────────────────────────────────────────────────────

	function svgPoint(e: PointerEvent): { x: number; y: number } {
		return {
			x: (e.clientX - svgEl!.getBoundingClientRect().left - tx) / scale,
			y: (e.clientY - svgEl!.getBoundingClientRect().top - ty) / scale
		};
	}

	function handleWheel(e: WheelEvent) {
		e.preventDefault();
		if (locked) return;
		const factor = e.deltaY < 0 ? GC.wheelZoomStep : 1 / GC.wheelZoomStep;
		const rect = svgEl!.getBoundingClientRect();
		const mx = e.clientX - rect.left;
		const my = e.clientY - rect.top;
		const newScale = Math.min(GC.zoomMax, Math.max(GC.zoomMin, scale * factor));
		tx = mx - (mx - tx) * (newScale / scale);
		ty = my - (my - ty) * (newScale / scale);
		scale = newScale;
	}

	function handleSvgPointerDown(e: PointerEvent) {
		// fires only on background (node pointerdown calls stopPropagation)
		tooltipNode = null;
		if (locked) return;
		panning = true;
		panStart = { x: e.clientX, y: e.clientY, tx, ty };

		const controller = new AbortController();
		function onMove(me: PointerEvent) {
			tx = panStart.tx + (me.clientX - panStart.x);
			ty = panStart.ty + (me.clientY - panStart.y);
		}
		function onUp() {
			panning = false;
			controller.abort();
		}
		window.addEventListener('pointermove', onMove, { signal: controller.signal });
		window.addEventListener('pointerup', onUp, { signal: controller.signal });
	}

	function handleNodePointerDown(e: PointerEvent, node: SimNode) {
		e.stopPropagation();
		e.preventDefault();
		// Pointer capture routes all subsequent move/up events here regardless of
		// how fast the cursor moves — the standard d3-force drag fix.
		const target = e.currentTarget as SVGGElement;
		target.setPointerCapture(e.pointerId);

		dragNode = node;
		dragMoved = false;
		dragDownPos = { x: e.clientX, y: e.clientY };
		node.fx = node.x;
		node.fy = node.y;
		node.vx = 0;
		node.vy = 0;
		simulation?.alphaTarget(GC.dragAlphaTarget).restart();

		function onMove(me: PointerEvent) {
			const dx = me.clientX - dragDownPos.x;
			const dy = me.clientY - dragDownPos.y;
			if (Math.sqrt(dx * dx + dy * dy) > 3) dragMoved = true;
			const pt = svgPoint(me);
			node.fx = pt.x;
			node.fy = pt.y;
		}
		function onUp() {
			node.vx = 0;
			node.vy = 0;
			node.fx = null;
			node.fy = null;
			simulation?.alphaTarget(0).restart();
			dragNode = null;
			hoveredNode = null;

			// Persist manual layout so dragged positions survive a page reload.
			if (dragMoved) savePositions(simNodes);

			if (!dragMoved) {
				tooltipNode = tooltipNode?.id === node.id ? null : node;
			}

			target.removeEventListener('pointermove', onMove);
			target.removeEventListener('pointerup', onUp);
		}
		target.addEventListener('pointermove', onMove);
		target.addEventListener('pointerup', onUp);
	}

	function handleOpenPage() {
		if (!tooltipNode) return;
		const slug = tooltipNode.id;
		tooltipNode = null;
		onnodeclick?.(new CustomEvent('nodeclick', { detail: { slug } }));
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div bind:this={wrapEl} class="relative h-full w-full overflow-hidden">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<svg
		bind:this={svgEl}
		class="h-full w-full"
		style="cursor: {dragNode ? 'grabbing' : panning ? 'grabbing' : 'grab'}; touch-action: none"
		onwheel={handleWheel}
		onpointerdown={handleSvgPointerDown}
	>
		<!-- transparent hit area covering the full SVG -->
		<rect width={width} height={height} fill="transparent" />

		<g
			transform="translate({tx},{ty}) scale({scale})"
			style="opacity: {graphReady ? 1 : 0}; transition: opacity 0.2s ease;"
		>
			<!-- edges -->
			{#each simLinks as link, i (i)}
				{@const s = link.source as SimNode}
				{@const t = link.target as SimNode}
				{#if _tick >= 0 && s.x != null && s.y != null && t.x != null && t.y != null}
					<line
						x1={s.x}
						y1={s.y}
						x2={t.x}
						y2={t.y}
						stroke="currentColor"
						stroke-opacity={highlightIds
							? (highlightIds.has(s.id) && highlightIds.has(t.id) ? 0.5 : 0.04)
							: 0.2}
						stroke-width="1"
					/>
				{/if}
			{/each}

			<!-- nodes — keyed by id so DOM is patched on each tick, preserving animate -->
			{#each simNodes as node (node.id)}
				{#if _tick >= 0 && node.x != null && node.y != null}
					{@const dimmed = highlightIds ? !highlightIds.has(node.id) : false}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<g
						style="cursor: pointer"
						onpointerdown={(e) => handleNodePointerDown(e, node)}
						onpointerenter={() => { if (!dragNode) hoveredNode = node; }}
						onpointerleave={() => { hoveredNode = null; }}
					>
						<!-- invisible hit area — stays large in SVG coords so nodes are
						     always draggable even when the graph is zoomed out (small scale) -->
						<circle
							cx={node.x}
							cy={node.y}
							r={GC.nodeHitRadius / scale}
							fill="transparent"
							stroke="none"
						/>

						<!-- selection ring -->
						{#if tooltipNode?.id === node.id}
							<circle
								cx={node.x}
								cy={node.y}
								r={GC.selectionRingRadius}
								fill="none"
								stroke={typeColor(node.page_type)}
								stroke-opacity="0.7"
								stroke-width="2"
							/>
						{/if}

						<!-- main dot -->
						<circle
							cx={node.x}
							cy={node.y}
							r={tooltipNode?.id === node.id ? GC.nodeRadiusSelected : GC.nodeRadius}
							fill={typeColor(node.page_type)}
							fill-opacity={dimmed ? 0.12 : (tooltipNode?.id === node.id ? 1 : 0.85)}
						/>

						{#if showLabels || hoveredNode?.id === node.id || tooltipNode?.id === node.id}
							<text
								x={node.x}
								y={node.y + GC.labelYOffset / scale}
								text-anchor="middle"
								font-size={GC.labelFontSize / scale}
								fill="currentColor"
								fill-opacity={dimmed ? 0.15 : 0.8}
								style="pointer-events: none; user-select: none"
							>{node.label}</text>
						{/if}
					</g>
				{/if}
			{/each}
		</g>
	</svg>

	<!-- info panel — pinned bottom-left, above legend -->
	{#if tooltipNode}
		{@const page = pageMap.get(tooltipNode.id)}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="pointer-events-auto absolute bottom-3 left-3 z-10 w-52 rounded-lg border bg-popover p-3 shadow-lg">
			<div class="mb-1.5 flex items-start justify-between gap-1">
				<p class="text-sm font-semibold leading-tight">{tooltipNode.label}</p>
				<button
					class="mt-0.5 shrink-0 text-muted-foreground hover:text-foreground"
					onclick={() => (tooltipNode = null)}
					aria-label="Close"
				>✕</button>
			</div>
			<span class="inline-block rounded px-1.5 py-0.5 text-[10px] font-medium {typeBg(tooltipNode.page_type)}">
				{tooltipNode.page_type}
			</span>
			{#if page?.tldr}
				<p class="mt-2 line-clamp-3 text-[11px] leading-relaxed text-muted-foreground">{page.tldr}</p>
			{/if}
			{#if page?.tags && page.tags.length > 0}
				<div class="mt-1.5 flex flex-wrap gap-1">
					{#each page.tags.slice(0, 4) as tag}
						<span class="text-[10px] text-muted-foreground">#{tag}</span>
					{/each}
				</div>
			{/if}
			{#if onnodeclick}
				<button
					class="mt-2.5 w-full rounded-md bg-primary px-2 py-1 text-[11px] font-medium text-primary-foreground hover:bg-primary/90"
					onclick={handleOpenPage}
				>Open page</button>
			{/if}
		</div>
	{/if}

	<!-- zoom controls -->
	<div class="absolute bottom-3 right-3 z-10 flex flex-col overflow-hidden rounded-md border bg-background shadow-md">
		<!-- Labels toggle — text-lines icon, above zoom buttons -->
		<button
			class="flex items-center justify-center border-b p-1.5 transition-colors {showLabels ? 'bg-muted text-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
			onclick={() => { showLabels = !showLabels; }}
			title={showLabels ? 'Hide labels' : 'Show labels'}
		>
			<!-- text/label icon: three horizontal lines of varying width -->
			<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
				<path d="M2 3h10M2 7h7M2 11h9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
		</button>
		<button
			class="flex items-center justify-center border-b p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
			onclick={zoomIn}
			title="Zoom in"
		>
			<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
				<path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
		</button>
		<button
			class="flex items-center justify-center border-b p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
			onclick={zoomOut}
			title="Zoom out"
		>
			<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
				<path d="M2 7h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
		</button>
		<button
			class="flex items-center justify-center border-b p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
			onclick={fitView}
			title="Fit view"
		>
			<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
				<path d="M1 4V1h3M10 1h3v3M13 10v3h-3M4 13H1v-3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
		</button>
		<button
			class="flex items-center justify-center p-1.5 transition-colors {locked ? 'bg-muted text-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
			onclick={() => (locked = !locked)}
			title={locked ? 'Unlock pan/zoom' : 'Lock pan/zoom'}
		>
			<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
				{#if locked}
					<rect x="2" y="6" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
					<path d="M4 6V4a3 3 0 0 1 6 0v2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				{:else}
					<rect x="2" y="6" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
					<path d="M4 6V4a3 3 0 0 1 6 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				{/if}
			</svg>
		</button>
	</div>

	<!-- simulation loading overlay — visible while d3 is still running -->
	{#if !graphReady}
		<div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-2">
			<Loader2 class="h-6 w-6 animate-spin text-muted-foreground" />
			<p class="text-xs text-muted-foreground">Laying out graph…</p>
		</div>
	{/if}

	<!-- legend -->
	<div class="pointer-events-none absolute right-3 top-3 flex flex-col gap-1 rounded-lg border bg-popover/80 px-2.5 py-2 backdrop-blur-sm">
		{#each [['concept','#3b82f6'],['entity','#22c55e'],['topic','#f97316'],['comparison','#a855f7'],['query','#ec4899']] as [type, color]}
			<div class="flex items-center gap-1.5">
				<svg width="10" height="10" viewBox="0 0 10 10">
					<circle cx="5" cy="5" r="4" fill={color} fill-opacity="0.85" />
				</svg>
				<span class="text-[10px] capitalize text-muted-foreground">{type}</span>
			</div>
		{/each}
	</div>
</div>
