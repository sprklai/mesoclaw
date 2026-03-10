import { apiGet, apiPost } from "$lib/api/client";
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

    startStream() {
      streaming = true;
      streamContent = "";
      error = "";
      activeToolCalls = [];
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
      ];
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

    finishStream(sessionId: string) {
      if (streamContent || activeToolCalls.length > 0) {
        const toolCalls: ToolCallRecord[] = activeToolCalls.map((tc) => ({
          id: tc.callId,
          message_id: "",
          session_id: sessionId,
          tool_name: tc.toolName,
          args: tc.args,
          output: tc.output,
          success: tc.success,
          duration_ms: tc.durationMs,
          created_at: new Date().toISOString(),
        }));

        messages = [
          ...messages,
          {
            id: crypto.randomUUID(),
            session_id: sessionId,
            role: "assistant",
            content: streamContent,
            created_at: Date.now(),
            tool_calls: toolCalls.length > 0 ? toolCalls : undefined,
          },
        ];
      }
      streaming = false;
      streamContent = "";
      activeToolCalls = [];
    },

    clear() {
      messages = [];
      streaming = false;
      streamContent = "";
      error = "";
      activeToolCalls = [];
    },
  };
}

export const messagesStore = createMessagesStore();
