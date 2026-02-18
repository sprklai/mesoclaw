/**
 * Zustand store for MesoClaw daemon gateway connection state.
 *
 * Tracks whether the gateway is reachable, the active WebSocket subscription,
 * and a rolling buffer of recent AppEvents.
 */

import { create } from "zustand";
import {
  type AppEvent,
  GatewayClient,
  createDefaultClient,
} from "@/lib/gateway-client";

const MAX_EVENTS = 100;

interface GatewayState {
  // Connection status
  connected: boolean;
  checking: boolean;
  port: number | null;
  error: string | null;

  // Recent events from the daemon event bus
  recentEvents: AppEvent[];

  // Active WebSocket cleanup function (internal)
  _wsCleanup: (() => void) | null;

  // Actions
  checkConnection: () => Promise<void>;
  connect: (port?: number, token?: string) => void;
  disconnect: () => void;
  clearEvents: () => void;
}

export const useGatewayStore = create<GatewayState>((set, get) => ({
  connected: false,
  checking: false,
  port: null,
  error: null,
  recentEvents: [],
  _wsCleanup: null,

  checkConnection: async () => {
    set({ checking: true, error: null });
    try {
      const client = createDefaultClient();
      const reachable = await client.isReachable();
      set({ connected: reachable, checking: false });
    } catch (err) {
      set({
        connected: false,
        checking: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  connect: (port = 18790, token = "") => {
    // Clean up any existing connection.
    get()._wsCleanup?.();

    const client = new GatewayClient({ port, token });
    const cleanup = client.connectEventStream(
      (event) => {
        set((state) => ({
          connected: true,
          recentEvents: [event, ...state.recentEvents].slice(0, MAX_EVENTS),
        }));
      },
      (err) => {
        set({ connected: false, error: err.message });
      },
    );

    set({ port, connected: false, _wsCleanup: cleanup });
  },

  disconnect: () => {
    get()._wsCleanup?.();
    set({ connected: false, _wsCleanup: null });
  },

  clearEvents: () => set({ recentEvents: [] }),
}));
