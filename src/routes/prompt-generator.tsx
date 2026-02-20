import { createFileRoute } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

import { PageHeader } from "@/components/layout/PageHeader";
import { useContextPanelStore } from "@/stores/contextPanelStore";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
	Check,
	CheckCircle2,
	Copy,
	Loader2,
	Pencil,
	Plus,
	RefreshCw,
	Trash2,
} from "@/lib/icons";
import { deleteSkill, getSkillDetails, updateSkill } from "@/lib/tauri/skills";
import { showError, showSuccess } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { useSkillStore } from "@/stores/skillStore";
import {
	type ArtifactType,
	type GeneratedArtifact,
	usePromptGeneratorStore,
} from "@/stores/promptGeneratorStore";

export const Route = createFileRoute("/prompt-generator")({
	component: PromptGeneratorPage,
});

const ARTIFACT_TYPES: { value: ArtifactType; label: string }[] = [
	{ value: "skill", label: "Skill" },
	{ value: "agent", label: "Agent" },
	{ value: "soul", label: "Soul" },
	{ value: "claude-skill", label: "Claude Skill" },
	{ value: "generic", label: "Generic" },
];

type LibraryTab = ArtifactType | "all";

function PromptContextPanel({
  artifactType,
  artifactCount,
  status,
  lastSaved,
}: {
  artifactType: ArtifactType;
  artifactCount: number;
  status: "idle" | "generating" | "done" | "error";
  lastSaved: GeneratedArtifact | null;
}) {
  const typeLabel = ARTIFACT_TYPES.find((t) => t.value === artifactType)?.label ?? artifactType;
  const typeDescriptions: Record<ArtifactType, string> = {
    skill: "Reusable prompt template for skills.",
    agent: "Autonomous agent system prompt.",
    soul: "Character and personality definition.",
    "claude-skill": "Claude Code skill file.",
    generic: "General-purpose prompt template.",
  };

  return (
    <div className="space-y-4 p-4">
      <div>
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Type
        </p>
        <p className="text-sm font-medium">{typeLabel}</p>
        <p className="mt-1 text-xs text-muted-foreground">{typeDescriptions[artifactType]}</p>
      </div>

      <div>
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Library
        </p>
        <p className="text-sm">
          {artifactCount} artifact{artifactCount !== 1 ? "s" : ""}
        </p>
      </div>

      {status === "generating" && (
        <div className="flex items-center gap-2 text-xs text-primary">
          <span className="size-2 animate-pulse rounded-full bg-primary" />
          Generating…
        </div>
      )}

      {lastSaved && (
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Last Generated
          </p>
          <p className="truncate text-xs text-foreground">{lastSaved.name}</p>
          {lastSaved.disk_path && (
            <p className="mt-0.5 truncate text-xs text-muted-foreground">{lastSaved.disk_path}</p>
          )}
        </div>
      )}
    </div>
  );
}

function PromptGeneratorPage() {
	const artifactType = usePromptGeneratorStore((s) => s.artifactType);
	const name = usePromptGeneratorStore((s) => s.name);
	const description = usePromptGeneratorStore((s) => s.description);
	const status = usePromptGeneratorStore((s) => s.status);
	const generatedContent = usePromptGeneratorStore((s) => s.generatedContent);
	const lastSaved = usePromptGeneratorStore((s) => s.lastSaved);
	const error = usePromptGeneratorStore((s) => s.error);
	const history = usePromptGeneratorStore((s) => s.history);
	const isDirty = usePromptGeneratorStore((s) => s.isDirty);

	const setArtifactType = usePromptGeneratorStore((s) => s.setArtifactType);
	const setName = usePromptGeneratorStore((s) => s.setName);
	const setDescription = usePromptGeneratorStore((s) => s.setDescription);
	const setGeneratedContent = usePromptGeneratorStore(
		(s) => s.setGeneratedContent,
	);
	const startGeneration = usePromptGeneratorStore((s) => s.startGeneration);
	const saveContent = usePromptGeneratorStore((s) => s.saveContent);
	const reset = usePromptGeneratorStore((s) => s.reset);
	const loadHistory = usePromptGeneratorStore((s) => s.loadHistory);
	const deleteArtifact = usePromptGeneratorStore((s) => s.deleteArtifact);

	// Skill store for filesystem-based skills
	const { skillsByCategory, loadSkills, refresh: refreshSkills } = useSkillStore();
	const allFsSkills = Object.values(skillsByCategory).flat();

	// Ref to scroll generator form into view
	const formRef = useRef<HTMLDivElement>(null);

	// Library tab state
	const [activeTab, setActiveTab] = useState<LibraryTab>("all");

	// Edit dialog for filesystem skills
	const [editFsSkillOpen, setEditFsSkillOpen] = useState(false);
	const [editingFsSkillId, setEditingFsSkillId] = useState<string | null>(null);
	const [editFsContent, setEditFsContent] = useState("");
	const [isSavingFsEdit, setIsSavingFsEdit] = useState(false);

	// Edit dialog for generated-prompt history artifacts
	const [editArtifactOpen, setEditArtifactOpen] = useState(false);
	const [editingArtifactId, setEditingArtifactId] = useState<string | null>(null);
	const [editArtifactContent, setEditArtifactContent] = useState("");
	const [isSavingArtifactEdit, setIsSavingArtifactEdit] = useState(false);

	// Delete confirm dialog for filesystem skills
	const [deleteFsSkillOpen, setDeleteFsSkillOpen] = useState(false);
	const [deletingFsSkillId, setDeletingFsSkillId] = useState<string | null>(null);
	const [deletingFsSkillName, setDeletingFsSkillName] = useState("");
	const [isDeletingFsSkill, setIsDeletingFsSkill] = useState(false);

	useEffect(() => {
		void loadHistory();
		void loadSkills();
	}, [loadHistory, loadSkills]);

	useEffect(() => {
		useContextPanelStore.getState().setContent(
			<PromptContextPanel
				artifactType={artifactType}
				artifactCount={history.length}
				status={status}
				lastSaved={lastSaved}
			/>,
		);
		return () => useContextPanelStore.getState().clearContent();
	}, [artifactType, history.length, status, lastSaved]);

	const isGenerating = status === "generating";
	const hasOutput = status === "done" || status === "error";

	const filteredHistory: GeneratedArtifact[] =
		activeTab === "all"
			? history
			: history.filter((a) => a.artifact_type === activeTab);

	async function handleCopy() {
		if (generatedContent) {
			await navigator.clipboard.writeText(generatedContent);
		}
	}

	function handleAddType(type: ArtifactType) {
		setArtifactType(type);
		setTimeout(() => {
			formRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
		}, 50);
	}

	// --- Filesystem skill edit ---
	async function handleEditFsSkill(skillId: string) {
		try {
			const details = await getSkillDetails(skillId);
			setEditingFsSkillId(skillId);
			setEditFsContent(details.template);
			setEditFsSkillOpen(true);
		} catch {
			showError("Failed to load skill content");
		}
	}

	async function handleSaveFsSkill() {
		if (!editingFsSkillId) return;
		setIsSavingFsEdit(true);
		try {
			await updateSkill(editingFsSkillId, editFsContent);
			await refreshSkills();
			setEditFsSkillOpen(false);
			showSuccess("Skill saved");
		} catch {
			showError("Failed to save skill");
		} finally {
			setIsSavingFsEdit(false);
		}
	}

	// --- History artifact edit ---
	function handleEditArtifact(artifact: GeneratedArtifact) {
		setEditingArtifactId(artifact.id);
		setEditArtifactContent(artifact.content);
		setEditArtifactOpen(true);
	}

	async function handleSaveArtifact() {
		if (!editingArtifactId) return;
		setIsSavingArtifactEdit(true);
		try {
			await invoke("update_generated_prompt_command", {
				id: editingArtifactId,
				content: editArtifactContent,
			});
			await loadHistory();
			setEditArtifactOpen(false);
			showSuccess("Saved");
		} catch {
			showError("Failed to save");
		} finally {
			setIsSavingArtifactEdit(false);
		}
	}

	// --- Filesystem skill delete ---
	async function handleDeleteFsSkill() {
		if (!deletingFsSkillId) return;
		setIsDeletingFsSkill(true);
		try {
			await deleteSkill(deletingFsSkillId);
			await refreshSkills();
			setDeleteFsSkillOpen(false);
			showSuccess(`Deleted "${deletingFsSkillName}"`);
		} catch {
			showError("Failed to delete skill");
		} finally {
			setIsDeletingFsSkill(false);
		}
	}

	const addButtonLabel =
		activeTab === "all"
			? "Skill"
			: (ARTIFACT_TYPES.find((t) => t.value === activeTab)?.label ?? "");

	const addButtonType: ArtifactType =
		activeTab === "all" ? "skill" : (activeTab as ArtifactType);

	const showLibrary = allFsSkills.length > 0 || history.length > 0;

	return (
		<div className="flex h-full flex-col gap-4 overflow-hidden">
			<PageHeader
				title="Generate Prompt"
				description="Generate AI prompt templates for skills, agents, souls, and more."
			/>

			<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
				{/* Type selector pills */}
				<div className="flex flex-wrap gap-2">
					{ARTIFACT_TYPES.map((t) => (
						<button
							key={t.value}
							type="button"
							onClick={() => setArtifactType(t.value)}
							disabled={isGenerating}
							className={cn(
								"rounded-full border px-3 py-1.5 text-sm font-medium transition-colors",
								"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
								"disabled:pointer-events-none disabled:opacity-50",
								artifactType === t.value
									? "border-primary bg-primary/10 text-primary"
									: "border-border text-muted-foreground hover:bg-accent hover:text-accent-foreground",
							)}
						>
							{t.label}
						</button>
					))}
				</div>

				{/* Input form */}
				<div ref={formRef} className="space-y-3">
					<div>
						<label
							htmlFor="prompt-name"
							className="mb-1.5 block text-sm font-medium"
						>
							Name
						</label>
						<Input
							id="prompt-name"
							placeholder="e.g. code-reviewer"
							value={name}
							onChange={(e) => setName(e.target.value)}
							disabled={isGenerating}
						/>
					</div>
					<div>
						<label
							htmlFor="prompt-description"
							className="mb-1.5 block text-sm font-medium"
						>
							Describe what you want
						</label>
						<Textarea
							id="prompt-description"
							placeholder="Describe the prompt you want to generate..."
							value={description}
							onChange={(e) => setDescription(e.target.value)}
							disabled={isGenerating}
							rows={4}
						/>
					</div>
					<div className="flex gap-2">
						<Button
							onClick={() => void startGeneration()}
							disabled={isGenerating || !description.trim()}
						>
							{isGenerating ? (
								<>
									<Loader2 aria-hidden className="animate-spin" />
									Generating...
								</>
							) : (
								"Generate"
							)}
						</Button>
						{hasOutput && (
							<Button
								variant="outline"
								onClick={() => void startGeneration()}
								disabled={isGenerating || !description.trim()}
							>
								<RefreshCw aria-hidden />
								Regenerate
							</Button>
						)}
					</div>
				</div>

				{/* Output panel */}
				{hasOutput && (
					<div className="space-y-2 rounded-lg border border-border p-4">
						<div className="flex items-center justify-between">
							<h2 className="text-sm font-semibold">Generated Prompt</h2>
							<div className="flex gap-2">
								{isDirty && (
									<Button
										variant="outline"
										size="sm"
										onClick={() => void saveContent()}
									>
										<Check aria-hidden />
										Save
									</Button>
								)}
								<Button
									variant="ghost"
									size="sm"
									onClick={() => void handleCopy()}
									disabled={!generatedContent}
									aria-label="Copy to clipboard"
								>
									<Copy aria-hidden />
									Copy
								</Button>
								<Button variant="ghost" size="sm" onClick={reset}>
									Clear
								</Button>
							</div>
						</div>

						{status === "done" && (
							<Textarea
								value={generatedContent}
								onChange={(e) => setGeneratedContent(e.target.value)}
								rows={16}
								className="font-mono text-sm"
								placeholder="Generated content will appear here..."
							/>
						)}

						{status === "error" && error && (
							<p className="text-xs text-destructive">{error}</p>
						)}

						{lastSaved?.disk_path && (
							<p className="flex items-center gap-1.5 text-xs text-muted-foreground">
								<CheckCircle2 aria-hidden className="size-3.5 text-green-500" />
								Saved to {lastSaved.disk_path}
							</p>
						)}
					</div>
				)}

				{/* Library */}
				{showLibrary && (
					<div className="space-y-3">
						<h2 className="text-sm font-semibold">Library</h2>

						{/* Tab bar */}
						<div className="flex flex-wrap items-center gap-1 border-b border-border pb-2">
							{(["all", "skill", "agent", "soul", "claude-skill", "generic"] as LibraryTab[]).map(
								(tab) => (
									<button
										key={tab}
										type="button"
										onClick={() => setActiveTab(tab)}
										className={cn(
											"rounded px-2.5 py-1 text-xs font-medium transition-colors",
											activeTab === tab
												? "bg-primary/10 text-primary"
												: "text-muted-foreground hover:text-foreground",
										)}
									>
										{tab === "all"
											? "All"
											: (ARTIFACT_TYPES.find((t) => t.value === tab)?.label ?? tab)}
									</button>
								),
							)}
							<div className="ml-auto">
								<Button
									variant="outline"
									size="sm"
									onClick={() => handleAddType(addButtonType)}
									className="h-7 text-xs"
								>
									<Plus aria-hidden className="size-3" />
									Add {addButtonLabel}
								</Button>
							</div>
						</div>

						{/* Filesystem skills (shown under "All" and "Skills" tabs) */}
						{(activeTab === "skill" || activeTab === "all") &&
							allFsSkills.length > 0 && (
								<div className="space-y-1">
									<p className="text-xs text-muted-foreground">Installed Skills</p>
									{allFsSkills.map((skill) => (
										<div
											key={skill.id}
											className="flex items-center justify-between rounded-md border border-border px-3 py-2 text-sm"
										>
											<div className="min-w-0 flex-1">
												<span className="font-medium">{skill.name}</span>
												{skill.description && (
													<span className="ml-2 truncate text-xs text-muted-foreground">
														{skill.description}
													</span>
												)}
											</div>
											<div className="flex shrink-0 gap-1">
												<Button
													variant="ghost"
													size="icon"
													onClick={() => void handleEditFsSkill(skill.id)}
													aria-label={`Edit ${skill.name}`}
												>
													<Pencil aria-hidden className="size-4" />
												</Button>
												<Button
													variant="ghost"
													size="icon"
													className="text-destructive hover:bg-destructive/10 hover:text-destructive"
													onClick={() => {
														setDeletingFsSkillId(skill.id);
														setDeletingFsSkillName(skill.name);
														setDeleteFsSkillOpen(true);
													}}
													aria-label={`Delete ${skill.name}`}
												>
													<Trash2 aria-hidden className="size-4" />
												</Button>
											</div>
										</div>
									))}
								</div>
							)}

						{/* Generated prompt history (filtered by tab) */}
						{filteredHistory.length > 0 && (
							<div className="space-y-1">
								{(activeTab === "skill" || activeTab === "all") &&
									allFsSkills.length > 0 && (
										<p className="text-xs text-muted-foreground">Generated</p>
									)}
								{filteredHistory.map((artifact) => (
									<div
										key={artifact.id}
										className="flex items-center justify-between rounded-md border border-border px-3 py-2 text-sm"
									>
										<div className="min-w-0 flex-1">
											<span className="font-medium">{artifact.name}</span>
											<span className="ml-2 text-xs text-muted-foreground">
												{artifact.artifact_type}
											</span>
										</div>
										<div className="flex shrink-0 gap-1">
											<Button
												variant="ghost"
												size="icon"
												onClick={() => handleEditArtifact(artifact)}
												aria-label={`Edit ${artifact.name}`}
											>
												<Pencil aria-hidden className="size-4" />
											</Button>
											<Button
												variant="ghost"
												size="icon"
												className="text-destructive hover:bg-destructive/10 hover:text-destructive"
												onClick={() => void deleteArtifact(artifact.id)}
												aria-label={`Delete ${artifact.name}`}
											>
												<Trash2 aria-hidden className="size-4" />
											</Button>
										</div>
									</div>
								))}
							</div>
						)}

						{/* Empty state */}
						{filteredHistory.length === 0 &&
							(activeTab !== "skill" || allFsSkills.length === 0) && (
								<p className="py-4 text-center text-xs text-muted-foreground">
									No{" "}
									{activeTab === "all"
										? "artifacts"
										: (ARTIFACT_TYPES.find((t) => t.value === activeTab)?.label ??
											activeTab) + "s"}{" "}
									yet.
								</p>
							)}
					</div>
				)}
			</div>

			{/* Edit filesystem skill dialog */}
			<Dialog open={editFsSkillOpen} onOpenChange={setEditFsSkillOpen}>
				<DialogContent className="max-w-2xl">
					<DialogHeader>
						<DialogTitle>Edit Skill</DialogTitle>
					</DialogHeader>
					<Textarea
						value={editFsContent}
						onChange={(e) => setEditFsContent(e.target.value)}
						rows={20}
						className="font-mono text-xs"
						placeholder="Skill content (Markdown with YAML frontmatter)..."
					/>
					<DialogFooter>
						<Button
							variant="ghost"
							onClick={() => setEditFsSkillOpen(false)}
							disabled={isSavingFsEdit}
						>
							Cancel
						</Button>
						<Button
							onClick={() => void handleSaveFsSkill()}
							disabled={isSavingFsEdit}
						>
							{isSavingFsEdit ? (
								<>
									<Loader2 aria-hidden className="animate-spin" />
									Saving...
								</>
							) : (
								"Save"
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			{/* Edit generated artifact dialog */}
			<Dialog open={editArtifactOpen} onOpenChange={setEditArtifactOpen}>
				<DialogContent className="max-w-2xl">
					<DialogHeader>
						<DialogTitle>Edit Artifact</DialogTitle>
					</DialogHeader>
					<Textarea
						value={editArtifactContent}
						onChange={(e) => setEditArtifactContent(e.target.value)}
						rows={20}
						className="font-mono text-xs"
						placeholder="Artifact content..."
					/>
					<DialogFooter>
						<Button
							variant="ghost"
							onClick={() => setEditArtifactOpen(false)}
							disabled={isSavingArtifactEdit}
						>
							Cancel
						</Button>
						<Button
							onClick={() => void handleSaveArtifact()}
							disabled={isSavingArtifactEdit}
						>
							{isSavingArtifactEdit ? (
								<>
									<Loader2 aria-hidden className="animate-spin" />
									Saving...
								</>
							) : (
								"Save"
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			{/* Delete filesystem skill confirmation */}
			<AlertDialog open={deleteFsSkillOpen} onOpenChange={setDeleteFsSkillOpen}>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>Delete Skill</AlertDialogTitle>
						<AlertDialogDescription>
							Are you sure you want to delete &ldquo;{deletingFsSkillName}&rdquo;?
							This will remove the file from disk and cannot be undone.
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel disabled={isDeletingFsSkill}>
							Cancel
						</AlertDialogCancel>
						<AlertDialogAction
							onClick={() => void handleDeleteFsSkill()}
							disabled={isDeletingFsSkill}
							className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
						>
							{isDeletingFsSkill ? (
								<>
									<Loader2 aria-hidden className="animate-spin" />
									Deleting...
								</>
							) : (
								"Delete"
							)}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</div>
	);
}
