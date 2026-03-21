import { apiGet, apiPost, apiDelete } from "$lib/api/client";
import type { ToolUIPartState } from "$lib/components/ai-elements/tool";

export interface Message {
  id: string;
  session_id: string;
  role: string;
  content: string;
  created_at: number;
  tool_calls?: ToolCallRecord[];
  delegation?: DelegationRecord;
}

export interface ToolCallRecord {
  id: string;
  message_id: string;
  session_id: string;
  tool_name: string;
  args: unknown;
  output?: string;
  success?: boolean;
  duration_ms?: number;
  created_at: string;
}

export interface DelegationAgentRecord {
  id: string;
  description: string;
  status: string;
  tool_uses: number;
  tokens_used: number;
  duration_ms: number;
  error?: string;
}

export interface DelegationRecord {
  delegation_id: string;
  total_duration_ms: number;
  total_tokens: number;
  agents: DelegationAgentRecord[];
}

export interface ActiveToolCall {
  callId: string;
  toolName: string;
  args: unknown;
  state: ToolUIPartState;
  output?: string;
  success?: boolean;
  durationMs?: number;
}

function createMessagesStore() {
  let messages = $state<Message[]>([]);
  let loading = $state(false);
  let streaming = $state(false);
  let streamContent = $state("");
  let error = $state("");
  let activeToolCalls = $state<ActiveToolCall[]>([]);
  let activeStreamSessionId = $state<string | null>(null);

  return {
    get messages() {
      return messages;
    },
    get loading() {
      return loading;
    },
    get streaming() {
      return streaming;
    },
    get streamContent() {
      return streamContent;
    },
    get error() {
      return error;
    },
    get activeToolCalls() {
      return activeToolCalls;
    },
    get activeStreamSessionId() {
      return activeStreamSessionId;
    },

    async load(sessionId: string) {
      loading = true;
      error = "";
      try {
        messages = await apiGet<Message[]>(
          `/sessions/${encodeURIComponent(sessionId)}/messages`,
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        error = `Failed to load messages. Is the daemon running? (${msg})`;
        console.error("messagesStore.load failed:", e);
      } finally {
        loading = false;
      }
    },

    async send(sessionId: string, role: string, content: string) {
      const msg = await apiPost<Message>(
        `/sessions/${encodeURIComponent(sessionId)}/messages`,
        { role, content },
      );
      messages = [...messages, msg];
      return msg;
    },

    startStream(sessionId: string) {
      streaming = true;
      streamContent = "";
      error = "";
      activeToolCalls = [];
      activeStreamSessionId = sessionId;
    },

    setError(msg: string) {
      error = msg;
    },

    appendToken(token: string) {
      streamContent += token;
    },

    addToolCall(callId: string, toolName: string, args: unknown) {
      activeToolCalls = [
        ...activeToolCalls,
        {
          callId,
          toolName,
          args,
          state: "input-available" as ToolUIPartState,
        },
      ].slice(-50); // Keep last 50 tool calls to prevent memory leak
    },

    completeToolCall(
      callId: string,
      output: string,
      success: boolean,
      durationMs: number,
    ) {
      activeToolCalls = activeToolCalls.map((tc) =>
        tc.callId === callId
          ? {
              ...tc,
              output,
              success,
              durationMs,
              state: (success
                ? "output-available"
                : "output-error") as ToolUIPartState,
            }
          : tc,
      );
    },

    async finishStream(sessionId: string) {
      activeStreamSessionId = null;

      // Reconcile with server-persisted messages (server is source of truth)
      try {
        const serverMessages = await apiGet<Message[]>(
          `/sessions/${encodeURIComponent(sessionId)}/messages`,
        );

        // Only replace if server has the new assistant response
        const lastServer = serverMessages[serverMessages.length - 1];
        const hasNewResponse =
          lastServer?.role === "assistant" && lastServer.content;

        if (hasNewResponse) {
          messages = serverMessages;
          streamContent = "";
        } else if (streamContent) {
          // Server doesn't have the response yet — keep streamed content as synthetic message
          messages = [
            ...serverMessages,
            {
              id: `temp-${Date.now()}`,
              session_id: sessionId,
              role: "assistant",
              content: streamContent,
              created_at: Date.now(),
            } as Message,
          ];
          streamContent = "";
        } else {
          messages = serverMessages;
        }
      } catch (e) {
        // If server load fails, preserve streamed content as a message
        console.error("finishStream: failed to reconcile with server:", e);
        if (streamContent) {
          messages = [
            ...messages,
            {
              id: `temp-${Date.now()}`,
              session_id: sessionId,
              role: "assistant",
              content: streamContent,
              created_at: Date.now(),
            } as Message,
          ];
          streamContent = "";
        }
      }

      // Set streaming=false AFTER reconciliation so streaming UI stays visible until data is ready
      streaming = false;
      activeToolCalls = [];
    },

    cancelStream() {
      streaming = false;
      streamContent = "";
      activeToolCalls = [];
      activeStreamSessionId = null;
    },

    async deleteFrom(sessionId: string, messageId: string) {
      await apiDelete(
        `/sessions/${encodeURIComponent(sessionId)}/messages/${encodeURIComponent(messageId)}/and-after`,
      );
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx !== -1) {
        messages = messages.slice(0, idx);
      }
    },

    clear() {
      messages = [];
      streaming = false;
      streamContent = "";
      error = "";
      activeToolCalls = [];
      activeStreamSessionId = null;
    },
  };
}

export const messagesStore = createMessagesStore();
