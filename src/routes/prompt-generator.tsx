import { createFileRoute } from "@tanstack/react-router";
import { useEffect } from "react";

import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { CheckCircle2, Copy, Loader2, Trash2 } from "@/lib/icons";
import { cn } from "@/lib/utils";
import {
	type ArtifactType,
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

function PromptGeneratorPage() {
	const artifactType = usePromptGeneratorStore((s) => s.artifactType);
	const name = usePromptGeneratorStore((s) => s.name);
	const description = usePromptGeneratorStore((s) => s.description);
	const status = usePromptGeneratorStore((s) => s.status);
	const generatedContent = usePromptGeneratorStore((s) => s.generatedContent);
	const lastSaved = usePromptGeneratorStore((s) => s.lastSaved);
	const error = usePromptGeneratorStore((s) => s.error);

	const setArtifactType = usePromptGeneratorStore((s) => s.setArtifactType);
	const setName = usePromptGeneratorStore((s) => s.setName);
	const setDescription = usePromptGeneratorStore((s) => s.setDescription);
	const startGeneration = usePromptGeneratorStore((s) => s.startGeneration);
	const reset = usePromptGeneratorStore((s) => s.reset);
	const loadHistory = usePromptGeneratorStore((s) => s.loadHistory);
	const history = usePromptGeneratorStore((s) => s.history);
	const deleteArtifact = usePromptGeneratorStore((s) => s.deleteArtifact);

	useEffect(() => {
		void loadHistory();
	}, [loadHistory]);

	const isGenerating = status === "generating";

	async function handleCopy() {
		if (generatedContent) {
			await navigator.clipboard.writeText(generatedContent);
		}
	}

	return (
		<div className="flex h-full flex-col gap-4 overflow-hidden p-4">
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
							className={cn(
								"rounded-full border px-3 py-1.5 text-sm font-medium transition-colors",
								"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
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
				<div className="space-y-3">
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
				</div>

				{/* Output panel */}
				{(generatedContent || status === "error") && (
					<div className="space-y-2 rounded-lg border border-border p-4">
						<div className="flex items-center justify-between">
							<h2 className="text-sm font-semibold">Output</h2>
							<div className="flex gap-2">
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

						<pre className="max-h-80 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-3 font-mono text-sm">
							{generatedContent}
							{isGenerating && (
								<span className="inline-block animate-pulse">|</span>
							)}
						</pre>

						{status === "done" && lastSaved?.disk_path && (
							<p className="flex items-center gap-1.5 text-xs text-muted-foreground">
								<CheckCircle2 aria-hidden className="size-3.5 text-green-500" />
								Saved to {lastSaved.disk_path}
							</p>
						)}

						{status === "error" && error && (
							<p className="text-xs text-destructive">{error}</p>
						)}
					</div>
				)}

				{/* History */}
				{history.length > 0 && (
					<div className="space-y-2">
						<h2 className="text-sm font-semibold">History</h2>
						<div className="space-y-1">
							{history.map((artifact) => (
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
									<Button
										variant="ghost"
										size="icon"
										onClick={() => void deleteArtifact(artifact.id)}
										aria-label={`Delete ${artifact.name}`}
									>
										<Trash2 aria-hidden className="size-4" />
									</Button>
								</div>
							))}
						</div>
					</div>
				)}
			</div>
		</div>
	);
}
