/**
 * Gateway client — HTTP + WebSocket interface to the MesoClaw daemon.
 *
 * The daemon runs on localhost (default port 18790) and requires a bearer
 * token stored at ~/.mesoclaw/daemon.token.  The desktop app exposes a Tauri
 * command to read the token and port; in development the values can be
 * provided directly.
 */

import { invoke } from "@tauri-apps/api/core";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface GatewayConfig {
  port: number;
  token: string;
}

export interface SessionResponse {
  sessionId: string;
  status: string;
}

export interface GatewayHealth {
  status: string;
  service: string;
}

/** Events forwarded from the Rust EventBus via the WebSocket connection. */
export interface AppEvent {
  type:
    | "agent_tool_start"
    | "agent_tool_result"
    | "agent_complete"
    | "approval_needed"
    | "approval_response"
    | "heartbeat_tick"
    | "cron_fired"
    | "channel_message"
    | "memory_stored"
    | "memory_recalled"
    | "system_ready"
    | "system_error"
    | "provider_health_change";
  [key: string]: unknown;
}

export type EventHandler = (event: AppEvent) => void;
export type ErrorHandler = (error: Error) => void;

// ─── Config resolution ────────────────────────────────────────────────────────

const DEFAULT_PORT = 18790;

/** Read daemon config from the Tauri backend. Falls back to defaults. */
export async function resolveDaemonConfig(): Promise<GatewayConfig | null> {
  try {
    const config = await invoke<{ port: number; token: string }>(
      "get_daemon_config_command",
    );
    return { port: config.port, token: config.token };
  } catch {
    // Daemon not running yet — caller can retry or use Tauri IPC directly.
    return null;
  }
}

// ─── GatewayClient ───────────────────────────────────────────────────────────

export class GatewayClient {
  private baseUrl: string;
  private wsUrl: string;
  private headers: Record<string, string>;

  constructor(config: GatewayConfig) {
    this.baseUrl = `http://127.0.0.1:${config.port}`;
    this.wsUrl = `ws://127.0.0.1:${config.port}`;
    this.headers = {
      "Content-Type": "application/json",
      Authorization: `Bearer ${config.token}`,
    };
  }

  // ── REST helpers ───────────────────────────────────────────────────────────

  private async get<T>(path: string, auth = true): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      headers: auth ? this.headers : {},
    });
    if (!res.ok) {
      throw new Error(`GET ${path} failed: ${res.status} ${res.statusText}`);
    }
    return res.json() as Promise<T>;
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: this.headers,
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      throw new Error(`POST ${path} failed: ${res.status} ${res.statusText}`);
    }
    return res.json() as Promise<T>;
  }

  // ── Health ─────────────────────────────────────────────────────────────────

  async health(): Promise<GatewayHealth> {
    return this.get<GatewayHealth>("/api/v1/health", false);
  }

  async isReachable(): Promise<boolean> {
    try {
      await this.health();
      return true;
    } catch {
      return false;
    }
  }

  // ── Agent sessions ─────────────────────────────────────────────────────────

  async listSessions(): Promise<{ sessions: SessionResponse[] }> {
    return this.get("/api/v1/sessions");
  }

  async createSession(systemPrompt?: string): Promise<SessionResponse> {
    return this.post("/api/v1/sessions", { system_prompt: systemPrompt });
  }

  // ── Provider status ────────────────────────────────────────────────────────

  async listProviders(): Promise<unknown> {
    return this.get("/api/v1/providers");
  }

  // ── WebSocket event stream ─────────────────────────────────────────────────

  /**
   * Open a WebSocket connection to the daemon event bus.
   *
   * @param onEvent   Called for each incoming event.
   * @param onError   Called on connection errors.
   * @returns A cleanup function that closes the socket.
   */
  connectEventStream(
    onEvent: EventHandler,
    onError?: ErrorHandler,
  ): () => void {
    const url = `${this.wsUrl}/api/v1/ws?token=${encodeURIComponent(this.headers.Authorization?.replace("Bearer ", "") ?? "")}`;
    let ws: WebSocket | null = null;
    let closed = false;

    const connect = () => {
      ws = new WebSocket(url);

      ws.onmessage = (ev) => {
        try {
          const event = JSON.parse(ev.data as string) as AppEvent;
          onEvent(event);
        } catch {
          // Ignore unparseable messages.
        }
      };

      ws.onerror = () => {
        onError?.(new Error("WebSocket connection error"));
      };

      ws.onclose = () => {
        if (!closed) {
          // Reconnect after 2 seconds.
          setTimeout(connect, 2000);
        }
      };
    };

    connect();

    return () => {
      closed = true;
      ws?.close();
    };
  }

  /**
   * Send an approval response to the daemon.
   *
   * This is a convenience wrapper that mirrors the Tauri
   * `approve_action_command` IPC call but goes via the gateway instead.
   */
  async sendApprovalResponse(
    actionId: string,
    approved: boolean,
  ): Promise<void> {
    // Publish the approval via Tauri IPC so the EventBus receives it
    // regardless of whether the gateway is running.  The gateway REST API
    // does not have a separate /approval endpoint — the EventBus is the
    // single source of truth for approval routing.
    await invoke("approve_action_command", { actionId, approved });
  }
}

// ─── Singleton management ─────────────────────────────────────────────────────

let _client: GatewayClient | null = null;

/** Return the shared GatewayClient, creating it with the given config if needed. */
export function getGatewayClient(config?: GatewayConfig): GatewayClient | null {
  if (!_client && config) {
    _client = new GatewayClient(config);
  }
  return _client;
}

/** Replace the shared client (e.g. after daemon restart). */
export function setGatewayClient(client: GatewayClient): void {
  _client = client;
}

/** Create a client for the default config. */
export function createDefaultClient(): GatewayClient {
  const client = new GatewayClient({ port: DEFAULT_PORT, token: "" });
  _client = client;
  return client;
}
