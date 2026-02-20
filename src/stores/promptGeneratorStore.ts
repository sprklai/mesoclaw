/**
 * Zustand store for the Prompt Generator UI.
 *
 * Manages artifact generation lifecycle: form state, a single invoke call
 * to the Rust backend (no streaming), and CRUD for previously generated
 * artifacts.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Strip <think>…</think> reasoning blocks from raw LLM output. */
function stripThinking(raw: string): string {
	return raw
		.replace(/<think>[\s\S]*?<\/think>/g, "")
		.replace(/<think>[\s\S]*$/, "") // unclosed block
		.trim();
}

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
	/** Editable output shown to the user (think tags stripped). */
	generatedContent: string;
	lastSaved: GeneratedArtifact | null;
	error: string | null;
	history: GeneratedArtifact[];

	setArtifactType: (type: ArtifactType) => void;
	setName: (name: string) => void;
	setDescription: (desc: string) => void;
	setGeneratedContent: (content: string) => void;
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
		lastSaved: null,
		error: null,
		history: [],

		setArtifactType: (type) => set({ artifactType: type }),
		setName: (name) => set({ name }),
		setDescription: (desc) => set({ description: desc }),
		setGeneratedContent: (content) => set({ generatedContent: content }),

		startGeneration: async () => {
			const { description, artifactType, name } = get();
			const sessionId = crypto.randomUUID();

			set({
				status: "generating",
				generatedContent: "",
				error: null,
				lastSaved: null,
			});

			try {
				const result = await invoke<GeneratedArtifact>(
					"generate_prompt_command",
					{ description, artifactType, name, sessionId },
				);
				set({
					status: "done",
					generatedContent: stripThinking(result.content),
					lastSaved: result,
				});
			} catch (err) {
				set({ status: "error", error: String(err) });
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
				lastSaved: null,
				error: null,
			}),
	}),
);
