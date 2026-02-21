import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { nanoid } from "nanoid";
import { toast } from "sonner";
import { CheckIcon, MessageSquare, Plus, Sparkles, Trash2 } from "lucide-react";
import type { ToolUIPart } from "ai";

import { useChatSessionStore, type ChatSession } from "@/stores/chatSessionStore";
import { useLLMStore } from "@/stores/llm";
import { tryExecuteCommand } from "@/lib/chatCommands";
import {
  Conversation,
  ConversationContent,
  ConversationScrollButton,
} from "@/components/ai-elements/conversation";
import {
  Message,
  MessageBranch,
  MessageBranchContent,
  MessageBranchNext,
  MessageBranchPage,
  MessageBranchPrevious,
  MessageBranchSelector,
  MessageContent,
  MessageResponse,
} from "@/components/ai-elements/message";
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
import {
  PromptInput,
  PromptInputBody,
  PromptInputButton,
  PromptInputFooter,
  PromptInputSubmit,
  PromptInputTextarea,
  PromptInputTools,
} from "@/components/ai-elements/prompt-input";
import {
  Reasoning,
  ReasoningContent,
  ReasoningTrigger,
} from "@/components/ai-elements/reasoning";
import {
  Source,
  Sources,
  SourcesContent,
  SourcesTrigger,
} from "@/components/ai-elements/sources";
import { Suggestion, Suggestions } from "@/components/ai-elements/suggestion";
import { APP_IDENTITY } from "@/config/app-identity";
import { useSettings } from "@/stores/settings";
import { useContextPanelStore } from "@/stores/contextPanelStore";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/chat")({
  component: ChatPage,
});

interface MessageType {
  key: string;
  from: "user" | "assistant";
  sources?: { href: string; title: string }[];
  versions: {
    id: string;
    content: string;
  }[];
  reasoning?: {
    content: string;
    duration: number;
  };
  tools?: {
    name: string;
    description: string;
    status: ToolUIPart["state"];
    parameters: Record<string, unknown>;
    result: string | undefined;
    error: string | undefined;
  }[];
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
  "What are the latest trends in AI?",
  "How does machine learning work?",
  "Explain quantum computing",
  "Best practices for React development",
  "Tell me about TypeScript benefits",
  "How to optimize database queries?",
  "What is the difference between SQL and NoSQL?",
  "Explain cloud computing basics",
];

// Suggestion Item component
const SuggestionItem = ({
  suggestion,
  onClick,
}: {
  suggestion: string;
  onClick: (suggestion: string) => void;
}) => {
  const handleClick = useCallback(() => {
    onClick(suggestion);
  }, [onClick, suggestion]);

  return <Suggestion onClick={handleClick} suggestion={suggestion} />;
};

// Model Item component
const ModelItem = ({
  m,
  isSelected,
  onSelect,
}: {
  m: AvailableModel;
  isSelected: boolean;
  onSelect: (id: string) => void;
}) => {
  const handleSelect = useCallback(() => {
    onSelect(m.id);
  }, [onSelect, m.id]);

  return (
    <ModelSelectorItem onSelect={handleSelect} value={m.id}>
      <ModelSelectorLogo provider={m.providerId} />
      <ModelSelectorName>{m.name}</ModelSelectorName>
      {isSelected ? (
        <CheckIcon className="ml-auto size-4" />
      ) : (
        <div className="ml-auto size-4" />
      )}
    </ModelSelectorItem>
  );
};

// Chat Context Panel Component
function ChatContextPanel({
  messages,
  selectedModelData,
  isStreaming,
  onClear,
  onModelClick,
  sessions,
  activeSessionId,
  onNewChat,
  onSelectSession,
  onDeleteSession,
}: {
  messages: MessageType[];
  selectedModelData: AvailableModel;
  isStreaming: boolean;
  onClear: () => void;
  onModelClick: () => void;
  sessions: ChatSession[];
  activeSessionId: string | null;
  onNewChat: () => void;
  onSelectSession: (sessionId: string) => void;
  onDeleteSession: (sessionId: string) => void;
}) {
  const userCount = messages.filter((m) => m.from === "user").length;
  const assistantCount = messages.filter((m) => m.from === "assistant").length;

  const formatSessionTime = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const getSessionModel = (sessionKey: string) => {
    const parts = sessionKey.split(":");
    if (parts.length >= 2) {
      return parts.slice(1).join(":");
    }
    return sessionKey;
  };

  return (
    <div className="flex h-full flex-col">
      {/* Active Model Section */}
      <div className="p-4">
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Active Model
        </p>
        <button
          type="button"
          onClick={onModelClick}
          className="w-full rounded-lg border border-border bg-card p-3 text-left transition-colors hover:border-primary/50 hover:bg-accent/50"
        >
          <div className="flex items-center gap-2">
            <ModelSelectorLogo provider={selectedModelData.providerId} />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{selectedModelData.name}</p>
              <p className="truncate text-xs text-muted-foreground">{selectedModelData.provider}</p>
            </div>
            <span className="text-xs text-muted-foreground">Change</span>
          </div>
        </button>
      </div>

      {/* Divider */}
      <div className="mx-4 border-t border-border" />

      {/* Current Session Stats */}
      <div className="px-4 pb-4">
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Current Session
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

      {/* Divider */}
      <div className="mx-4 border-t border-border" />

      {isStreaming && (
        <div className="flex items-center gap-2 px-4 pb-4 text-xs text-primary">
          <span className="size-2 animate-pulse rounded-full bg-primary" />
          Streaming...
        </div>
      )}

      {/* Divider */}
      <div className="mx-4 border-t border-border" />

      {/* Clear Conversation Button */}
      {messages.length > 0 && (
        <div className="p-4 pb-2">
          <button
            type="button"
            onClick={onClear}
            className="w-full rounded-lg border border-border px-3 py-2 text-xs text-muted-foreground transition-colors hover:border-destructive/50 hover:text-destructive"
          >
            Clear conversation
          </button>
        </div>
      )}

      {/* Divider */}
      <div className="mx-4 border-t border-border" />

      {/* Session History Section */}
      <div className="flex-1 overflow-hidden">
        <div className="flex items-center justify-between p-4 pb-2">
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Chat History
          </p>
          <button
            type="button"
            onClick={onNewChat}
            className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-primary transition-colors hover:bg-primary/10"
          >
            <Plus className="size-3" />
            New
          </button>
        </div>

        <div className="max-h-[200px] overflow-y-auto px-2">
          {sessions.length === 0 ? (
            <p className="px-2 py-3 text-center text-xs text-muted-foreground">
              No previous conversations
            </p>
          ) : (
            <div className="space-y-1">
              {sessions.slice(0, 10).map((session) => (
                <div
                  key={session.id}
                  role="button"
                  tabIndex={0}
                  onClick={() => onSelectSession(session.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      onSelectSession(session.id);
                    }
                  }}
                  className={cn(
                    "group flex w-full cursor-pointer items-center gap-2 rounded-md px-2 py-2 text-left transition-colors",
                    activeSessionId === session.id
                      ? "bg-primary/10 text-primary"
                      : "hover:bg-accent text-foreground"
                  )}
                >
                  <MessageSquare className="size-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-xs font-medium">
                      {session.title || getSessionModel(session.sessionKey)}
                    </p>
                    <p className="truncate text-xs text-muted-foreground">
                      {formatSessionTime(session.updatedAt)}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteSession(session.id);
                    }}
                    className="opacity-0 transition-opacity group-hover:opacity-100 hover:text-destructive"
                    aria-label="Delete session"
                  >
                    <Trash2 className="size-3" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Divider */}
      <div className="mx-4 border-t border-border" />
    </div>
  );
}

function ChatPage() {
  const settings = useSettings((state) => state.settings);

  // LLM store for global model sync
  const { saveProviderConfig, config: llmConfig } = useLLMStore();

  // Session store integration
  const {
    sessions,
    activeSessionId,
    isLoading: isLoadingSessions,
    loadSessions,
    createSession,
    loadSession,
    saveMessage,
    clearMessages,
    deleteSession,
    // Persisted UI state
    isStreaming: storeIsStreaming,
    currentInput: storeCurrentInput,
    setStreaming,
    setCurrentInput,
  } = useChatSessionStore();

  const [messages, setMessages] = useState<MessageType[]>([]);
  const input = storeCurrentInput;
  const setInput = setCurrentInput;
  const isStreaming = storeIsStreaming;
  const [sessionId, setSessionId] = useState(() => nanoid());
  const [apiKey, setApiKey] = useState("");
  const [modelSelectorOpen, setModelSelectorOpen] = useState(false);

  // Track if we've initialized session loading
  const sessionInitialized = useRef(false);
  // Track saved message count to avoid re-saving
  const savedMessageCount = useRef(0);

  // Sync sessionId with activeSessionId when it changes
  useEffect(() => {
    if (activeSessionId && activeSessionId !== sessionId) {
      setSessionId(activeSessionId);
    }
  }, [activeSessionId, sessionId]);

  // Available models
  const [availableModels, setAvailableModels] = useState<AvailableModel[]>([]);

  // Loaded providers (needed to check requiresApiKey)
  const [providers, setProviders] = useState<ProviderWithModels[]>([]);

  // Selected model state
  const [selectedModel, setSelectedModel] = useState<{
    providerId: string;
    modelId: string;
  }>(() => {
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

        setProviders(loadedProviders);

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

        setAvailableModels(models);

        if (models.length === 0) {
          toast.error("No AI models configured", {
            description: "Please add models to your providers in Settings - AI Providers",
          });
        }
      } catch (error) {
        toast.error("Failed to load AI models", {
          description: error instanceof Error ? error.message : String(error),
        });
      }
    }
    loadModels();
  }, []);

  // Load sessions on mount
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Load or create session on mount
  useEffect(() => {
    if (sessionInitialized.current || isLoadingSessions) return;
    sessionInitialized.current = true;

    async function initSession() {
      if (sessions && sessions.length > 0 && !activeSessionId) {
        const lastSession = sessions[0];
        await loadSession(lastSession.id);
        setSessionId(lastSession.id);

        const sessionMessages = useChatSessionStore.getState().messages.get(lastSession.id);
        if (sessionMessages) {
          setMessages(sessionMessages.map((m) => ({
            key: m.id,
            from: m.role as "user" | "assistant",
            versions: [{
              id: m.id,
              content: m.content,
            }],
            isStreaming: false,
          })));
          savedMessageCount.current = sessionMessages.length;
        }
      }
    }
    initSession();
  }, [sessions, activeSessionId, isLoadingSessions, loadSession]);

  // Auto-save messages with debounce and automatic memory capture
  useEffect(() => {
    if (messages.length === 0 || !activeSessionId || isStreaming) return;

    if (messages.length <= savedMessageCount.current) return;

    const timeout = setTimeout(async () => {
      const previousCount = savedMessageCount.current;
      const newMessages = messages.slice(previousCount);
      for (const msg of newMessages) {
        if (!msg.isStreaming) {
          const content = msg.versions[0]?.content || "";
          await saveMessage(msg.from, content);

          if (msg.from === "assistant" && content.length > 50) {
            try {
              const memoryKey = `chat:${activeSessionId}:${msg.key}`;
              await invoke("store_memory_command", {
                key: memoryKey,
                content: content,
                category: "conversation",
              });
            } catch {
              // Non-blocking - memory storage failure shouldn't affect chat
            }
          }
        }
      }
      savedMessageCount.current = messages.length;

      // Reload sessions to get updated title (generated on first message save)
      if (previousCount === 0 && messages.length > 0) {
        loadSessions();
      }
    }, 1000);

    return () => clearTimeout(timeout);
  }, [messages, activeSessionId, isStreaming, saveMessage, loadSessions]);

  // Update selected model when settings or LLMStore config changes
  useEffect(() => {
    if (llmConfig?.providerId && llmConfig?.modelId) {
      setSelectedModel({
        providerId: llmConfig.providerId,
        modelId: llmConfig.modelId,
      });
    } else if (settings?.llmModel) {
      const parts = settings.llmModel.split("/");
      setSelectedModel({
        providerId: parts[0] || "openai",
        modelId: parts.length >= 2 ? parts.slice(1).join("/") : "gpt-4o",
      });
    }
  }, [llmConfig, settings?.llmModel]);

  // Load API key from keychain when provider changes
  useEffect(() => {
    async function loadApiKey() {
      const provider = providers.find((p) => p.id === selectedModel.providerId);
      if (!provider?.requiresApiKey) {
        setApiKey("");
        return;
      }

      try {
        const key = await invoke<string>("keychain_get", {
          service: APP_IDENTITY.keychainService,
          key: `api_key:${selectedModel.providerId}`,
        });
        setApiKey(key || "");
      } catch {
        setApiKey("");
      }
    }
    loadApiKey();
  }, [selectedModel.providerId, providers]);

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

  // Handlers for session management
  const handleNewChat = useCallback(async () => {
    try {
      setMessages([]);
      savedMessageCount.current = 0;

      const newSessionId = await createSession(selectedModel.providerId, selectedModel.modelId);
      setSessionId(newSessionId);

      toast.success("New chat started");
    } catch (error) {
      toast.error("Failed to start new chat");
    }
  }, [createSession, selectedModel.providerId, selectedModel.modelId]);

  const handleSelectSession = useCallback(async (sessionId: string) => {
    if (sessionId === activeSessionId) return;

    try {
      await loadSession(sessionId);
      setSessionId(sessionId);

      const sessionMessages = useChatSessionStore.getState().messages.get(sessionId);
      if (sessionMessages) {
        setMessages(sessionMessages.map((m) => ({
          key: m.id,
          from: m.role as "user" | "assistant",
          versions: [{
            id: m.id,
            content: m.content,
          }],
          isStreaming: false,
        })));
        savedMessageCount.current = sessionMessages.length;
      } else {
        setMessages([]);
        savedMessageCount.current = 0;
      }
    } catch {
      toast.error("Failed to load conversation");
    }
  }, [activeSessionId, loadSession]);

  const handleDeleteSession = useCallback(async (sessionId: string) => {
    try {
      await deleteSession(sessionId);

      if (sessionId === activeSessionId) {
        setMessages([]);
        savedMessageCount.current = 0;
        setSessionId(nanoid());
      }

      toast.success("Conversation deleted");
    } catch {
      toast.error("Failed to delete conversation");
    }
  }, [activeSessionId, deleteSession]);

  // Inject context panel content
  useEffect(() => {
    useContextPanelStore.getState().setContent(
      <ChatContextPanel
        messages={messages}
        selectedModelData={selectedModelData}
        isStreaming={isStreaming}
        onClear={() => {
          setMessages([]);
          savedMessageCount.current = 0;
          clearMessages();
        }}
        onModelClick={() => setModelSelectorOpen(true)}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onNewChat={handleNewChat}
        onSelectSession={handleSelectSession}
        onDeleteSession={handleDeleteSession}
      />,
    );
    return () => useContextPanelStore.getState().clearContent();
  }, [messages, selectedModelData, isStreaming, clearMessages, sessions, activeSessionId, handleNewChat, handleSelectSession, handleDeleteSession]);

  const handleModelSelect = useCallback(async (modelId: string) => {
    const parts = modelId.split("/");
    if (parts.length >= 2) {
      const providerId = parts[0];
      const newModelId = parts.slice(1).join("/");
      setSelectedModel({
        providerId,
        modelId: newModelId,
      });
      setModelSelectorOpen(false);
      try {
        await saveProviderConfig(providerId, newModelId);
        toast.success(`Switched to ${newModelId}`);
      } catch {
        toast.error("Failed to save model selection");
      }
    }
  }, [saveProviderConfig]);

  const handleSubmit = useCallback(
    async (message: { text?: string; files?: File[] }) => {
      const text = message.text?.trim();
      if (!text) {
        return;
      }

      // Check if message is a command (starts with /)
      if (tryExecuteCommand(text, useChatSessionStore.getState())) {
        setInput("");
        return;
      }

      // Check if provider requires API key
      const provider = providers.find((p) => p.id === selectedModel.providerId);
      if (provider?.requiresApiKey && !apiKey) {
        toast.error(`No API key found for ${selectedModel.providerId}`, {
          description: "Please add an API key in Settings - AI Providers",
        });
        return;
      }

      // Create session if one doesn't exist
      let currentSessionId = activeSessionId || sessionId;
      if (!activeSessionId) {
        try {
          currentSessionId = await createSession(selectedModel.providerId, selectedModel.modelId);
          setSessionId(currentSessionId);
          // Ensure sessions list is refreshed
          loadSessions();
        } catch {
          toast.error("Failed to create chat session");
          return;
        }
      }

      // Add user message
      const userMessageId = nanoid();
      const userMessage: MessageType = {
        key: userMessageId,
        from: "user",
        versions: [{
          id: userMessageId,
          content: text,
        }],
      };
      setMessages((prev) => [...prev, userMessage]);
      setInput("");
      setStreaming(true);

      // Add placeholder assistant message
      const assistantMessageId = nanoid();
      const assistantMessage: MessageType = {
        key: assistantMessageId,
        from: "assistant",
        versions: [{
          id: assistantMessageId,
          content: "",
        }],
        isStreaming: true,
      };
      setMessages((prev) => [...prev, assistantMessage]);

      try {
        // Prepare messages for API
        const chatMessages = [...messages, userMessage].map((m) => ({
          role: m.from,
          content: m.versions[0]?.content || "",
        }));

        // Call streaming command
        await invoke("stream_chat_command", {
          request: {
            providerId: selectedModel.providerId,
            modelId: selectedModel.modelId,
            apiKey: apiKey,
            messages: chatMessages,
            sessionId: currentSessionId,
          },
        });
      } catch (error) {
        toast.error("Failed to send message", {
          description: error instanceof Error ? error.message : String(error),
        });
        setStreaming(false);

        // Remove streaming message
        setMessages((prev) => prev.filter((m) => m.key !== assistantMessageId));
      }
    },
    [messages, selectedModel, apiKey, activeSessionId, createSession, loadSessions, providers, setInput]
  );

  // Track virtual keyboard height
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
      document.documentElement.style.setProperty("--keyboard-height", "0px");
    };
  }, []);

  // Listen for streaming events - use sessionId directly
  useEffect(() => {
    const eventName = `chat-stream-${sessionId}`;

    const unlistenPromise = listen<{
      type: "start" | "token" | "done" | "error";
      content?: string;
      error?: string;
      reasoning?: string;
      sources?: { href: string; title: string }[];
    }>(eventName, (event) => {
      const payload = event.payload;

      if (payload.type === "start") {
        // Stream started
      } else if (payload.type === "token" && payload.content) {
        setMessages((prev) => {
          const lastIndex = prev.length - 1;
          const lastMessage = prev[lastIndex];
          if (lastMessage && lastMessage.from === "assistant" && lastMessage.isStreaming) {
            const updatedMessage: MessageType = {
              ...lastMessage,
              versions: [{
                id: lastMessage.versions[0]?.id || nanoid(),
                content: payload.content || "",
              }],
            };
            if (payload.reasoning) {
              updatedMessage.reasoning = {
                content: payload.reasoning,
                duration: 0,
              };
            }
            if (payload.sources) {
              updatedMessage.sources = payload.sources;
            }
            return [...prev.slice(0, lastIndex), updatedMessage];
          }
          return prev;
        });
      } else if (payload.type === "done") {
        setStreaming(false);
        setMessages((prev) => {
          const lastIndex = prev.length - 1;
          const lastMessage = prev[lastIndex];
          if (lastMessage) {
            const updatedMessage: MessageType = {
              ...lastMessage,
              isStreaming: false,
            };
            return [...prev.slice(0, lastIndex), updatedMessage];
          }
          return prev;
        });
      } else if (payload.type === "error") {
        setStreaming(false);
        toast.error(payload.error || "An error occurred");
        setMessages((prev) => prev.slice(0, -1));
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => {
        unlisten();
      });
    };
  }, [sessionId, setStreaming]);

  const handleSuggestionClick = useCallback((suggestion: string) => {
    handleSubmit({ text: suggestion });
  }, [handleSubmit]);

  const handleStopStreaming = useCallback(() => {
    setStreaming(false);
    setMessages((prev) => {
      const lastIndex = prev.length - 1;
      const lastMessage = prev[lastIndex];
      if (lastMessage && lastMessage.isStreaming) {
        // If no content was received, remove the placeholder
        if (!lastMessage.versions[0]?.content) {
          return prev.slice(0, -1);
        }
        // Mark as not streaming
        const updatedMessage: MessageType = {
          ...lastMessage,
          isStreaming: false,
        };
        return [...prev.slice(0, lastIndex), updatedMessage];
      }
      return prev;
    });
    toast.info("Generation stopped");
  }, [setStreaming]);

  const handleTextChange = useCallback((event: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(event.target.value);
  }, [setInput]);

  const handleModelSelectWrapper = useCallback((modelId: string) => {
    handleModelSelect(modelId);
  }, [handleModelSelect]);

  const isSubmitDisabled = useMemo(
    () => !input.trim() || isStreaming,
    [input, isStreaming]
  );

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
    <div className="relative flex size-full flex-col divide-y overflow-hidden">
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
            messages.map(({ versions, ...message }) => (
              <MessageBranch defaultBranch={0} key={message.key}>
                <MessageBranchContent>
                  {versions.map((version) => (
                    <Message
                      from={message.from}
                      key={`${message.key}-${version.id}`}
                    >
                      <div>
                        {message.sources?.length && (
                          <Sources>
                            <SourcesTrigger count={message.sources.length} />
                            <SourcesContent>
                              {message.sources.map((source) => (
                                <Source
                                  href={source.href}
                                  key={source.href}
                                  title={source.title}
                                />
                              ))}
                            </SourcesContent>
                          </Sources>
                        )}
                        {message.reasoning && (
                          <Reasoning duration={message.reasoning.duration}>
                            <ReasoningTrigger />
                            <ReasoningContent>
                              {message.reasoning.content}
                            </ReasoningContent>
                          </Reasoning>
                        )}
                        <MessageContent>
                          <MessageResponse>{version.content || (message.isStreaming ? "..." : "")}</MessageResponse>
                        </MessageContent>
                      </div>
                    </Message>
                  ))}
                </MessageBranchContent>
                {versions.length > 1 && (
                  <MessageBranchSelector>
                    <MessageBranchPrevious />
                    <MessageBranchPage />
                    <MessageBranchNext />
                  </MessageBranchSelector>
                )}
              </MessageBranch>
            ))
          )}
        </ConversationContent>
        <ConversationScrollButton />
      </Conversation>

      <div className="grid shrink-0 gap-4 pt-4">
        {messages.length === 0 && (
          <Suggestions className="px-4">
            {suggestions.map((suggestion) => (
              <SuggestionItem
                key={suggestion}
                onClick={handleSuggestionClick}
                suggestion={suggestion}
              />
            ))}
          </Suggestions>
        )}
        <div className="w-full px-4 pb-4">
          <PromptInput onSubmit={handleSubmit}>
            <PromptInputBody>
              <PromptInputTextarea
                onChange={handleTextChange}
                value={input}
                placeholder="Type your message..."
              />
            </PromptInputBody>
            <PromptInputFooter>
              <PromptInputTools>
                <ModelSelectorDialog
                  onOpenChange={setModelSelectorOpen}
                  open={modelSelectorOpen}
                >
                  <ModelSelectorTrigger asChild>
                    <PromptInputButton>
                      {selectedModelData.providerId && (
                        <ModelSelectorLogo provider={selectedModelData.providerId} />
                      )}
                      {selectedModelData.name && (
                        <ModelSelectorName>
                          {selectedModelData.name}
                        </ModelSelectorName>
                      )}
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
                              <ModelItem
                                isSelected={selectedModelData.id === m.id}
                                key={m.id}
                                m={m}
                                onSelect={handleModelSelectWrapper}
                              />
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
                onStop={handleStopStreaming}
              />
            </PromptInputFooter>
          </PromptInput>
        </div>
      </div>
    </div>
  );
}
