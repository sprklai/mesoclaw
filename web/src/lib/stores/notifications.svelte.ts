import { toast } from "svelte-sonner";
import { inboxStore } from "./inbox.svelte";
import { configStore } from "./config.svelte";
import { sessionsStore } from "./sessions.svelte";
import { isTauri, showNotification } from "$lib/tauri";
import { workflowsStore } from "./workflows.svelte";

export interface NotificationRouting {
  scheduler_notification: string[];
  scheduler_job_completed: string[];
  heartbeat_alert: string[];
  channel_message: string[];
}

export interface ChannelAgentActivity {
  channel: string;
  sessionId: string;
  sender: string;
  startedAt: number;
}

export interface SchedulerNotification {
  eventType: string;
  jobId: string;
  jobName: string;
  message?: string;
  status?: string;
  error?: string;
  timestamp: number;
}

const MAX_RECONNECT_ATTEMPTS = 10;

const DEFAULT_ROUTING: NotificationRouting = {
  scheduler_notification: ["toast", "desktop"],
  scheduler_job_completed: ["toast", "desktop"],
  heartbeat_alert: ["toast", "desktop"],
  channel_message: ["toast", "desktop"],
};

/** Check if a target is enabled for an event type in the routing config. */
export function hasTarget(eventType: string, target: string): boolean {
  const routing = (configStore.config.notification_routing ??
    DEFAULT_ROUTING) as NotificationRouting;
  const targets =
    routing[eventType as keyof NotificationRouting] ??
    DEFAULT_ROUTING[eventType as keyof NotificationRouting] ??
    [];
  return targets.includes(target);
}

// Tauri WebSocket plugin type (lazy-loaded)
type TauriWsInstance = Awaited<
  ReturnType<typeof import("@tauri-apps/plugin-websocket").default.connect>
>;

class NotificationStore {
  notifications = $state<SchedulerNotification[]>([]);
  channelAgentActivity = $state<ChannelAgentActivity | null>(null);
  ws: WebSocket | null = null;
  private tauriWs: TauriWsInstance | null = null;
  connected = $state(false);
  disconnectedPermanently = $state(false);

  private shouldReconnect = true;
  private reconnectAttempt = 0;
  private reconnectTimeoutId: ReturnType<typeof setTimeout> | undefined;
  private currentUrl: string | null = null;

  connect(wsUrl: string) {
    this.currentUrl = wsUrl;
    this.shouldReconnect = true;
    this.disconnectedPermanently = false;
    this.reconnectAttempt = 0;
    this.openSocket(wsUrl);
  }

  private openSocket(wsUrl: string) {
    this.cleanupSocket();

    if (isTauri) {
      this.openSocketTauri(wsUrl);
    } else {
      this.openSocketBrowser(wsUrl);
    }
  }

  private openSocketBrowser(wsUrl: string) {
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      this.connected = true;
      this.reconnectAttempt = 0;
      this.disconnectedPermanently = false;
    };

    this.ws.onclose = () => {
      this.connected = false;
      this.ws = null;
      this.scheduleReconnect();
    };

    this.ws.onerror = () => {
      this.connected = false;
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event.data);
    };
  }

  private async openSocketTauri(wsUrl: string) {
    try {
      const { default: TauriWebSocket } =
        await import("@tauri-apps/plugin-websocket");
      const tauriWs = await TauriWebSocket.connect(wsUrl);
      this.tauriWs = tauriWs;
      this.connected = true;
      this.reconnectAttempt = 0;
      this.disconnectedPermanently = false;

      tauriWs.addListener((msg) => {
        if (msg.type === "Text" && typeof msg.data === "string") {
          this.handleMessage(msg.data);
        } else if (msg.type === "Close") {
          this.connected = false;
          this.tauriWs = null;
          this.scheduleReconnect();
        }
      });
    } catch {
      this.connected = false;
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    if (!this.shouldReconnect) return;

    if (this.reconnectAttempt >= MAX_RECONNECT_ATTEMPTS) {
      this.disconnectedPermanently = true;
      return;
    }

    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempt), 30000);
    this.reconnectAttempt++;
    this.reconnectTimeoutId = setTimeout(() => {
      if (this.shouldReconnect && this.currentUrl) {
        this.openSocket(this.currentUrl);
      }
    }, delay);
  }

  /* eslint-disable @typescript-eslint/no-explicit-any */
  private handleMessage(raw: string) {
    try {
      const data: any = JSON.parse(raw);
      if (data.type === "channel_agent_started") {
        this.channelAgentActivity = {
          channel: data.channel,
          sessionId: data.session_id,
          sender: data.sender,
          startedAt: Date.now(),
        };
      } else if (data.type === "channel_agent_completed") {
        if (this.channelAgentActivity?.sessionId === data.session_id) {
          this.channelAgentActivity = null;
        }
      } else if (data.type === "channel_message") {
        inboxStore.handleRealtimeMessage({
          channel: data.channel,
          sender: data.sender,
          session_id: data.session_id,
          content_preview: data.content_preview,
          role: data.role,
        });

        // Show toast for incoming user messages only, if toast target enabled
        if (data.role === "user" && hasTarget("channel_message", "toast")) {
          toast.info(
            `${data.channel}: ${data.sender} — ${data.content_preview.slice(0, 60)}`,
          );
        }

        // Desktop notification for channel messages
        if (
          data.role === "user" &&
          hasTarget("channel_message", "desktop") &&
          isTauri
        ) {
          showNotification(
            `${data.channel}: ${data.sender}`,
            data.content_preview.slice(0, 120),
          );
        }
      } else if (data.type === "workflow_started") {
        workflowsStore.setRunning(data.workflow_id, data.run_id);
      } else if (data.type === "workflow_step_completed") {
        workflowsStore.stepCompleted(
          data.workflow_id,
          data.run_id,
          data.step_name,
          data.success,
        );
      } else if (data.type === "workflow_completed") {
        workflowsStore.setCompleted(data.workflow_id, data.run_id, data.status);
        // Refresh workflow list so history is available immediately
        workflowsStore.load();
        if (data.status === "completed") {
          toast.success(`Workflow "${data.workflow_id}" completed`);
        } else if (data.status === "cancelled") {
          toast.info(`Workflow "${data.workflow_id}" cancelled`);
        } else if (data.status === "failed") {
          toast.error(`Workflow "${data.workflow_id}" failed`);
        }
        // Desktop notification for workflow completion
        if (isTauri) {
          const detail =
            data.status === "completed" ? "completed successfully" : "failed";
          showNotification(`Workflow "${data.workflow_id}"`, detail);
        }
      } else if (data.type === "session_created") {
        sessionsStore.prependFromEvent({
          id: data.session_id,
          title: data.title,
          source: data.source,
        });
      } else if (data.type === "session_deleted") {
        sessionsStore.removeFromEvent(data.session_id);
      } else if (data.type === "message_added") {
        sessionsStore.bumpSession(data.session_id);
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

        // Show toast if enabled
        if (data.event_type === "scheduler_notification") {
          if (hasTarget("scheduler_notification", "toast")) {
            toast.info(`${data.job_name}: ${data.message}`);
          }
          if (hasTarget("scheduler_notification", "desktop") && isTauri) {
            showNotification(data.job_name, data.message ?? "");
          }
        } else if (data.event_type === "heartbeat_alert") {
          if (hasTarget("heartbeat_alert", "toast")) {
            toast.info(data.message ?? "Heartbeat");
          }
          if (hasTarget("heartbeat_alert", "desktop") && isTauri) {
            showNotification("Heartbeat", data.message ?? "");
          }
        } else if (data.event_type === "scheduler_job_completed") {
          if (hasTarget("scheduler_job_completed", "toast")) {
            if (data.status === "success") {
              toast.success(`Job "${data.job_name}" completed`);
            } else if (data.status === "failed") {
              toast.error(
                `Job "${data.job_name}" failed${data.error ? ": " + data.error : ""}`,
              );
            }
          }
          if (hasTarget("scheduler_job_completed", "desktop") && isTauri) {
            const detail =
              data.status === "success"
                ? "completed successfully"
                : `failed${data.error ? ": " + data.error : ""}`;
            showNotification(`Job "${data.job_name}"`, detail);
          }
        }
      }
    } catch {
      // Ignore malformed messages
    }
  }
  /* eslint-enable @typescript-eslint/no-explicit-any */

  private cleanupSocket() {
    if (this.ws) {
      // Remove handlers before closing to prevent onclose from triggering reconnect
      this.ws.onopen = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.onmessage = null;
      if (
        this.ws.readyState === WebSocket.OPEN ||
        this.ws.readyState === WebSocket.CONNECTING
      ) {
        this.ws.close();
      }
      this.ws = null;
    }
    if (this.tauriWs) {
      this.tauriWs.disconnect();
      this.tauriWs = null;
    }
  }

  disconnect() {
    this.shouldReconnect = false;
    if (this.reconnectTimeoutId !== undefined) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = undefined;
    }
    this.cleanupSocket();
    this.connected = false;
    this.currentUrl = null;
  }

  /** Reset reconnect state and attempt to reconnect after permanent disconnection. */
  retryConnection() {
    if (!this.currentUrl) return;
    if (this.reconnectTimeoutId !== undefined) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = undefined;
    }
    this.reconnectAttempt = 0;
    this.disconnectedPermanently = false;
    this.shouldReconnect = true;
    this.openSocket(this.currentUrl);
  }

  clear() {
    this.notifications = [];
  }
}

export const notificationStore = new NotificationStore();
