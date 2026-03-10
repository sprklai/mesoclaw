import { apiGet } from "$lib/api/client";

export interface ChannelConversation {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  source: string;
  channel_key?: string;
}

export interface ChannelMessageItem {
  id: string;
  session_id: string;
  role: string;
  content: string;
  created_at: string;
}

export interface ChannelRealtimeEvent {
  channel: string;
  sender: string;
  session_id: string;
  content_preview: string;
  role: string;
}

const LAST_READ_KEY = "mesoclaw_inbox_last_read";

function getLastReadTimestamps(): Record<string, number> {
  try {
    const stored = localStorage.getItem(LAST_READ_KEY);
    return stored ? JSON.parse(stored) : {};
  } catch {
    return {};
  }
}

function saveLastReadTimestamps(timestamps: Record<string, number>) {
  localStorage.setItem(LAST_READ_KEY, JSON.stringify(timestamps));
}

class InboxStore {
  conversations = $state<ChannelConversation[]>([]);
  selectedId = $state<string | null>(null);
  messages = $state<ChannelMessageItem[]>([]);
  channelFilter = $state<string | null>(null);
  loading = $state(false);
  loadingMessages = $state(false);
  lastReadTimestamps = $state<Record<string, number>>(getLastReadTimestamps());

  get totalUnread(): number {
    let count = 0;
    for (const conv of this.conversations) {
      const lastRead = this.lastReadTimestamps[conv.id] ?? 0;
      const updatedAt = new Date(conv.updated_at).getTime();
      if (updatedAt > lastRead && conv.message_count > 0) {
        count++;
      }
    }
    return count;
  }

  async load() {
    this.loading = true;
    try {
      const params = new URLSearchParams();
      if (this.channelFilter) {
        params.set("source", this.channelFilter);
      }
      params.set("limit", "50");
      const query = params.toString();
      this.conversations = await apiGet<ChannelConversation[]>(
        `/channels/sessions?${query}`,
      );
    } catch {
      this.conversations = [];
    } finally {
      this.loading = false;
    }
  }

  async selectConversation(id: string) {
    this.selectedId = id;
    this.loadingMessages = true;
    try {
      this.messages = await apiGet<ChannelMessageItem[]>(
        `/channels/sessions/${encodeURIComponent(id)}/messages?limit=50`,
      );
    } catch {
      this.messages = [];
    } finally {
      this.loadingMessages = false;
    }
    this.markAsRead(id);
  }

  async loadMoreMessages() {
    if (!this.selectedId || this.messages.length === 0) return;
    const oldestId = this.messages[0]?.id;
    if (!oldestId) return;

    try {
      const older = await apiGet<ChannelMessageItem[]>(
        `/channels/sessions/${encodeURIComponent(this.selectedId)}/messages?limit=50&before=${encodeURIComponent(oldestId)}`,
      );
      if (older.length > 0) {
        this.messages = [...older, ...this.messages];
      }
    } catch {
      // Ignore pagination errors
    }
  }

  handleRealtimeMessage(event: ChannelRealtimeEvent) {
    // Update conversation list
    const existing = this.conversations.find((c) => c.id === event.session_id);
    if (existing) {
      existing.updated_at = new Date().toISOString();
      existing.message_count++;
      // Move to top
      this.conversations = [
        existing,
        ...this.conversations.filter((c) => c.id !== event.session_id),
      ];
    } else {
      // Reload to pick up new conversation
      this.load();
    }

    // If viewing this conversation, append the message preview
    if (this.selectedId === event.session_id) {
      this.messages = [
        ...this.messages,
        {
          id: `rt-${Date.now()}`,
          session_id: event.session_id,
          role: event.role,
          content: event.content_preview,
          created_at: new Date().toISOString(),
        },
      ];
      this.markAsRead(event.session_id);
    }
  }

  markAsRead(sessionId: string) {
    this.lastReadTimestamps = {
      ...this.lastReadTimestamps,
      [sessionId]: Date.now(),
    };
    saveLastReadTimestamps(this.lastReadTimestamps);
  }

  setFilter(source: string | null) {
    this.channelFilter = source;
    this.load();
  }
}

export const inboxStore = new InboxStore();
