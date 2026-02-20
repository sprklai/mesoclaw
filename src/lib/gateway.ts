/**
 * Gateway REST client for agent and memory operations.
 *
 * Thin wrapper around the daemon REST API.  Resolves the daemon port and
 * bearer token via Tauri IPC (`get_daemon_config_command`) on every call so
 * the values are always fresh.  The Tauri IPC call is the only
 * desktop-specific coupling; all data I/O goes through the gateway REST API,
 * allowing the CLI and GUI to share the same data path.
 */

import { resolveDaemonConfig } from "@/lib/gateway-client";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface GatewayMemoryEntry {
  id: string;
  key: string;
  content: string;
  category: string;
  score: number;
  created_at: string;
  updated_at: string;
}

export interface GatewaySessionsResponse {
  sessions: string[];
  count: number;
}

export interface GatewaySessionResponse {
  session_id: string;
  status: string;
}

export interface GatewayMemoryListResponse {
  entries: GatewayMemoryEntry[];
  count: number;
}

export interface GatewayMemorySearchResponse {
  entries: GatewayMemoryEntry[];
  count: number;
}

export interface GatewayStoreMemoryResponse {
  key: string;
  status: string;
}

export interface GatewayForgetMemoryResponse {
  key: string;
  status: string;
}

export interface GatewayCreateSessionPayload {
  system_prompt?: string;
  user_message?: string;
  provider_id?: string;
  channel?: string;
  context?: string;
}

// ─── Internal fetch helper ────────────────────────────────────────────────────

async function daemonFetch(
  path: string,
  options: RequestInit = {},
): Promise<Response> {
  const config = await resolveDaemonConfig();
  const port = config?.port ?? 18790;
  const token = config?.token ?? "";

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...((options.headers as Record<string, string>) ?? {}),
  };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  return fetch(`http://127.0.0.1:${port}/api/v1/${path}`, {
    ...options,
    headers,
  });
}

async function daemonGet<T>(path: string): Promise<T> {
  const res = await daemonFetch(path);
  if (!res.ok) {
    throw new Error(`GET /${path} failed: ${res.status} ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

async function daemonPost<T>(path: string, body: unknown): Promise<T> {
  const res = await daemonFetch(path, {
    method: "POST",
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`POST /${path} failed: ${res.status} ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

async function daemonDelete<T>(path: string): Promise<T> {
  const res = await daemonFetch(path, { method: "DELETE" });
  if (!res.ok) {
    throw new Error(`DELETE /${path} failed: ${res.status} ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

// ─── gateway namespace ────────────────────────────────────────────────────────

/**
 * Typed REST helpers for the MesoClaw gateway daemon.
 *
 * All methods resolve the daemon connection on each call (cheap — reads two
 * small files from disk via Tauri IPC).
 *
 * Usage:
 *   import { gateway } from "@/lib/gateway";
 *   const { entries } = await gateway.searchMemory("project goal");
 */
export const gateway = {
  // ── Agent sessions ────────────────────────────────────────────────────────

  listSessions(): Promise<GatewaySessionsResponse> {
    return daemonGet<GatewaySessionsResponse>("sessions");
  },

  createSession(payload: GatewayCreateSessionPayload): Promise<GatewaySessionResponse> {
    return daemonPost<GatewaySessionResponse>("sessions", payload);
  },

  // ── Memory ────────────────────────────────────────────────────────────────

  listMemory(): Promise<GatewayMemoryListResponse> {
    return daemonGet<GatewayMemoryListResponse>("memory");
  },

  storeMemory(
    key: string,
    content: string,
    category = "core",
  ): Promise<GatewayStoreMemoryResponse> {
    return daemonPost<GatewayStoreMemoryResponse>("memory", {
      key,
      content,
      category,
    });
  },

  async searchMemory(
    query: string,
    limit = 20,
  ): Promise<GatewayMemorySearchResponse> {
    const params = new URLSearchParams({ q: query, limit: String(limit) });
    return daemonGet<GatewayMemorySearchResponse>(
      `memory/search?${params.toString()}`,
    );
  },

  forgetMemory(key: string): Promise<GatewayForgetMemoryResponse> {
    return daemonDelete<GatewayForgetMemoryResponse>(
      `memory/${encodeURIComponent(key)}`,
    );
  },

  // ── WebSocket URL helper ───────────────────────────────────────────────────

  async wsUrl(): Promise<string> {
    const config = await resolveDaemonConfig();
    const port = config?.port ?? 18790;
    const token = config?.token ?? "";
    return `ws://127.0.0.1:${port}/api/v1/ws${token ? `?token=${encodeURIComponent(token)}` : ""}`;
  },
};
