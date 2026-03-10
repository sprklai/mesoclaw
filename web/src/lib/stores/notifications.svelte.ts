import { toast } from "svelte-sonner";
import { inboxStore } from "./inbox.svelte";

export interface SchedulerNotification {
  eventType: string;
  jobId: string;
  jobName: string;
  message?: string;
  status?: string;
  error?: string;
  timestamp: number;
}

class NotificationStore {
  notifications = $state<SchedulerNotification[]>([]);
  ws: WebSocket | null = null;
  connected = $state(false);

  connect(baseUrl: string) {
    const wsUrl = baseUrl.replace(/^http/, "ws") + "/ws/notifications";
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      this.connected = true;
    };

    this.ws.onclose = () => {
      this.connected = false;
      // Auto-reconnect after 5 seconds
      setTimeout(() => this.connect(baseUrl), 5000);
    };

    this.ws.onerror = () => {
      this.connected = false;
    };

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === "channel_message") {
          inboxStore.handleRealtimeMessage({
            channel: data.channel,
            sender: data.sender,
            session_id: data.session_id,
            content_preview: data.content_preview,
            role: data.role,
          });

          // Show toast for incoming user messages only
          if (data.role === "user") {
            toast.info(
              `${data.channel}: ${data.sender} — ${data.content_preview.slice(0, 60)}`,
            );
          }
        } else if (data.type === "notification") {
          const notification: SchedulerNotification = {
            eventType: data.event_type,
            jobId: data.job_id,
            jobName: data.job_name,
            message: data.message,
            status: data.status,
            error: data.error,
            timestamp: Date.now(),
          };

          this.notifications = [notification, ...this.notifications].slice(
            0,
            100,
          );

          // Show toast
          if (data.event_type === "scheduler_notification") {
            toast.info(`${data.job_name}: ${data.message}`);
          } else if (data.event_type === "scheduler_job_completed") {
            if (data.status === "success") {
              toast.success(`Job "${data.job_name}" completed`);
            } else if (data.status === "failed") {
              toast.error(
                `Job "${data.job_name}" failed${data.error ? ": " + data.error : ""}`,
              );
            }
          }
        }
      } catch {
        // Ignore malformed messages
      }
    };
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
      this.connected = false;
    }
  }

  clear() {
    this.notifications = [];
  }
}

export const notificationStore = new NotificationStore();
