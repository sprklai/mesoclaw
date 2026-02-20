/**
 * Notification store for managing system notifications.
 *
 * Provides state management for:
 * - In-app notification history
 * - Unread count tracking
 * - Notification categories and preferences
 * - Mark as read / clear functionality
 */
import { create } from "zustand";
import { persist } from "zustand/middleware";

export type NotificationCategory =
  | "system"
  | "agent"
  | "channel"
  | "scheduler"
  | "approval";

export type NotificationPriority = "low" | "normal" | "high" | "urgent";

export interface AppNotification {
  id: string;
  title: string;
  message: string;
  category: NotificationCategory;
  priority: NotificationPriority;
  isRead: boolean;
  timestamp: string;
  actionUrl?: string; // Deep link URL (e.g., mesoclaw://session/abc123)
  actionLabel?: string; // Label for action button
  metadata?: Record<string, unknown>; // Additional context
}

interface NotificationState {
  notifications: AppNotification[];
  maxNotifications: number;

  // Actions
  addNotification: (notification: Omit<AppNotification, "id" | "timestamp" | "isRead">) => string;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  removeNotification: (id: string) => void;
  clearAll: () => void;
  clearByCategory: (category: NotificationCategory) => void;

  // Getters
  getUnreadCount: () => number;
  getUnreadByCategory: (category: NotificationCategory) => number;
  getByCategory: (category: NotificationCategory) => AppNotification[];
}

export const useNotificationStore = create<NotificationState>()(
  persist(
    (set, get) => ({
      notifications: [],
      maxNotifications: 100,

      addNotification: (notification) => {
        const id = `notif-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
        const newNotification: AppNotification = {
          ...notification,
          id,
          timestamp: new Date().toISOString(),
          isRead: false,
        };

        set((state) => {
          const notifications = [newNotification, ...state.notifications];
          // Trim to max notifications
          if (notifications.length > state.maxNotifications) {
            return { notifications: notifications.slice(0, state.maxNotifications) };
          }
          return { notifications };
        });

        return id;
      },

      markAsRead: (id) => {
        set((state) => ({
          notifications: state.notifications.map((n) =>
            n.id === id ? { ...n, isRead: true } : n
          ),
        }));
      },

      markAllAsRead: () => {
        set((state) => ({
          notifications: state.notifications.map((n) => ({ ...n, isRead: true })),
        }));
      },

      removeNotification: (id) => {
        set((state) => ({
          notifications: state.notifications.filter((n) => n.id !== id),
        }));
      },

      clearAll: () => {
        set({ notifications: [] });
      },

      clearByCategory: (category) => {
        set((state) => ({
          notifications: state.notifications.filter((n) => n.category !== category),
        }));
      },

      getUnreadCount: () => {
        return get().notifications.filter((n) => !n.isRead).length;
      },

      getUnreadByCategory: (category) => {
        return get().notifications.filter((n) => !n.isRead && n.category === category).length;
      },

      getByCategory: (category) => {
        return get().notifications.filter((n) => n.category === category);
      },
    }),
    {
      name: "mesoclaw-notifications",
      partialize: (state) => ({
        notifications: state.notifications.slice(0, 50), // Persist only last 50
      }),
    }
  )
);

// Helper function to create notifications from common events
export const createAgentNotification = (
  title: string,
  message: string,
  sessionId?: string
): Omit<AppNotification, "id" | "timestamp" | "isRead"> => ({
  title,
  message,
  category: "agent",
  priority: "normal",
  actionUrl: sessionId ? `mesoclaw://session/${sessionId}` : undefined,
  actionLabel: sessionId ? "View Session" : undefined,
});

export const createChannelNotification = (
  title: string,
  message: string,
  channelId?: string
): Omit<AppNotification, "id" | "timestamp" | "isRead"> => ({
  title,
  message,
  category: "channel",
  priority: "normal",
  actionUrl: channelId ? `mesoclaw://channel/${channelId}` : undefined,
  actionLabel: channelId ? "View Channel" : undefined,
});

export const createApprovalNotification = (
  title: string,
  message: string,
  approvalId: string
): Omit<AppNotification, "id" | "timestamp" | "isRead"> => ({
  title,
  message,
  category: "approval",
  priority: "urgent",
  actionUrl: `mesoclaw://approval/${approvalId}`,
  actionLabel: "Review",
});

export const createSchedulerNotification = (
  title: string,
  message: string,
  jobId?: string
): Omit<AppNotification, "id" | "timestamp" | "isRead"> => ({
  title,
  message,
  category: "scheduler",
  priority: "normal",
  actionUrl: jobId ? `mesoclaw://scheduler/${jobId}` : undefined,
  actionLabel: jobId ? "View Job" : undefined,
});
