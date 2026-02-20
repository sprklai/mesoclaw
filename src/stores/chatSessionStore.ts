import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface ChatSession {
  id: string;
  sessionKey: string;
  agent: string;
  scope: string;
  channel: string;
  peer: string;
  createdAt: string;
  updatedAt: string;
  compactionSummary?: string;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  createdAt: string;
}

interface ChatSessionState {
  sessions: ChatSession[];
  activeSessionId: string | null;
  messages: Map<string, ChatMessage[]>;
  isLoading: boolean;
  error: string | null;

  loadSessions: () => Promise<void>;
  createSession: (providerId: string, modelId: string) => Promise<string>;
  loadSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  loadMessages: (sessionId: string) => Promise<void>;
  saveMessage: (role: "user" | "assistant" | "system", content: string) => Promise<void>;
  clearMessages: () => Promise<void>;

  // Helper methods for commands
  getCurrentSession: () => ChatSession | null;
  getMessages: () => ChatMessage[];
}

export type UseChatSessionStore = ReturnType<typeof useChatSessionStore>;

export const useChatSessionStore = create<ChatSessionState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  messages: new Map(),
  isLoading: false,
  error: null,

  loadSessions: async () => {
    set({ isLoading: true, error: null });
    try {
      const sessions = await invoke<ChatSession[]>("list_chat_sessions_command");
      set({ sessions, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  createSession: async (providerId: string, modelId: string) => {
    try {
      const session = await invoke<ChatSession>("create_chat_session_command", {
        request: { providerId, modelId },
      });
      set((state) => ({
        sessions: [session, ...state.sessions],
        activeSessionId: session.id,
        messages: new Map(state.messages).set(session.id, []),
      }));
      return session.id;
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  loadSession: async (sessionId: string) => {
    set({ activeSessionId: sessionId });
    await get().loadMessages(sessionId);
  },

  deleteSession: async (sessionId: string) => {
    try {
      await invoke("delete_chat_session_command", { id: sessionId });
      set((state) => {
        const newMessages = new Map(state.messages);
        newMessages.delete(sessionId);
        return {
          sessions: state.sessions.filter((s) => s.id !== sessionId),
          activeSessionId:
            state.activeSessionId === sessionId ? null : state.activeSessionId,
          messages: newMessages,
        };
      });
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  loadMessages: async (sessionId: string) => {
    try {
      const messages = await invoke<ChatMessage[]>("load_messages_command", {
        sessionId,
      });
      set((state) => {
        const newMessages = new Map(state.messages);
        newMessages.set(sessionId, messages);
        return { messages: newMessages };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  saveMessage: async (role: "user" | "assistant" | "system", content: string) => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    try {
      const message = await invoke<ChatMessage>("save_message_command", {
        request: { sessionId: activeSessionId, role, content },
      });
      set((state) => {
        const newMessages = new Map(state.messages);
        const existing = newMessages.get(activeSessionId) || [];
        newMessages.set(activeSessionId, [...existing, message]);
        return { messages: newMessages };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  clearMessages: async () => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    try {
      await invoke("clear_session_messages_command", { sessionId: activeSessionId });
      set((state) => {
        const newMessages = new Map(state.messages);
        newMessages.set(activeSessionId, []);
        return { messages: newMessages };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  getCurrentSession: () => {
    const { sessions, activeSessionId } = get();
    if (!activeSessionId) return null;
    return sessions.find((s) => s.id === activeSessionId) || null;
  },

  getMessages: () => {
    const { activeSessionId, messages } = get();
    if (!activeSessionId) return [];
    return messages.get(activeSessionId) || [];
  },
}));
