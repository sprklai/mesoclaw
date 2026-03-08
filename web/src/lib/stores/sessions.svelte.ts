import { apiGet, apiPost, apiPut, apiDelete } from "$lib/api/client";

export interface Session {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
}

export interface SessionSummary {
  id: string;
  title: string;
  created_at: number;
}

function createSessionsStore() {
  let sessions = $state<SessionSummary[]>([]);
  let active = $state<Session | null>(null);
  let loading = $state(false);

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

    async load() {
      loading = true;
      try {
        sessions = await apiGet<SessionSummary[]>("/sessions");
      } finally {
        loading = false;
      }
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
      sessions = [
        {
          id: session.id,
          title: session.title,
          created_at: session.created_at,
        },
        ...sessions,
      ];
      active = session;
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
  };
}

export const sessionsStore = createSessionsStore();
