<script lang="ts">
	import {
		Conversation,
		ConversationContent,
		ConversationEmptyState,
		ConversationScrollButton
	} from '$lib/components/ai-elements/conversation';
	import { Message, MessageContent } from '$lib/components/ai-elements/message';
	import { Response } from '$lib/components/ai-elements/response';
	import { Loader } from '$lib/components/ai-elements/loader';
	import {
		Tool,
		ToolHeader,
		ToolContent,
		ToolInput,
		ToolOutput
	} from '$lib/components/ai-elements/tool';
	import { Actions, Action } from '$lib/components/ai-elements/action';
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
	import { Copy, RefreshCw } from '@lucide/svelte';
	import { messagesStore } from '$lib/stores/messages.svelte';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { providersStore } from '$lib/stores/providers.svelte';
	import { createChatStream } from '$lib/api/websocket';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	let { sessionId = undefined }: { sessionId?: string } = $props();

	onMount(async () => {
		await providersStore.load();
		await providersStore.loadDefault();
	});

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
		// Find the preceding user message
		for (let i = msgIndex - 1; i >= 0; i--) {
			if (messages[i].role === 'user') {
				handleSubmit({ text: messages[i].content });
				return;
			}
		}
	}

	async function handleSubmit(message: PromptInputMessage) {
		const prompt = (message.text ?? '').trim();
		if (!prompt || messagesStore.streaming) return;

		let currentSessionId = sessionId;
		const isFirstMessage = !currentSessionId || messagesStore.messages.length === 0;

		if (!currentSessionId) {
			const session = await sessionsStore.create(prompt.slice(0, 50));
			currentSessionId = session.id;
			goto(`/chat/${currentSessionId}`, { replaceState: true });
		}

		await messagesStore.send(currentSessionId, 'user', prompt);

		messagesStore.startStream();

		const capturedSessionId = currentSessionId;
		const capturedModel = providersStore.selectedModel || undefined;
		createChatStream(
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
				onDone() {
					messagesStore.finishStream(capturedSessionId);
					if (isFirstMessage) {
						sessionsStore.generateTitle(capturedSessionId, capturedModel);
					}
				},
				onError(error) {
					messagesStore.setError(error);
					messagesStore.finishStream(capturedSessionId);
					console.error('Chat error:', error);
				}
			},
			providersStore.selectedModel || undefined
		);
	}
</script>

<div class="flex h-[calc(100vh-4rem)] flex-col">
	<Conversation class="flex-1">
		<ConversationContent>
			{#if messagesStore.messages.length === 0 && !messagesStore.streaming}
				<ConversationEmptyState
					title="Start a conversation"
					description="Send a message to begin chatting with MesoClaw"
				/>
			{:else}
				<div class="space-y-2">
					{#each messagesStore.messages as msg, idx (msg.id)}
						<Message from={msg.role === 'user' ? 'user' : 'assistant'}>
							<MessageContent variant="flat">
								{#if msg.role === 'user'}
									<p class="whitespace-pre-wrap">{msg.content}</p>
								{:else}
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
									<Response content={msg.content} />
									<Actions class="mt-1 opacity-0 transition-opacity group-hover:opacity-100">
										<Action tooltip="Copy" onclick={() => copyMessage(msg.content)}>
											<Copy class="size-3.5" />
										</Action>
										<Action tooltip="Retry" onclick={() => retryMessage(idx)}>
											<RefreshCw class="size-3.5" />
										</Action>
									</Actions>
								{/if}
							</MessageContent>
						</Message>
					{/each}

					{#if messagesStore.streaming}
						<Message from="assistant">
							<MessageContent variant="flat">
								{#each messagesStore.activeToolCalls as tc (tc.callId)}
									<Tool>
										<ToolHeader type={tc.toolName} state={tc.state} />
										<ToolContent>
											<ToolInput input={tc.args} />
											{#if tc.output !== undefined}
												<ToolOutput output={tc.output} />
											{/if}
										</ToolContent>
									</Tool>
								{/each}
								{#if messagesStore.streamContent}
									<Response content={messagesStore.streamContent} />
								{:else if messagesStore.activeToolCalls.length === 0}
									<Loader />
								{/if}
							</MessageContent>
						</Message>
					{/if}

					{#if messagesStore.error}
						<div class="mx-auto max-w-xl rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950 dark:text-red-300">
							{messagesStore.error}
						</div>
					{/if}
				</div>
			{/if}
		</ConversationContent>
		<ConversationScrollButton />
	</Conversation>

	<div class="p-4">
		<PromptInput onSubmit={handleSubmit}>
			<PromptInputTextarea placeholder="Send a message..." />
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
				<div class="flex-1"></div>
				<PromptInputSubmit
					status={messagesStore.streaming ? 'streaming' : 'idle'}
					disabled={messagesStore.streaming}
				/>
			</PromptInputToolbar>
		</PromptInput>
	</div>
</div>
