import { describe, it, expect, vi, beforeEach } from "vitest";
import { createChatStream, type ChatStreamCallbacks } from "./websocket";

// Mock localStorage for getBaseUrl/getToken
const store: Record<string, string> = {};
Object.defineProperty(globalThis, "localStorage", {
  value: {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      for (const key in store) delete store[key];
    },
  },
  writable: true,
});

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  url: string;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: ((event: { wasClean: boolean; code: number }) => void) | null = null;
  sentMessages: string[] = [];
  closed = false;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sentMessages.push(data);
  }

  close() {
    this.closed = true;
  }
}

vi.stubGlobal("WebSocket", MockWebSocket);

describe("WebSocket chat streaming", () => {
  let callbacks: ChatStreamCallbacks;

  beforeEach(() => {
    for (const key in store) delete store[key];
    MockWebSocket.instances = [];
    callbacks = {
      onToken: vi.fn(),
      onDone: vi.fn(),
      onError: vi.fn(),
    };
  });

  // 6.3: WebSocket manager connects and parses messages
  it("connects to WS URL with token", () => {
    store["mesoclaw_token"] = "ws-token";
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    expect(ws.url).toContain("/ws/chat?token=ws-token");
  });

  it("sends prompt on open", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    ws.onopen!();

    expect(ws.sentMessages).toHaveLength(1);
    const sent = JSON.parse(ws.sentMessages[0]);
    expect(sent.prompt).toBe("hello");
    expect(sent.session_id).toBe("sess-1");
  });

  it("calls onToken for text messages", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({ data: JSON.stringify({ type: "text", content: "Hi" }) });

    expect(callbacks.onToken).toHaveBeenCalledWith("Hi");
  });

  it("calls onDone and closes on done message", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({ data: JSON.stringify({ type: "done" }) });

    expect(callbacks.onDone).toHaveBeenCalled();
    expect(ws.closed).toBe(true);
  });

  it("calls onError and closes on error message", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({ data: JSON.stringify({ type: "error", error: "fail" }) });

    expect(callbacks.onError).toHaveBeenCalledWith("fail");
    expect(ws.closed).toBe(true);
  });

  it("calls onError on invalid JSON", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({ data: "not json" });

    expect(callbacks.onError).toHaveBeenCalledWith(
      "Failed to parse WebSocket message",
    );
  });

  it("connects without token in URL when none set", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    expect(ws.url).not.toContain("token=");
    expect(ws.url).toContain("/ws/chat");
  });

  // TV.23 — onToolCall callback fires for tool_call message
  it("calls onToolCall for tool_call messages", () => {
    const onToolCall = vi.fn();
    createChatStream("hello", "sess-1", { ...callbacks, onToolCall });

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({
      data: JSON.stringify({
        type: "tool_call",
        call_id: "tc-1",
        tool_name: "WebSearch",
        args: { query: "rust" },
      }),
    });

    expect(onToolCall).toHaveBeenCalledWith("tc-1", "WebSearch", {
      query: "rust",
    });
  });

  // TV.24 — onToolResult callback fires for tool_result message
  it("calls onToolResult for tool_result messages", () => {
    const onToolResult = vi.fn();
    createChatStream("hello", "sess-1", { ...callbacks, onToolResult });

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({
      data: JSON.stringify({
        type: "tool_result",
        call_id: "tc-1",
        tool_name: "WebSearch",
        output: "found results",
        success: true,
        duration_ms: 150,
      }),
    });

    expect(onToolResult).toHaveBeenCalledWith(
      "tc-1",
      "WebSearch",
      "found results",
      true,
      150,
    );
  });

  // TV.25 — Unknown message types are ignored gracefully
  it("ignores unknown message types gracefully", () => {
    createChatStream("hello", "sess-1", callbacks);

    const ws = MockWebSocket.instances[0];
    ws.onmessage!({
      data: JSON.stringify({ type: "unknown_type", data: "something" }),
    });

    expect(callbacks.onToken).not.toHaveBeenCalled();
    expect(callbacks.onDone).not.toHaveBeenCalled();
    expect(callbacks.onError).not.toHaveBeenCalled();
  });
});
