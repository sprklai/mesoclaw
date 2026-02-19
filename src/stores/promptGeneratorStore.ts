/**
 * Zustand store for the Prompt Generator UI.
 *
 * Manages artifact generation lifecycle: form state, streaming token events
 * from the Rust backend, and CRUD for previously generated artifacts.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";

// ─── Types ────────────────────────────────────────────────────────────────────

export type ArtifactType =
	| "skill"
	| "agent"
	| "soul"
	| "claude-skill"
	| "generic";

export interface GeneratedArtifact {
	id: string;
	name: string;
	artifact_type: string;
	content: string;
	disk_path: string | null;
}

interface PromptGeneratorState {
	artifactType: ArtifactType;
	name: string;
	description: string;
	status: "idle" | "generating" | "done" | "error";
	generatedContent: string;
	sessionId: string | null;
	lastSaved: GeneratedArtifact | null;
	error: string | null;
	history: GeneratedArtifact[];

	setArtifactType: (type: ArtifactType) => void;
	setName: (name: string) => void;
	setDescription: (desc: string) => void;
	startGeneration: () => Promise<void>;
	loadHistory: () => Promise<void>;
	deleteArtifact: (id: string) => Promise<void>;
	reset: () => void;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const usePromptGeneratorStore = create<PromptGeneratorState>(
	(set, get) => ({
		artifactType: "skill",
		name: "",
		description: "",
		status: "idle",
		generatedContent: "",
		sessionId: null,
		lastSaved: null,
		error: null,
		history: [],

		setArtifactType: (type) => set({ artifactType: type }),
		setName: (name) => set({ name }),
		setDescription: (desc) => set({ description: desc }),

		startGeneration: async () => {
			const { description, artifactType, name } = get();
			const sessionId = crypto.randomUUID();

			set({
				sessionId,
				status: "generating",
				generatedContent: "",
				error: null,
				lastSaved: null,
			});

			// Listen for streaming token events from the backend.
			const unlisten = await listen<{ type: string; content?: string }>(
				`prompt-gen-${sessionId}`,
				(event) => {
					const payload = event.payload;
					if (payload.type === "token" && payload.content) {
						set((s) => ({
							generatedContent: s.generatedContent + payload.content,
						}));
					} else if (payload.type === "done") {
						set({ status: "done" });
						unlisten();
					}
				},
			);

			try {
				const result = await invoke<GeneratedArtifact>(
					"generate_prompt_command",
					{
						description,
						artifactType,
						name,
						sessionId,
					},
				);
				set({ lastSaved: result });
			} catch (err) {
				set({ status: "error", error: String(err) });
				unlisten();
			}
		},

		loadHistory: async () => {
			try {
				const history = await invoke<GeneratedArtifact[]>(
					"list_generated_prompts_command",
				);
				set({ history });
			} catch (err) {
				console.error("[PromptGeneratorStore] Failed to load history:", err);
			}
		},

		deleteArtifact: async (id) => {
			try {
				await invoke("delete_generated_prompt_command", { id });
				await get().loadHistory();
			} catch (err) {
				console.error("[PromptGeneratorStore] Failed to delete artifact:", err);
			}
		},

		reset: () =>
			set({
				artifactType: "skill",
				name: "",
				description: "",
				status: "idle",
				generatedContent: "",
				sessionId: null,
				lastSaved: null,
				error: null,
			}),
	}),
);
