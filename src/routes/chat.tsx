import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@/components/layout/PageHeader";
import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { nanoid } from "nanoid";
import { toast } from "sonner";
import { CheckIcon, Sparkles } from "lucide-react";

import {
  Conversation,
  ConversationContent,
  ConversationScrollButton,
  Message,
  MessageContent,
  MessageResponse,
  PromptInput,
  PromptInputBody,
  PromptInputFooter,
  PromptInputTextarea,
  PromptInputSubmit,
  PromptInputTools,
  PromptInputButton,
  Suggestions,
  Suggestion,
} from "@/components/ai-elements";
import {
  ModelSelector,
  ModelSelectorContent,
  ModelSelectorDialog,
  ModelSelectorEmpty,
  ModelSelectorGroup,
  ModelSelectorInput,
  ModelSelectorItem,
  ModelSelectorList,
  ModelSelectorLogo,
  ModelSelectorName,
  ModelSelectorTrigger,
} from "@/components/ai-elements/model-selector";
import { APP_IDENTITY } from "@/config/app-identity";
import { useSettings } from "@/stores/settings";
import { useContextPanelStore } from "@/stores/contextPanelStore";

export const Route = createFileRoute("/chat")({
  component: ChatPage,
});

interface MessageType {
  id: string;
  role: "user" | "assistant";
  content: string;
  isStreaming?: boolean;
}

interface AvailableModel {
  id: string;
  name: string;
  provider: string;
  providerId: string;
  modelId: string;
}

interface ProviderWithModels {
  id: string;
  name: string;
  baseUrl: string;
  requiresApiKey: boolean;
  isActive: boolean;
  isUserDefined: boolean;
  models: {
    id: string;
    providerId: string;
    modelId: string;
    displayName: string;
    contextLimit: number | null;
    isCustom: boolean;
    isActive: boolean;
    createdAt: string;
  }[];
}

const suggestions = [
  "Explain how to build a desktop app with Tauri",
  "What are the benefits of using React 19?",
  "How does TypeScript improve code quality?",
  "Best practices for state management in React",
  "Explain the concept of hooks in React",
];

function ChatContextPanel({
  messages,
  selectedModelData,
  isStreaming,
  onClear,
}: {
  messages: MessageType[];
  selectedModelData: AvailableModel;
  isStreaming: boolean;
  onClear: () => void;
}) {
  const userCount = messages.filter((m) => m.role === "user").length;
  const assistantCount = messages.filter((m) => m.role === "assistant").length;

  return (
    <div className="space-y-4 p-4">
      <div>
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Active Model
        </p>
        <div className="flex items-center gap-2">
          <ModelSelectorLogo provider={selectedModelData.providerId} />
          <div>
            <p className="text-sm font-medium">{selectedModelData.name}</p>
            <p className="text-xs text-muted-foreground">{selectedModelData.provider}</p>
          </div>
        </div>
      </div>

      <div>
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Session
        </p>
        <div className="space-y-1 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">You</span>
            <span>{userCount}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">AI</span>
            <span>{assistantCount}</span>
          </div>
        </div>
      </div>

      {isStreaming && (
        <div className="flex items-center gap-2 text-xs text-primary">
          <span className="size-2 animate-pulse rounded-full bg-primary" />
          Streaming…
        </div>
      )}

      {messages.length > 0 && (
        <button
          type="button"
          onClick={onClear}
          className="w-full rounded-lg border border-border px-3 py-2 text-xs text-muted-foreground transition-colors hover:border-destructive/50 hover:text-destructive"
        >
          Clear conversation
        </button>
      )}
    </div>
  );
}

function ChatPage() {
  const settings = useSettings((state) => state.settings);

  const [messages, setMessages] = useState<MessageType[]>([]);
  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [sessionId] = useState(() => nanoid());
  const [apiKey, setApiKey] = useState("");
  const [modelSelectorOpen, setModelSelectorOpen] = useState(false);

  // Available models
  const [availableModels, setAvailableModels] = useState<AvailableModel[]>([]);

  // Loaded providers (needed to check requiresApiKey)
  const [providers, setProviders] = useState<ProviderWithModels[]>([]);

  // Selected model state
  const [selectedModel, setSelectedModel] = useState<{
    providerId: string;
    modelId: string;
  }>(() => {
    // Initialize from settings
    if (settings?.llmModel) {
      const parts = settings.llmModel.split("/");
      return {
        providerId: parts[0] || "openai",
        modelId: parts.length >= 2 ? parts.slice(1).join("/") : "gpt-4o",
      };
    }
    return { providerId: "openai", modelId: "gpt-4o" };
  });

  // Load available models from configured providers
  useEffect(() => {
    async function loadModels() {
      try {
        const loadedProviders = await invoke<ProviderWithModels[]>("list_ai_providers_command");
        console.log("Loaded providers:", loadedProviders);

        // Store providers for requiresApiKey checks
        setProviders(loadedProviders);

        // Transform providers into flat model list
        const models: AvailableModel[] = [];
        for (const provider of loadedProviders) {
          for (const model of provider.models) {
            models.push({
              id: `${provider.id}/${model.modelId}`,
              name: model.displayName,
              provider: provider.name,
              providerId: provider.id,
              modelId: model.modelId,
            });
          }
        }

        console.log("Transformed models:", models);
        setAvailableModels(models);

        // If no models found, show a message
        if (models.length === 0) {
          console.warn("No models configured in providers");
          toast.error("No AI models configured", {
            description: "Please add models to your providers in Settings → AI Providers",
          });
        }
      } catch (error) {
        console.error("Failed to load models:", error);
        toast.error("Failed to load AI models", {
          description: error instanceof Error ? error.message : String(error),
        });
      }
    }
    loadModels();
  }, []);

  // Update selected model when settings change
  useEffect(() => {
    if (settings?.llmModel) {
      const parts = settings.llmModel.split("/");
      setSelectedModel({
        providerId: parts[0] || "openai",
        modelId: parts.length >= 2 ? parts.slice(1).join("/") : "gpt-4o",
      });
    }
  }, [settings?.llmModel]);

  // Load API key from Stronghold when provider changes
  useEffect(() => {
    async function loadApiKey() {
      try {
        const key = await invoke<string>("keychain_get", {
          service: APP_IDENTITY.keychainService,
          key: `api_key:${selectedModel.providerId}`,
        });
        setApiKey(key || "");
        console.log("API key loaded for provider:", selectedModel.providerId);
      } catch (error) {
        console.warn("No API key found for provider:", selectedModel.providerId);
        setApiKey("");
      }
    }
    loadApiKey();
  }, [selectedModel.providerId]);

  // Get selected model data
  const selectedModelData = useMemo(() => {
    const fullId = `${selectedModel.providerId}/${selectedModel.modelId}`;
    return availableModels.find((m) => m.id === fullId) || {
      id: fullId,
      name: selectedModel.modelId,
      provider: selectedModel.providerId,
      providerId: selectedModel.providerId,
      modelId: selectedModel.modelId,
    };
  }, [selectedModel, availableModels]);

  // Inject context panel content
  useEffect(() => {
    useContextPanelStore.getState().setContent(
      <ChatContextPanel
        messages={messages}
        selectedModelData={selectedModelData}
        isStreaming={isStreaming}
        onClear={() => setMessages([])}
      />,
    );
    return () => useContextPanelStore.getState().clearContent();
  }, [messages, selectedModelData, isStreaming]);

  const handleModelSelect = useCallback((modelId: string) => {
    const parts = modelId.split("/");
    if (parts.length >= 2) {
      setSelectedModel({
        providerId: parts[0],
        modelId: parts.slice(1).join("/"),
      });
      setModelSelectorOpen(false);
      toast.success(`Switched to ${parts.slice(1).join("/")}`);
    }
  }, []);

  const handleSubmit = useCallback(
    async (message: { text?: string }) => {
      const text = message.text?.trim();
      if (!text) {
        console.log("Empty message, skipping");
        return;
      }

      console.log("Submitting message:", text);
      console.log("Selected model:", selectedModel);
      console.log("API key available:", !!apiKey);

      // Check if provider requires API key
      const provider = providers.find((p) => p.id === selectedModel.providerId);
      if (provider?.requiresApiKey && !apiKey) {
        toast.error(`No API key found for ${selectedModel.providerId}`, {
          description: "Please add an API key in Settings → AI Providers",
        });
        return;
      }

      // Add user message
      const userMessage: MessageType = {
        id: nanoid(),
        role: "user",
        content: text,
      };
      setMessages((prev) => [...prev, userMessage]);
      setInput("");
      setIsStreaming(true);

      // Add placeholder assistant message
      const assistantMessageId = nanoid();
      const assistantMessage: MessageType = {
        id: assistantMessageId,
        role: "assistant",
        content: "",
        isStreaming: true,
      };
      setMessages((prev) => [...prev, assistantMessage]);

      try {
        // Prepare messages for API
        const chatMessages = [...messages, userMessage].map((m) => ({
          role: m.role,
          content: m.content,
        }));

        console.log("Invoking stream_chat_command...");

        // Call streaming command
        await invoke("stream_chat_command", {
          request: {
            providerId: selectedModel.providerId,
            modelId: selectedModel.modelId,
            apiKey: apiKey,
            messages: chatMessages,
            sessionId,
          },
        });

        console.log("stream_chat_command invoked successfully");
      } catch (error) {
        console.error("Chat error:", error);
        toast.error("Failed to send message", {
          description: error instanceof Error ? error.message : String(error),
        });
        setIsStreaming(false);

        // Remove streaming message
        setMessages((prev) => prev.filter((m) => m.id !== assistantMessageId));
      }
    },
    [messages, selectedModel, apiKey, sessionId]
  );

  // Track virtual keyboard height so the chat input can be pushed up when the
  // soft keyboard appears on mobile (uses the Visual Viewport API).
  useEffect(() => {
    const vv = window.visualViewport;
    if (!vv) return;

    const handler = () => {
      const offset = window.innerHeight - vv.height;
      document.documentElement.style.setProperty("--keyboard-height", `${offset}px`);
    };

    vv.addEventListener("resize", handler);
    return () => {
      vv.removeEventListener("resize", handler);
      // Reset when the component unmounts.
      document.documentElement.style.setProperty("--keyboard-height", "0px");
    };
  }, []);

  // Listen for streaming events
  useEffect(() => {
    const eventName = `chat-stream-${sessionId}`;
    console.log("Setting up listener for:", eventName);

    const unlistenPromise = listen<{
      type: "start" | "token" | "done" | "error";
      content?: string;
      error?: string;
    }>(eventName, (event) => {
      console.log("Received event:", event.payload);
      const payload = event.payload;

      if (payload.type === "start") {
        console.log("Stream started");
      } else if (payload.type === "token" && payload.content) {
        console.log("Received token:", payload.content.substring(0, 50));
        setMessages((prev) => {
          const updated = [...prev];
          const lastMessage = updated[updated.length - 1];
          if (lastMessage && lastMessage.role === "assistant" && lastMessage.isStreaming) {
            lastMessage.content = payload.content || "";
          }
          return updated;
        });
      } else if (payload.type === "done") {
        console.log("Stream completed");
        setIsStreaming(false);
        setMessages((prev) => {
          const updated = [...prev];
          const lastMessage = updated[updated.length - 1];
          if (lastMessage) {
            lastMessage.isStreaming = false;
          }
          return updated;
        });
      } else if (payload.type === "error") {
        console.error("Stream error:", payload.error);
        setIsStreaming(false);
        toast.error(payload.error || "An error occurred");
        // Remove streaming message
        setMessages((prev) => prev.slice(0, -1));
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => {
        console.log("Cleaning up listener");
        unlisten();
      });
    };
  }, [sessionId]);

  const handleSuggestionClick = useCallback((suggestion: string) => {
    setInput(suggestion);
  }, []);

  const handleTextChange = useCallback((event: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(event.target.value);
  }, []);

  const isSubmitDisabled = !input.trim() || isStreaming;

  // Group models by provider
  const modelsByProvider = useMemo(() => {
    const grouped: Record<string, AvailableModel[]> = {};
    availableModels.forEach((model) => {
      if (!grouped[model.provider]) {
        grouped[model.provider] = [];
      }
      grouped[model.provider].push(model);
    });
    return grouped;
  }, [availableModels]);

  return (
    <div className="relative flex size-full flex-col overflow-hidden">
      <PageHeader title="AI Chat" description={selectedModelData.name} className="px-4 pt-4 pb-2" />
      <Conversation>
        <ConversationContent>
          {messages.length === 0 ? (
            <div className="flex h-full items-center justify-center text-muted-foreground">
              <div className="text-center">
                <Sparkles className="mx-auto mb-4 size-12 text-primary" />
                <h2 className="mb-2 text-lg font-semibold text-foreground">
                  Start a conversation
                </h2>
                <p className="text-sm">Ask me anything!</p>
              </div>
            </div>
          ) : (
            messages.map((message) => (
              <Message key={message.id} from={message.role}>
                <MessageContent>
                  <MessageResponse>
                    {message.content || (message.isStreaming ? "..." : "")}
                  </MessageResponse>
                </MessageContent>
              </Message>
            ))
          )}
        </ConversationContent>
        <ConversationScrollButton />
      </Conversation>

      <div className="grid shrink-0 gap-2 pt-2">
        {messages.length === 0 && (
          <Suggestions className="px-4">
            {suggestions.map((suggestion) => (
              <Suggestion
                key={suggestion}
                suggestion={suggestion}
                onClick={() => handleSuggestionClick(suggestion)}
              />
            ))}
          </Suggestions>
        )}

        <div className="w-full px-4 pb-2">
          <div className="rounded-2xl border-2 border-border shadow-sm transition-all focus-within:border-primary/40 focus-within:shadow-md">
          <PromptInput value={input} onChange={setInput} onSubmit={handleSubmit}>
            <PromptInputBody>
              <PromptInputTextarea
                value={input}
                onChange={handleTextChange}
                placeholder="Type your message..."
                className="min-h-[60px] w-full resize-none border-0 bg-transparent px-3 py-2 focus:outline-none focus:ring-0"
              />
            </PromptInputBody>
            <PromptInputFooter>
              <PromptInputTools>
                <ModelSelectorDialog
                  open={modelSelectorOpen}
                  onOpenChange={setModelSelectorOpen}
                >
                  <ModelSelectorTrigger asChild>
                    <PromptInputButton className="gap-2">
                      <ModelSelectorLogo provider={selectedModel.providerId} />
                      <ModelSelectorName>{selectedModelData.name}</ModelSelectorName>
                    </PromptInputButton>
                  </ModelSelectorTrigger>
                  <ModelSelectorContent>
                    <ModelSelector>
                      <ModelSelectorInput placeholder="Search models..." />
                      <ModelSelectorList>
                        <ModelSelectorEmpty>No models found.</ModelSelectorEmpty>
                        {Object.entries(modelsByProvider).map(([provider, models]) => (
                          <ModelSelectorGroup heading={provider} key={provider}>
                            {models.map((m) => (
                              <ModelSelectorItem
                                key={m.id}
                                value={m.id}
                                onSelect={() => handleModelSelect(m.id)}
                                className="flex items-center gap-2"
                              >
                                <ModelSelectorLogo provider={m.providerId} />
                                <ModelSelectorName className="flex-1">{m.name}</ModelSelectorName>
                                {selectedModelData.id === m.id && (
                                  <CheckIcon className="size-4 shrink-0" />
                                )}
                              </ModelSelectorItem>
                            ))}
                          </ModelSelectorGroup>
                        ))}
                      </ModelSelectorList>
                    </ModelSelector>
                  </ModelSelectorContent>
                </ModelSelectorDialog>
              </PromptInputTools>
              <PromptInputSubmit
                disabled={isSubmitDisabled}
                status={isStreaming ? "streaming" : "ready"}
              />
            </PromptInputFooter>
          </PromptInput>
          </div>
        </div>
      </div>
    </div>
  );
}
