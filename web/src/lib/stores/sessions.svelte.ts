import { apiGet, apiPost, apiPut, apiDelete } from "$lib/api/client";

export interface Session {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  source?: string;
}

export interface SessionSummary {
  id: string;
  title: string;
  created_at: number;
  source?: string;
}

function createSessionsStore() {
  let sessions = $state<SessionSummary[]>([]);
  let active = $state<Session | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  /** IDs of sessions being created locally — used to suppress duplicate push events. */
  const pendingLocalIds = new Set<string>();

  return {
    get sessions() {
      return sessions;
    },
    get active() {
      return active;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },

    async load() {
      loading = true;
      error = null;
      const maxAttempts = 3;
      for (let attempt = 0; attempt < maxAttempts; attempt++) {
        try {
          sessions = await apiGet<SessionSummary[]>("/sessions");
          error = null;
          break;
        } catch (e) {
          if (attempt < maxAttempts - 1) {
            // Exponential backoff: 1s, 2s, 4s
            await new Promise((r) =>
              setTimeout(r, 1000 * Math.pow(2, attempt)),
            );
          } else {
            const msg = e instanceof Error ? e.message : String(e);
            error = `Failed to load sessions. Is the daemon running? (${msg})`;
            console.error("sessionsStore.load failed after retries:", e);
          }
        }
      }
      loading = false;
    },

    async get(id: string) {
      active = await apiGet<Session>(`/sessions/${encodeURIComponent(id)}`);
      sessions = sessions.map((s) =>
        s.id === id ? { ...s, title: active!.title } : s,
      );
      return active;
    },

    async create(title: string) {
      const session = await apiPost<Session>("/sessions", { title });
      pendingLocalIds.add(session.id);
      sessions = [
        {
          id: session.id,
          title: session.title,
          created_at: session.created_at,
        },
        ...sessions,
      ];
      active = session;
      // Allow push event dedup window to pass, then clear
      setTimeout(() => pendingLocalIds.delete(session.id), 2000);
      return session;
    },

    async update(id: string, title: string) {
      const session = await apiPut<Session>(
        `/sessions/${encodeURIComponent(id)}`,
        { title },
      );
      sessions = sessions.map((s) => (s.id === id ? { ...s, title } : s));
      if (active?.id === id) active = session;
      return session;
    },

    async remove(id: string) {
      await apiDelete(`/sessions/${encodeURIComponent(id)}`);
      sessions = sessions.filter((s) => s.id !== id);
      if (active?.id === id) active = null;
    },

    async generateTitle(id: string, model?: string) {
      try {
        const session = await apiPost<Session>(
          `/sessions/${encodeURIComponent(id)}/generate-title`,
          model ? { model } : {},
        );
        sessions = sessions.map((s) =>
          s.id === id ? { ...s, title: session.title } : s,
        );
        if (active?.id === id) active = session;
      } catch (e) {
        console.warn("generateTitle failed:", e);
      }
    },

    setActive(session: Session | null) {
      active = session;
    },

    /** Prepend a session from a push notification (guards against duplicates). */
    prependFromEvent(data: { id: string; title: string; source?: string }) {
      if (pendingLocalIds.has(data.id)) return;
      if (sessions.some((s) => s.id === data.id)) return;
      sessions = [
        {
          id: data.id,
          title: data.title,
          created_at: Date.now(),
          source: data.source,
        },
        ...sessions,
      ];
    },

    /** Remove a session from a push notification. */
    removeFromEvent(sessionId: string) {
      sessions = sessions.filter((s) => s.id !== sessionId);
      if (active?.id === sessionId) active = null;
    },

    /** Move a session to the top of the list (most recent activity). */
    bumpSession(sessionId: string) {
      const idx = sessions.findIndex((s) => s.id === sessionId);
      if (idx > 0) {
        const session = sessions[idx];
        sessions = [session, ...sessions.filter((s) => s.id !== sessionId)];
      }
    },
  };
}

export const sessionsStore = createSessionsStore();
