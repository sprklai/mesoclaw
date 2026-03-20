import { apiGet, apiPost, apiDelete } from "$lib/api/client";
import type { ToolUIPartState } from "$lib/components/ai-elements/tool";

export interface Message {
  id: string;
  session_id: string;
  role: string;
  content: string;
  created_at: number;
  tool_calls?: ToolCallRecord[];
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
      // Keep streamed content visible while we reconcile with server
      streaming = false;
      activeStreamSessionId = null;

      // Reconcile with server-persisted messages (server is source of truth)
      try {
        const serverMessages = await apiGet<Message[]>(
          `/sessions/${encodeURIComponent(sessionId)}/messages`,
        );
        messages = serverMessages;
      } catch (e) {
        // If server load fails, keep what we have — stream content is already
        // visible in the messages list as the last assistant message
        console.error("finishStream: failed to reconcile with server:", e);
      }

      streamContent = "";
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
