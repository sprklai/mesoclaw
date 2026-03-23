<script lang="ts">
	import {
		Conversation,
		ConversationContent,
		ConversationEmptyState,
		ConversationScrollButton
	} from '$lib/components/ai-elements/conversation';
	import { Message, MessageContent, MessageResponse, MessageActions, MessageAction } from '$lib/components/ai-elements/new-message';
	import { Loader } from '$lib/components/ai-elements/loader';
	import { Shimmer } from '$lib/components/ai-elements/shimmer';
	import {
		Tool,
		ToolHeader,
		ToolContent,
		ToolInput,
		ToolOutput
	} from '$lib/components/ai-elements/tool';
	import {
		PromptInput,
		PromptInputTextarea,
		PromptInputToolbar,
		PromptInputSubmit,
		PromptInputModelSelect,
		PromptInputModelSelectTrigger,
		PromptInputModelSelectContent,
		PromptInputModelSelectItem,
		PromptInputModelSelectValue,
		type PromptInputMessage
	} from '$lib/components/ai-elements/prompt-input';
	import { Copy, Pencil, RefreshCw } from '@lucide/svelte';
	import AgentTree from '$lib/components/AgentTree.svelte';
	import DelegationSummary from '$lib/components/DelegationSummary.svelte';
	import { messagesStore } from '$lib/stores/messages.svelte';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { providersStore } from '$lib/stores/providers.svelte';
	import { delegationStore, buildDelegationRecord } from '$lib/stores/delegation.svelte';
	import { notificationStore } from '$lib/stores/notifications.svelte';
	import { approvalsStore } from '$lib/stores/approvals.svelte';
	import { ToolApproval } from '$lib/components/ai-elements/tool-approval';
	import { createChatStream, sendApprovalResponse, type ChatConnection } from '$lib/api/websocket';
	import { goto } from '$app/navigation';
	import { onMount, onDestroy } from 'svelte';

	let { sessionId = undefined }: { sessionId?: string } = $props();

	let providersLoaded = $state(false);
	let activeWs = $state<ChatConnection | null>(null);
	let editText = $state("");
	let delegationEnabled = $state(false);

	onMount(async () => {
		try {
			await providersStore.load();
			await providersStore.loadDefault();
		} catch (e) {
			console.error('Failed to load providers:', e);
		} finally {
			providersLoaded = true;
		}
	});

	onDestroy(() => {
		if (activeWs) {
			activeWs.close();
			activeWs = null;
		}
		if (messagesStore.streaming && messagesStore.activeStreamSessionId) {
			const fallbackDelegation = delegationStore.delegation
				? buildDelegationRecord(delegationStore.delegation)
				: undefined;
			messagesStore.finishStream(messagesStore.activeStreamSessionId, fallbackDelegation).then(() => {
				delegationStore.clear();
			});
		}
	});

	const hasUsableModel = $derived(providersStore.hasUsableModel);

	const currentModelLabel = $derived(
		providersStore.configuredModels.find((m) => m.value === providersStore.selectedModel)
			?.label ?? ''
	);

	async function copyMessage(content: string) {
		try {
			await navigator.clipboard.writeText(content);
		} catch {
			// Fallback for non-secure contexts
			const textarea = document.createElement('textarea');
			textarea.value = content;
			textarea.style.position = 'fixed';
			textarea.style.opacity = '0';
			document.body.appendChild(textarea);
			textarea.select();
			document.execCommand('copy');
			document.body.removeChild(textarea);
		}
	}

	function retryMessage(msgIndex: number) {
		const messages = messagesStore.messages;
		const msg = messages[msgIndex];
		if (msg.role === 'user') {
			handleSubmit({ text: msg.content });
			return;
		}
		// For assistant messages, find the preceding user message
		for (let i = msgIndex - 1; i >= 0; i--) {
			if (messages[i].role === 'user') {
				handleSubmit({ text: messages[i].content });
				return;
			}
		}
	}

	async function editMessage(msgIndex: number) {
		const msg = messagesStore.messages[msgIndex];
		if (!msg || msg.role !== 'user' || !sessionId) return;
		const content = msg.content;
		await messagesStore.deleteFrom(sessionId, msg.id);
		editText = content;
	}

	async function handleSubmit(message: PromptInputMessage) {
		const prompt = (message.text ?? '').trim();
		if (!prompt || messagesStore.streaming || !hasUsableModel) return;

		let currentSessionId = sessionId;
		const isFirstMessage = !currentSessionId || messagesStore.messages.length === 0;

		if (!currentSessionId) {
			const session = await sessionsStore.create(prompt.slice(0, 50));
			currentSessionId = session.id;
			// P0.3: Send + claim stream BEFORE navigation so the
			// [id]/+page.svelte $effect sees streaming=true and skips clear()+load()
			await messagesStore.send(currentSessionId, 'user', prompt);
			messagesStore.startStream(currentSessionId);
			await goto(`/chat/${currentSessionId}`, { replaceState: true });
		} else {
			await messagesStore.send(currentSessionId, 'user', prompt);
			messagesStore.startStream(currentSessionId);
		}

		const capturedSessionId = currentSessionId;
		const capturedModel = providersStore.selectedModel || undefined;
		const capturedDelegation = delegationEnabled || undefined;
		activeWs = await createChatStream(
			prompt,
			currentSessionId,
			{
				onToken(content) {
					messagesStore.appendToken(content);
				},
				onToolCall(callId, toolName, args) {
					messagesStore.addToolCall(callId, toolName, args);
				},
				onToolResult(callId, _toolName, output, success, durationMs) {
					messagesStore.completeToolCall(callId, output, success, durationMs);
				},
				onDelegationStarted(delegationId, agents) {
					delegationStore.startDelegation(delegationId, agents);
				},
				onAgentProgress(delegationId, agentId, toolUses, tokensUsed, activity) {
					delegationStore.updateAgent(agentId, toolUses, tokensUsed, activity);
				},
				onAgentCompleted(delegationId, agentId, status, durationMs, toolUses, tokensUsed) {
					delegationStore.completeAgent(agentId, status, durationMs, toolUses, tokensUsed);
				},
				onDelegationCompleted() {
					delegationStore.completeDelegation();
				},
				onApprovalRequest(approvalId, callId, toolName, argsSummary, riskLevel, reason, timeoutSecs) {
					approvalsStore.addRequest(approvalId, callId, toolName, argsSummary, riskLevel, reason, timeoutSecs);
				},
				onApprovalResolved(approvalId) {
					approvalsStore.resolve(approvalId);
				},
				onWarning(warning: string) {
					console.warn('[Chat] Server warning:', warning);
				},
				onDone() {
					activeWs = null;
					approvalsStore.clear();
					// Capture delegation data before clearing for fallback rendering
					const fallbackDelegation = delegationStore.delegation
						? buildDelegationRecord(delegationStore.delegation)
						: undefined;
					// P0.2: Only finish if this stream still owns the session
					if (messagesStore.activeStreamSessionId === capturedSessionId) {
						messagesStore.finishStream(capturedSessionId, fallbackDelegation).then(() => {
							delegationStore.clear();
						});
					} else {
						delegationStore.clear();
					}
					if (isFirstMessage) {
						sessionsStore.generateTitle(capturedSessionId, capturedModel);
					}
				},
				onError(error) {
					activeWs = null;
					approvalsStore.clear();
					const fallbackDelegation = delegationStore.delegation
						? buildDelegationRecord(delegationStore.delegation)
						: undefined;
					const friendlyError =
						error.toLowerCase().includes('no agent configured') ||
						error.toLowerCase().includes('no provider')
							? '__NO_PROVIDER__'
							: error;
					messagesStore.setError(friendlyError);
					// P0.2: Only finish if this stream still owns the session
					if (messagesStore.activeStreamSessionId === capturedSessionId) {
						messagesStore.finishStream(capturedSessionId, fallbackDelegation).then(() => {
							delegationStore.clear();
						});
					} else {
						delegationStore.clear();
					}
					console.error('Chat error:', error);
				}
			},
			providersStore.selectedModel || undefined,
			capturedDelegation
		);
	}

	function handleApprovalResponse(approvalId: string, decision: 'approve' | 'approve_always' | 'deny') {
		if (activeWs) {
			sendApprovalResponse(activeWs, approvalId, decision);
		}
		approvalsStore.resolve(approvalId);
	}

	function stopStream() {
		if (activeWs) {
			activeWs.close();
			activeWs = null;
		}
		const fallbackDelegation = delegationStore.delegation
			? buildDelegationRecord(delegationStore.delegation)
			: undefined;
		approvalsStore.clear();
		if (messagesStore.streaming && messagesStore.activeStreamSessionId) {
			messagesStore.finishStream(messagesStore.activeStreamSessionId, fallbackDelegation).then(() => {
				delegationStore.clear();
			});
		} else {
			delegationStore.clear();
		}
	}
</script>

<div class="flex h-[calc(100vh-4rem)] flex-col">
	<Conversation class="flex-1">
		<ConversationContent>
			{#if messagesStore.messages.length === 0 && !messagesStore.streaming}
				<ConversationEmptyState
					title="Start a conversation"
					description="Send a message to begin chatting with Zenii"
				/>
			{:else}
				<div class="space-y-2">
					{#each messagesStore.messages as msg, idx (msg.id)}
						<Message from={msg.role === 'user' ? 'user' : 'assistant'}>
							{#if msg.role === 'user'}
								<MessageContent>
									<p class="whitespace-pre-wrap">{msg.content}</p>
								</MessageContent>
								<MessageActions class="mt-1 opacity-0 transition-opacity group-hover:opacity-100">
									<MessageAction tooltip="Copy" onclick={() => copyMessage(msg.content)}>
										<Copy class="size-3.5" />
									</MessageAction>
									<MessageAction tooltip="Edit" onclick={() => editMessage(idx)}>
										<Pencil class="size-3.5" />
									</MessageAction>
									<MessageAction tooltip="Retry" onclick={() => retryMessage(idx)}>
										<RefreshCw class="size-3.5" />
									</MessageAction>
								</MessageActions>
							{:else}
								<MessageContent>
									{#if msg.tool_calls && msg.tool_calls.length > 0}
										{#each msg.tool_calls as tc (tc.id)}
											<Tool>
												<ToolHeader
													type={tc.tool_name}
													state={tc.success === false ? 'output-error' : tc.success === true ? 'output-available' : 'input-available'}
												/>
												<ToolContent>
													<ToolInput input={tc.args} />
													{#if tc.output !== undefined}
														<ToolOutput output={tc.output} />
													{/if}
												</ToolContent>
											</Tool>
										{/each}
									{/if}
									{#if msg.delegation}
										<DelegationSummary delegation={msg.delegation} />
									{/if}
									<MessageResponse content={msg.content} />
								</MessageContent>
								<MessageActions class="mt-1 opacity-0 transition-opacity group-hover:opacity-100">
									<MessageAction tooltip="Copy" onclick={() => copyMessage(msg.content)}>
										<Copy class="size-3.5" />
									</MessageAction>
									<MessageAction tooltip="Retry" onclick={() => retryMessage(idx)}>
										<RefreshCw class="size-3.5" />
									</MessageAction>
								</MessageActions>
							{/if}
						</Message>
					{/each}

					{#if delegationStore.active && delegationStore.delegation}
						<AgentTree delegation={delegationStore.delegation} />
						{#if delegationStore.aggregating && !messagesStore.streamContent}
							<div class="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground">
								<Shimmer content_length={30} duration={1.5}>Synthesizing agent results...</Shimmer>
							</div>
						{/if}
					{/if}

					{#if messagesStore.streaming}
						<Message from="assistant">
							<MessageContent>
								{#each messagesStore.activeToolCalls as tc (tc.callId)}
									<Tool>
										<ToolHeader type={tc.toolName} state={tc.state} />
										<ToolContent>
											<ToolInput input={tc.args} />
											{#if tc.output !== undefined}
												<ToolOutput output={tc.output} />
											{:else if tc.state === 'input-available'}
												<div class="px-3 pb-3">
													<Shimmer content_length={40} duration={1.5}>Processing...</Shimmer>
												</div>
											{/if}
										</ToolContent>
									</Tool>
								{/each}
								{#each approvalsStore.pending as approval (approval.approvalId)}
									<div class="my-2">
										<ToolApproval {approval} onRespond={handleApprovalResponse} />
									</div>
								{/each}
								{#if messagesStore.streamContent}
									<MessageResponse content={messagesStore.streamContent} />
								{:else}
									{@const allToolsDone = messagesStore.activeToolCalls.length > 0 &&
										messagesStore.activeToolCalls.every(tc => tc.state !== 'input-available')}
									{#if allToolsDone || messagesStore.activeToolCalls.length === 0}
										<Loader />
									{/if}
								{/if}
							</MessageContent>
						</Message>
					{/if}

					{#if notificationStore.channelAgentActivity && !messagesStore.streaming}
					<Message from="assistant">
						<MessageContent>
							<div class="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground">
								<Shimmer content_length={30} duration={1.5}>
									Processing {notificationStore.channelAgentActivity.channel} message from @{notificationStore.channelAgentActivity.sender}...
								</Shimmer>
							</div>
						</MessageContent>
					</Message>
				{/if}

				{#if messagesStore.error}
						<div class="mx-auto max-w-xl rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950 dark:text-red-300">
							{#if messagesStore.error === '__NO_PROVIDER__'}
								No AI provider configured.
								<a href="/settings#providers" class="underline font-medium hover:text-red-900 dark:hover:text-red-200">Go to Settings &rarr; Providers</a>
								to set up a provider and model.
							{:else}
								{messagesStore.error}
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</ConversationContent>
		<ConversationScrollButton />
	</Conversation>

	<div class="p-4">
		{#if providersLoaded && !hasUsableModel}
			<div class="rounded-md border border-amber-500/50 bg-amber-500/10 px-4 py-3 text-sm text-amber-700 dark:text-amber-400">
				No API key found — your key may not have persisted across restarts.
				<a href="/settings#providers" class="underline font-medium hover:text-amber-900 dark:hover:text-amber-200">
					Add one in Settings &rarr; Providers
				</a>
				to start chatting.
			</div>
		{/if}
		<PromptInput onSubmit={handleSubmit}>
			<PromptInputTextarea
				bind:value={editText}
				placeholder={hasUsableModel ? 'Send a message...' : 'Configure a provider to start chatting...'}
			/>
			<PromptInputToolbar>
				{#if providersStore.configuredModels.length > 0}
					<PromptInputModelSelect
						value={providersStore.selectedModel}
						onValueChange={(v) => {
							if (v) providersStore.selectedModel = v;
						}}
					>
						<PromptInputModelSelectTrigger>
							<PromptInputModelSelectValue
								value={currentModelLabel}
								placeholder="Select model"
							/>
						</PromptInputModelSelectTrigger>
						<PromptInputModelSelectContent>
							{#each providersStore.configuredModels as model}
								<PromptInputModelSelectItem value={model.value}>
									{model.label}
								</PromptInputModelSelectItem>
							{/each}
						</PromptInputModelSelectContent>
					</PromptInputModelSelect>
				{/if}
				<label class="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer select-none" title="Enable multi-agent delegation">
					<input type="checkbox" bind:checked={delegationEnabled} class="accent-cyan-500" />
					Delegate
				</label>
				<div class="flex-1"></div>
				<PromptInputSubmit
					status={messagesStore.streaming ? 'streaming' : 'idle'}
					disabled={!messagesStore.streaming && !hasUsableModel}
					onclick={messagesStore.streaming ? stopStream : undefined}
				/>
			</PromptInputToolbar>
		</PromptInput>
	</div>
</div>
