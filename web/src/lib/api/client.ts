import { isTauri } from "$lib/tauri";

const DEFAULT_BASE_URL = "http://127.0.0.1:18981";
const TOKEN_KEY = "zenii_token";
const BASE_URL_KEY = "zenii_base_url";

/**
 * Resolved fetch: Tauri plugin on desktop (bypasses WebView2 mixed-content),
 * browser native otherwise.
 */
async function resolvedFetch(
  input: string | URL | Request,
  init?: RequestInit,
): Promise<Response> {
  if (isTauri) {
    const { fetch: tauriFetch } = await import("@tauri-apps/plugin-http");
    return tauriFetch(input, init);
  }
  return fetch(input, init);
}

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

export function isValidBaseUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

export function getBaseUrl(): string {
  if (typeof window === "undefined") return DEFAULT_BASE_URL;
  const stored = localStorage.getItem(BASE_URL_KEY);
  if (stored && isValidBaseUrl(stored)) return stored;
  return DEFAULT_BASE_URL;
}

export function setBaseUrl(url: string): void {
  localStorage.setItem(BASE_URL_KEY, url);
}

export function clearBaseUrl(): void {
  localStorage.removeItem(BASE_URL_KEY);
}

/**
 * Perform a health check without authentication headers.
 * Used to detect if the daemon requires auth at all.
 */
export async function healthCheckNoAuth(): Promise<boolean> {
  const baseUrl = getBaseUrl();
  try {
    const response = await resolvedFetch(`${baseUrl}/health`);
    if (!response.ok) {
      console.warn(
        `[API] healthCheckNoAuth: ${baseUrl}/health returned ${response.status}`,
      );
      return false;
    }
    const data = await response.json();
    return data.status === "ok";
  } catch (e) {
    console.warn(`[API] healthCheckNoAuth: ${baseUrl}/health fetch error:`, e);
    return false;
  }
}

export interface ApiError {
  error_code: string;
  message: string;
  details?: string;
}

export class MesoApiError extends Error {
  constructor(
    public status: number,
    public errorCode: string,
    public details?: string,
  ) {
    super(`${errorCode}: ${details ?? "Unknown error"}`);
    this.name = "MesoApiError";
  }
}

export async function api<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const token = getToken();
  const baseUrl = getBaseUrl();
  const url = `${baseUrl}${path}`;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  let response: Response;
  try {
    response = await resolvedFetch(url, {
      ...options,
      headers,
    });
  } catch (e) {
    console.error(`[API] ${options.method ?? "GET"} ${url} fetch error:`, e);
    throw e;
  }

  if (!response.ok) {
    let errorCode = "ZENII_UNKNOWN";
    let details = response.statusText;
    try {
      const body: ApiError = await response.json();
      errorCode = body.error_code;
      details = body.message;
    } catch {
      // response wasn't JSON
    }
    console.warn(
      `[API] ${options.method ?? "GET"} ${url} -> ${response.status} ${errorCode}: ${details}`,
    );
    throw new MesoApiError(response.status, errorCode, details);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

export async function apiGet<T>(path: string): Promise<T> {
  return api<T>(path, { method: "GET" });
}

export async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  return api<T>(path, {
    method: "POST",
    body: body ? JSON.stringify(body) : undefined,
  });
}

export async function apiPut<T>(path: string, body?: unknown): Promise<T> {
  return api<T>(path, {
    method: "PUT",
    body: body ? JSON.stringify(body) : undefined,
  });
}

export async function apiDelete<T>(path: string): Promise<T> {
  return api<T>(path, { method: "DELETE" });
}

export async function healthCheck(): Promise<boolean> {
  try {
    const data = await apiGet<{ status: string }>("/health");
    return data.status === "ok";
  } catch {
    return false;
  }
}
