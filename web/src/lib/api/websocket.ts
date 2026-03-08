import { getBaseUrl, getToken } from "./client";

export interface WsTextMessage {
  type: "text";
  content: string;
}

export interface WsDoneMessage {
  type: "done";
}

export interface WsErrorMessage {
  type: "error";
  error: string;
}

export interface WsToolCallMessage {
  type: "tool_call";
  call_id: string;
  tool_name: string;
  args: unknown;
}

export interface WsToolResultMessage {
  type: "tool_result";
  call_id: string;
  tool_name: string;
  output: string;
  success: boolean;
  duration_ms: number;
}

export type WsMessage =
  | WsTextMessage
  | WsDoneMessage
  | WsErrorMessage
  | WsToolCallMessage
  | WsToolResultMessage;

export interface ChatStreamCallbacks {
  onToken: (content: string) => void;
  onToolCall?: (callId: string, toolName: string, args: unknown) => void;
  onToolResult?: (
    callId: string,
    toolName: string,
    output: string,
    success: boolean,
    durationMs: number,
  ) => void;
  onDone: () => void;
  onError: (error: string) => void;
}

export function createChatStream(
  prompt: string,
  sessionId: string | undefined,
  callbacks: ChatStreamCallbacks,
  model?: string,
): WebSocket {
  const baseUrl = getBaseUrl().replace(/^http/, "ws");
  const token = getToken();
  const url = token
    ? `${baseUrl}/ws/chat?token=${encodeURIComponent(token)}`
    : `${baseUrl}/ws/chat`;

  const ws = new WebSocket(url);
  let intentionalClose = false;

  ws.onopen = () => {
    ws.send(
      JSON.stringify({
        prompt,
        session_id: sessionId,
        model: model || undefined,
      }),
    );
  };

  ws.onmessage = (event) => {
    try {
      const msg: WsMessage = JSON.parse(event.data);
      switch (msg.type) {
        case "text":
          callbacks.onToken(msg.content);
          break;
        case "tool_call":
          callbacks.onToolCall?.(msg.call_id, msg.tool_name, msg.args);
          break;
        case "tool_result":
          callbacks.onToolResult?.(
            msg.call_id,
            msg.tool_name,
            msg.output,
            msg.success,
            msg.duration_ms,
          );
          break;
        case "done":
          callbacks.onDone();
          intentionalClose = true;
          ws.close();
          break;
        case "error":
          callbacks.onError(msg.error);
          intentionalClose = true;
          ws.close();
          break;
      }
    } catch {
      callbacks.onError("Failed to parse WebSocket message");
      intentionalClose = true;
      ws.close();
    }
  };

  ws.onerror = () => {
    if (!intentionalClose) {
      callbacks.onError("WebSocket connection error");
    }
  };

  ws.onclose = (event) => {
    if (!intentionalClose && !event.wasClean && event.code !== 1000) {
      callbacks.onError(`Connection closed unexpectedly (code: ${event.code})`);
    }
  };

  return ws;
}
