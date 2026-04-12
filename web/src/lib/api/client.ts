import { isTauri } from "$lib/tauri";

const DEFAULT_BASE_URL = "http://127.0.0.1:18981";
const TOKEN_KEY = "zenii_token";
const BASE_URL_KEY = "zenii_base_url";

/** Race a promise against a timeout. Rejects with an Error if the deadline is exceeded. */
export function withTimeout<T>(
  promise: Promise<T>,
  ms: number,
  label?: string,
): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout>;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(
      () =>
        reject(
          new Error(
            `Request timed out after ${ms}ms${label ? `: ${label}` : ""}`,
          ),
        ),
      ms,
    );
  });
  return Promise.race([promise, timeout]).finally(() =>
    clearTimeout(timeoutId),
  );
}

/**
 * Resolved fetch: Tauri plugin on desktop (bypasses WebView CORS/mixed-content
 * restrictions on both Windows WebView2 and macOS WKWebView), browser native otherwise.
 */
async function resolvedFetch(
  input: string | URL | Request,
  init?: RequestInit,
  timeoutMs = 15_000,
): Promise<Response> {
  if (isTauri) {
    const { fetch: tauriFetch } = await import("@tauri-apps/plugin-http");
    return withTimeout(tauriFetch(input, init), timeoutMs, String(input));
  }
  return withTimeout(fetch(input, init), timeoutMs, String(input));
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
  hint?: string;
}

export class ZeniiApiError extends Error {
  constructor(
    public status: number,
    public errorCode: string,
    public details?: string,
    public hint?: string,
  ) {
    super(`${errorCode}: ${details ?? "Unknown error"}`);
    this.name = "ZeniiApiError";
  }
}

export async function api<T>(
  path: string,
  options: RequestInit & { timeout?: number } = {},
): Promise<T> {
  const { timeout, ...fetchOptions } = options;
  const token = getToken();
  const baseUrl = getBaseUrl();
  const url = `${baseUrl}${path}`;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(fetchOptions.headers as Record<string, string>),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  let response: Response;
  try {
    response = await resolvedFetch(url, { ...fetchOptions, headers }, timeout);
  } catch (e) {
    console.error(`[API] ${options.method ?? "GET"} ${url} fetch error:`, e);
    throw e;
  }

  if (!response.ok) {
    let errorCode = "ZENII_UNKNOWN";
    let details = response.statusText;
    let hint: string | undefined;
    try {
      const body = await response.json();
      errorCode = body.error_code ?? "ZENII_ERROR";
      details = body.message ?? body.error ?? response.statusText;
      hint = body.hint;
    } catch {
      // response wasn't JSON
    }
    console.warn(
      `[API] ${options.method ?? "GET"} ${url} -> ${response.status} ${errorCode}: ${details}`,
    );
    throw new ZeniiApiError(response.status, errorCode, details, hint);
  }

  // Handle responses with no body — read text first, only parse if non-empty.
  // Covers 204 No Content, 201 Created with no body, 200 OK with no body, etc.
  const text = await response.text();
  if (!text) {
    return undefined as T;
  }

  return JSON.parse(text) as T;
}

export async function apiGet<T>(path: string): Promise<T> {
  return api<T>(path, { method: "GET" });
}

export async function apiPost<T>(
  path: string,
  body?: unknown,
  opts?: { timeout?: number },
): Promise<T> {
  return api<T>(path, {
    method: "POST",
    body: body ? JSON.stringify(body) : undefined,
    ...opts,
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

/** GET request returning plain text (uses Tauri HTTP plugin on desktop). */
export async function apiGetText(path: string): Promise<string> {
  const token = getToken();
  const baseUrl = getBaseUrl();
  const url = `${baseUrl}${path}`;
  const headers: Record<string, string> = {};
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  const response = await resolvedFetch(url, { headers });
  if (!response.ok) {
    throw new ZeniiApiError(
      response.status,
      "ZENII_UNKNOWN",
      response.statusText,
    );
  }
  return response.text();
}

export async function healthCheck(): Promise<boolean> {
  try {
    const data = await apiGet<{ status: string }>("/health");
    return data.status === "ok";
  } catch {
    return false;
  }
}
