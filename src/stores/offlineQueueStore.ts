/**
 * offlineQueueStore — persists unsent messages when the network is unavailable.
 *
 * Messages are queued in `localStorage` and automatically flushed when the
 * browser/WebView reports it is back online (`navigator.onLine` + `"online"` event).
 *
 * Each queued message has a `status`:
 * - `"pending"` — waiting to be sent
 * - `"sending"` — flush in progress
 * - `"sent"` — successfully delivered (kept briefly for UI feedback)
 * - `"failed"` — send failed after retry
 *
 * Phase 7.3.5 implementation.
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";

// ─── Types ────────────────────────────────────────────────────────────────────

export type QueuedMessageStatus = "pending" | "sending" | "sent" | "failed";

export interface QueuedMessage {
  /** Unique client-side ID. */
  id: string;
  /** Message text to send. */
  text: string;
  /** ISO-8601 timestamp when the message was queued. */
  queuedAt: string;
  status: QueuedMessageStatus;
  /** Error description, if `status === "failed"`. */
  error?: string;
}

// ─── Store ────────────────────────────────────────────────────────────────────

interface OfflineQueueStore {
  queue: QueuedMessage[];
  isOnline: boolean;

  /** Enqueue a message for delivery when back online. */
  enqueue: (text: string) => string;
  /** Remove a message from the queue. */
  remove: (id: string) => void;
  /** Mark all pending messages as sent (called after successful flush). */
  markSent: (id: string) => void;
  /** Mark a message as failed. */
  markFailed: (id: string, error: string) => void;
  /** Flush all pending messages using the provided sender function. */
  flush: (sender: (text: string) => Promise<void>) => Promise<void>;
  /** Update the online status (called from the online/offline event handler). */
  setOnline: (online: boolean) => void;
}

export const useOfflineQueueStore = create<OfflineQueueStore>()(
  persist(
    (set, get) => ({
      queue: [],
      isOnline: typeof navigator !== "undefined" ? navigator.onLine : true,

      enqueue: (text) => {
        const id = `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
        const msg: QueuedMessage = {
          id,
          text,
          queuedAt: new Date().toISOString(),
          status: "pending",
        };
        set((state) => ({ queue: [...state.queue, msg] }));
        return id;
      },

      remove: (id) =>
        set((state) => ({ queue: state.queue.filter((m) => m.id !== id) })),

      markSent: (id) =>
        set((state) => ({
          queue: state.queue.map((m) =>
            m.id === id ? { ...m, status: "sent" as const } : m,
          ),
        })),

      markFailed: (id, error) =>
        set((state) => ({
          queue: state.queue.map((m) =>
            m.id === id ? { ...m, status: "failed" as const, error } : m,
          ),
        })),

      flush: async (sender) => {
        const { queue, markSent, markFailed } = get();
        const pending = queue.filter((m) => m.status === "pending");

        for (const msg of pending) {
          // Mark as sending.
          set((state) => ({
            queue: state.queue.map((m) =>
              m.id === msg.id ? { ...m, status: "sending" as const } : m,
            ),
          }));

          try {
            await sender(msg.text);
            markSent(msg.id);
          } catch (err) {
            markFailed(msg.id, String(err));
          }
        }

        // Remove successfully sent messages after a short delay (for UI feedback).
        setTimeout(() => {
          set((state) => ({
            queue: state.queue.filter((m) => m.status !== "sent"),
          }));
        }, 2000);
      },

      setOnline: (online) => set({ isOnline: online }),
    }),
    {
      name: "mesoclaw-offline-queue",
      // Only persist the queue, not runtime state.
      partialize: (state) => ({ queue: state.queue }),
    },
  ),
);

// ─── Global online/offline listeners ────────────────────────────────────────

if (typeof window !== "undefined") {
  window.addEventListener("online", () => {
    useOfflineQueueStore.getState().setOnline(true);
  });
  window.addEventListener("offline", () => {
    useOfflineQueueStore.getState().setOnline(false);
  });
}
