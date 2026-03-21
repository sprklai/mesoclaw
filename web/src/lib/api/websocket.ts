import { isTauri } from "$lib/tauri";
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

export interface WsDelegationStartedMessage {
  type: "delegation_started";
  delegation_id: string;
  agent_count: number;
  agents: Array<{ id: string; description: string }>;
}

export interface WsAgentProgressMessage {
  type: "agent_progress";
  delegation_id: string;
  agent_id: string;
  tool_uses: number;
  tokens_used: number;
  current_activity: string;
}

export interface WsAgentCompletedMessage {
  type: "agent_completed";
  delegation_id: string;
  agent_id: string;
  status: string;
  duration_ms: number;
  tool_uses: number;
  tokens_used: number;
  error?: string;
}

export interface WsDelegationCompletedMessage {
  type: "delegation_completed";
  delegation_id: string;
  total_duration_ms: number;
  total_tokens: number;
}

export interface WsApprovalRequestMessage {
  type: "approval_request";
  approval_id: string;
  call_id: string;
  tool_name: string;
  args_summary: string;
  risk_level: string;
  reason: string;
  timeout_secs: number;
}

export interface WsApprovalResolvedMessage {
  type: "approval_resolved";
  approval_id: string;
  decision: string;
  auto: boolean;
}

export interface WsWarningMessage {
  type: "warning";
  warning: string;
}

export type WsMessage =
  | WsTextMessage
  | WsDoneMessage
  | WsErrorMessage
  | WsToolCallMessage
  | WsToolResultMessage
  | WsDelegationStartedMessage
  | WsAgentProgressMessage
  | WsAgentCompletedMessage
  | WsDelegationCompletedMessage
  | WsApprovalRequestMessage
  | WsApprovalResolvedMessage
  | WsWarningMessage;

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
  onDelegationStarted?: (
    delegationId: string,
    agents: Array<{ id: string; description: string }>,
  ) => void;
  onAgentProgress?: (
    delegationId: string,
    agentId: string,
    toolUses: number,
    tokensUsed: number,
    activity: string,
  ) => void;
  onAgentCompleted?: (
    delegationId: string,
    agentId: string,
    status: string,
    durationMs: number,
    toolUses: number,
    tokensUsed: number,
    error?: string,
  ) => void;
  onDelegationCompleted?: (
    delegationId: string,
    totalDurationMs: number,
    totalTokens: number,
  ) => void;
  onApprovalRequest?: (
    approvalId: string,
    callId: string,
    toolName: string,
    argsSummary: string,
    riskLevel: string,
    reason: string,
    timeoutSecs: number,
  ) => void;
  onApprovalResolved?: (approvalId: string, decision: string) => void;
  onWarning?: (message: string) => void;
  onDone: () => void;
  onError: (error: string) => void;
}

/** Minimal interface for an active chat connection (browser WS or Tauri WS). */
export interface ChatConnection {
  send(data: string): void;
  close(): void;
  readonly isOpen: boolean;
}

/** Dispatch a parsed WsMessage to the appropriate callback. Returns true if the connection should close. */
function dispatchMessage(
  msg: WsMessage,
  callbacks: ChatStreamCallbacks,
): boolean {
  switch (msg.type) {
    case "text":
      callbacks.onToken(msg.content);
      return false;
    case "tool_call":
      callbacks.onToolCall?.(msg.call_id, msg.tool_name, msg.args);
      return false;
    case "tool_result":
      callbacks.onToolResult?.(
        msg.call_id,
        msg.tool_name,
        msg.output,
        msg.success,
        msg.duration_ms,
      );
      return false;
    case "delegation_started":
      callbacks.onDelegationStarted?.(msg.delegation_id, msg.agents);
      return false;
    case "agent_progress":
      callbacks.onAgentProgress?.(
        msg.delegation_id,
        msg.agent_id,
        msg.tool_uses,
        msg.tokens_used,
        msg.current_activity,
      );
      return false;
    case "agent_completed":
      callbacks.onAgentCompleted?.(
        msg.delegation_id,
        msg.agent_id,
        msg.status,
        msg.duration_ms,
        msg.tool_uses,
        msg.tokens_used,
        msg.error,
      );
      return false;
    case "delegation_completed":
      callbacks.onDelegationCompleted?.(
        msg.delegation_id,
        msg.total_duration_ms,
        msg.total_tokens,
      );
      return false;
    case "approval_request":
      callbacks.onApprovalRequest?.(
        msg.approval_id,
        msg.call_id,
        msg.tool_name,
        msg.args_summary,
        msg.risk_level,
        msg.reason,
        msg.timeout_secs,
      );
      return false;
    case "approval_resolved":
      callbacks.onApprovalResolved?.(msg.approval_id, msg.decision);
      return false;
    case "warning":
      callbacks.onWarning?.(msg.warning);
      return false;
    case "done":
      callbacks.onDone();
      return true;
    case "error":
      callbacks.onError(msg.error);
      return true;
    default:
      console.warn(
        `[WS] Unknown message type: ${(msg as { type: string }).type}`,
        msg,
      );
      return false;
  }
}

function buildChatUrl(): string {
  const baseUrl = getBaseUrl().replace(/^http/, "ws");
  const token = getToken();
  return token
    ? `${baseUrl}/ws/chat?token=${encodeURIComponent(token)}`
    : `${baseUrl}/ws/chat`;
}

function buildPromptPayload(
  prompt: string,
  sessionId: string | undefined,
  model?: string,
  delegation?: boolean,
): string {
  return JSON.stringify({
    prompt,
    session_id: sessionId,
    model: model || undefined,
    delegation: delegation || undefined,
  });
}

/** Browser WebSocket path. */
function createChatStreamBrowser(
  prompt: string,
  sessionId: string | undefined,
  callbacks: ChatStreamCallbacks,
  model?: string,
  delegation?: boolean,
): ChatConnection {
  const url = buildChatUrl();
  console.log(`[WS] Connecting to ${url.replace(/token=[^&]+/, "token=***")}`);
  const ws = new WebSocket(url);
  let intentionalClose = false;

  ws.onopen = () => {
    console.log(`[WS] Connected, sending prompt`);
    ws.send(buildPromptPayload(prompt, sessionId, model, delegation));
  };

  ws.onmessage = (event) => {
    try {
      const msg: WsMessage = JSON.parse(event.data);
      const shouldClose = dispatchMessage(msg, callbacks);
      if (shouldClose) {
        intentionalClose = true;
        ws.close();
      }
    } catch {
      callbacks.onError("Failed to parse WebSocket message");
      intentionalClose = true;
      ws.close();
    }
  };

  ws.onerror = (e) => {
    console.error(`[WS] Error:`, e);
    if (!intentionalClose) {
      callbacks.onError("WebSocket connection error");
    }
  };

  ws.onclose = (event) => {
    console.log(
      `[WS] Closed: code=${event.code} reason=${event.reason} clean=${event.wasClean}`,
    );
    if (!intentionalClose && !event.wasClean && event.code !== 1000) {
      callbacks.onError(`Connection closed unexpectedly (code: ${event.code})`);
    }
  };

  return {
    send: (data: string) => ws.send(data),
    close: () => {
      intentionalClose = true;
      ws.close();
    },
    get isOpen() {
      return ws.readyState === WebSocket.OPEN;
    },
  };
}

/** Tauri WebSocket plugin path — bypasses WebView2 mixed-content blocking. */
async function createChatStreamTauri(
  prompt: string,
  sessionId: string | undefined,
  callbacks: ChatStreamCallbacks,
  model?: string,
  delegation?: boolean,
): Promise<ChatConnection> {
  const { default: TauriWebSocket } =
    await import("@tauri-apps/plugin-websocket");
  const url = buildChatUrl();
  console.log(
    `[WS/Tauri] Connecting to ${url.replace(/token=[^&]+/, "token=***")}`,
  );

  const ws = await TauriWebSocket.connect(url);
  let intentionalClose = false;
  let open = true;

  ws.addListener((msg) => {
    if (msg.type === "Text" && typeof msg.data === "string") {
      try {
        const parsed: WsMessage = JSON.parse(msg.data);
        const shouldClose = dispatchMessage(parsed, callbacks);
        if (shouldClose) {
          intentionalClose = true;
          open = false;
          ws.disconnect();
        }
      } catch {
        callbacks.onError("Failed to parse WebSocket message");
        intentionalClose = true;
        open = false;
        ws.disconnect();
      }
    } else if (msg.type === "Close") {
      open = false;
      if (!intentionalClose) {
        callbacks.onError("Connection closed unexpectedly");
      }
    }
  });

  // Connection is already open after connect() resolves — send prompt immediately
  console.log(`[WS/Tauri] Connected, sending prompt`);
  await ws.send(buildPromptPayload(prompt, sessionId, model, delegation));

  return {
    send: (data: string) => {
      ws.send(data);
    },
    close: () => {
      intentionalClose = true;
      open = false;
      ws.disconnect();
    },
    get isOpen() {
      return open;
    },
  };
}

/**
 * Create a chat stream connection. Uses Tauri WebSocket plugin on desktop
 * (bypasses WebView2 mixed-content) or browser WebSocket otherwise.
 */
export async function createChatStream(
  prompt: string,
  sessionId: string | undefined,
  callbacks: ChatStreamCallbacks,
  model?: string,
  delegation?: boolean,
): Promise<ChatConnection> {
  if (isTauri) {
    return createChatStreamTauri(
      prompt,
      sessionId,
      callbacks,
      model,
      delegation,
    );
  }
  return createChatStreamBrowser(
    prompt,
    sessionId,
    callbacks,
    model,
    delegation,
  );
}

/** Send an approval response through an active chat connection. */
export function sendApprovalResponse(
  conn: ChatConnection,
  approvalId: string,
  decision: "approve" | "approve_always" | "deny",
): void {
  if (conn.isOpen) {
    conn.send(
      JSON.stringify({
        type: "approval_response",
        approval_id: approvalId,
        decision,
      }),
    );
  }
}
