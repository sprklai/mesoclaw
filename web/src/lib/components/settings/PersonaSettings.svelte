<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Separator } from '$lib/components/ui/separator';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Plus from '@lucide/svelte/icons/plus';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { apiGet, apiPut, apiPost, apiDelete } from '$lib/api/client';
	import { onMount } from 'svelte';
	import * as m from '$lib/paraglide/messages';

	interface IdentityFile {
		name: string;
		content: string;
		is_default: boolean;
		description: string;
	}

	interface SkillInfo {
		id: string;
		category: string;
		description: string;
		created_at: number;
		domain?: string;
		surface?: string;
	}

	let identityFiles = $state<IdentityFile[]>([]);
	let skills = $state<SkillInfo[]>([]);
	let editingFile = $state<{ name: string; content: string } | null>(null);
	let editingSkill = $state<{ id: string; content: string } | null>(null);
	let loading = $state(true);
	let addSkillOpen = $state(false);
	let newSkillId = $state('');
	let newSkillContent = $state('');
	let skillDeleteOpen = $state(false);
	let skillDeleteTarget = $state<string | null>(null);

	onMount(async () => {
		await loadAll();
	});

	async function loadAll() {
		loading = true;
		try {
			const [idResult, skillResult] = await Promise.all([
				apiGet<{ files: IdentityFile[] }>('/identity'),
				apiGet<{ skills: SkillInfo[] }>('/skills')
			]);
			identityFiles = idResult.files;
			skills = skillResult.skills;
		} finally {
			loading = false;
		}
	}

	async function handleEditFile(name: string) {
		const file = await apiGet<IdentityFile>(`/identity/${encodeURIComponent(name)}`);
		editingFile = { name: file.name, content: file.content };
	}

	async function handleSaveFile() {
		if (!editingFile) return;
		await apiPut(`/identity/${encodeURIComponent(editingFile.name)}`, { content: editingFile.content });
		editingFile = null;
		await loadAll();
	}

	async function handleReloadIdentity() {
		await apiPost('/identity/reload');
		await loadAll();
	}

	async function handleReloadSkills() {
		await apiPost('/skills/reload');
		await loadAll();
	}

	async function handleAddSkill() {
		if (!newSkillId.trim() || !newSkillContent.trim()) return;
		await apiPost('/skills', { id: newSkillId.trim(), content: newSkillContent.trim() });
		newSkillId = '';
		newSkillContent = '';
		addSkillOpen = false;
		await loadAll();
	}

	async function handleEditSkill(id: string) {
		const skill = await apiGet<{ id: string; content: string }>(`/skills/${encodeURIComponent(id)}`);
		editingSkill = { id: skill.id, content: skill.content };
	}

	async function handleSaveSkill() {
		if (!editingSkill) return;
		await apiPut(`/skills/${encodeURIComponent(editingSkill.id)}`, { content: editingSkill.content });
		editingSkill = null;
		await loadAll();
	}

	function handleDeleteSkill(id: string) {
		skillDeleteTarget = id;
		skillDeleteOpen = true;
	}

	async function confirmDeleteSkill() {
		if (!skillDeleteTarget) return;
		const id = skillDeleteTarget;
		await apiDelete(`/skills/${encodeURIComponent(id)}`);
		skills = skills.filter((s) => s.id !== id);
	}
</script>

{#if loading}
	<div class="space-y-2">
		<Skeleton class="h-20 w-full" />
		<Skeleton class="h-20 w-full" />
	</div>
{:else}
	<Card.Root>
		<Card.Header>
			<div class="flex items-center justify-between">
				<Card.Title>{m.settings_persona_identity_title()}</Card.Title>
				<Button variant="ghost" size="icon" onclick={handleReloadIdentity}>
					<RefreshCw class="h-4 w-4" />
				</Button>
			</div>
		</Card.Header>
		<Card.Content class="space-y-2">
			{#each identityFiles as file (file.name)}
				<div class="flex items-center justify-between p-2 rounded-lg bg-muted">
					<div>
						<span class="font-medium">{file.name}</span>
						{#if file.is_default}
							<Badge variant="secondary" class="ml-2">{m.settings_persona_badge_default()}</Badge>
						{/if}
					</div>
					<Button variant="ghost" size="icon" class="h-7 w-7" onclick={() => handleEditFile(file.name)}>
						<Pencil class="h-3.5 w-3.5" />
					</Button>
				</div>
			{/each}
		</Card.Content>
	</Card.Root>

	<Separator />

	<Card.Root>
		<Card.Header>
			<div class="flex items-center justify-between">
				<Card.Title>{m.settings_persona_skills_title()}</Card.Title>
				<div class="flex gap-1">
					<Button variant="ghost" size="icon" onclick={handleReloadSkills}>
						<RefreshCw class="h-4 w-4" />
					</Button>
					<Button variant="ghost" size="icon" onclick={() => (addSkillOpen = true)}>
						<Plus class="h-4 w-4" />
					</Button>
				</div>
			</div>
		</Card.Header>
		<Card.Content class="space-y-2">
			{#each skills as skill (skill.id)}
				<div class="flex items-center justify-between p-2 rounded-lg bg-muted">
					<div class="flex items-center flex-wrap gap-1.5">
						<span class="font-medium">{skill.id}</span>
						<Badge variant="secondary">{skill.category}</Badge>
						{#if skill.domain}
							<Badge variant="outline" class="text-[10px]">{skill.domain}</Badge>
						{/if}
						{#if skill.surface}
							<Badge variant="outline" class="text-[10px] border-blue-500/50 text-blue-600 dark:text-blue-400">{skill.surface}</Badge>
						{/if}
					</div>
					<div class="flex gap-1">
						<Button variant="ghost" size="icon" class="h-7 w-7" onclick={() => handleEditSkill(skill.id)}>
							<Pencil class="h-3.5 w-3.5" />
						</Button>
						<Button
							variant="ghost"
							size="icon"
							class="h-7 w-7 text-destructive"
							onclick={() => handleDeleteSkill(skill.id)}
						>
							<Trash2 class="h-3.5 w-3.5" />
						</Button>
					</div>
				</div>
			{/each}
			{#if skills.length === 0}
				<p class="text-muted-foreground text-sm">{m.settings_persona_no_skills()}</p>
			{/if}
		</Card.Content>
	</Card.Root>
{/if}

<Dialog.Root open={!!editingFile} onOpenChange={(open) => { if (!open) editingFile = null; }}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.settings_persona_edit_dialog_title({ name: editingFile?.name ?? '' })}</Dialog.Title>
		</Dialog.Header>
		{#if editingFile}
			<Textarea bind:value={editingFile.content} rows={15} class="font-mono text-sm" />
			<Button class="w-full" onclick={handleSaveFile}>{m.settings_persona_save_button()}</Button>
		{/if}
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root open={!!editingSkill} onOpenChange={(open) => { if (!open) editingSkill = null; }}>
	<Dialog.Content class="sm:max-w-3xl">
		<Dialog.Header>
			<Dialog.Title>{m.settings_persona_edit_dialog_title({ name: editingSkill?.id ?? '' })}</Dialog.Title>
		</Dialog.Header>
		{#if editingSkill}
			<Textarea bind:value={editingSkill.content} rows={25} class="font-mono text-sm" />
			<Button class="w-full" onclick={handleSaveSkill}>{m.settings_persona_save_button()}</Button>
		{/if}
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={addSkillOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{m.settings_persona_add_skill_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="space-y-3">
			<Input placeholder={m.settings_persona_skill_id_placeholder()} bind:value={newSkillId} />
			<Textarea placeholder={m.settings_persona_skill_content_placeholder()} bind:value={newSkillContent} rows={8} class="font-mono text-sm" />
			<Button class="w-full" onclick={handleAddSkill}>{m.settings_persona_create_button()}</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>

<ConfirmDialog
	bind:open={skillDeleteOpen}
	title={m.settings_persona_confirm_delete_title()}
	description={m.settings_persona_confirm_delete_description()}
	onConfirm={confirmDeleteSkill}
/>
