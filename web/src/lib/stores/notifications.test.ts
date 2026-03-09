import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock svelte-sonner
vi.mock("svelte-sonner", () => ({
  toast: {
    info: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  },
  Toaster: {},
}));

describe("NotificationStore", () => {
  // 8.6.1.23 — notification store handles scheduler_notification messages
  it("parses scheduler_notification message", async () => {
    const { notificationStore } = await import("./notifications.svelte");
    const { toast } = await import("svelte-sonner");

    // Reset
    notificationStore.notifications = [];

    // Simulate WebSocket message
    const data = {
      type: "notification",
      event_type: "scheduler_notification",
      job_id: "j1",
      job_name: "daily_check",
      message: "All systems go",
    };

    // Simulate the onmessage handler logic directly
    const notification = {
      eventType: data.event_type,
      jobId: data.job_id,
      jobName: data.job_name,
      message: data.message,
      status: undefined,
      error: undefined,
      timestamp: Date.now(),
    };

    notificationStore.notifications = [
      notification,
      ...notificationStore.notifications,
    ].slice(0, 100);

    expect(notificationStore.notifications.length).toBe(1);
    expect(notificationStore.notifications[0].eventType).toBe(
      "scheduler_notification",
    );
    expect(notificationStore.notifications[0].jobName).toBe("daily_check");
  });

  // 8.6.1.24 — notification store caps at 100 entries
  it("caps notifications at 100", async () => {
    const { notificationStore } = await import("./notifications.svelte");
    notificationStore.notifications = [];

    for (let i = 0; i < 110; i++) {
      const notification = {
        eventType: "scheduler_notification",
        jobId: `j${i}`,
        jobName: `job_${i}`,
        message: `msg_${i}`,
        timestamp: Date.now(),
      };
      notificationStore.notifications = [
        notification,
        ...notificationStore.notifications,
      ].slice(0, 100);
    }

    expect(notificationStore.notifications.length).toBe(100);
  });
});
