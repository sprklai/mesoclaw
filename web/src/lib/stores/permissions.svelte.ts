import { apiGet, apiPut, apiDelete } from "$lib/api/client";

export interface ToolPermissionInfo {
  name: string;
  description: string;
  risk_level: "low" | "medium" | "high";
  state: "allowed" | "denied" | "ask_once" | "ask_always";
  is_override: boolean;
}

export interface PermissionsResponse {
  surface: string;
  tools: ToolPermissionInfo[];
}

export interface AllPermissionsResponse {
  surfaces: string[];
}

const STANDARD_SURFACES = [
  "desktop",
  "cli",
  "tui",
  "telegram",
  "slack",
  "discord",
];

function createPermissionsStore() {
  let surfaces = $state<string[]>(STANDARD_SURFACES);
  let allPermissions = $state<Map<string, ToolPermissionInfo[]>>(new Map());
  let loading = $state(false);
  let error = $state<string | null>(null);

  return {
    get surfaces() {
      return surfaces;
    },
    get allPermissions() {
      return allPermissions;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },

    async loadSurfaces() {
      try {
        const resp = await apiGet<AllPermissionsResponse>("/permissions");
        surfaces = resp.surfaces;
      } catch {
        surfaces = STANDARD_SURFACES;
      }
    },

    async loadAllPermissions() {
      loading = true;
      error = null;
      try {
        const results = await Promise.allSettled(
          surfaces.map((s) =>
            apiGet<PermissionsResponse>(
              `/permissions/${encodeURIComponent(s)}`,
            ),
          ),
        );
        const map = new Map<string, ToolPermissionInfo[]>();
        const failures: string[] = [];
        for (let i = 0; i < results.length; i++) {
          const r = results[i];
          if (r.status === "fulfilled") {
            map.set(r.value.surface, r.value.tools);
          } else {
            failures.push(surfaces[i]);
          }
        }
        allPermissions = map;
        if (failures.length > 0 && map.size === 0) {
          error = `Failed to load permissions for: ${failures.join(", ")}`;
        }
      } catch (e) {
        error = e instanceof Error ? e.message : "Failed to load permissions";
        allPermissions = new Map();
      } finally {
        loading = false;
      }
    },

    async setPermission(
      surface: string,
      toolName: string,
      state: "allowed" | "denied",
    ) {
      try {
        await apiPut(
          `/permissions/${encodeURIComponent(surface)}/${encodeURIComponent(toolName)}`,
          { state },
        );
        // Reload just this surface
        const resp = await apiGet<PermissionsResponse>(
          `/permissions/${encodeURIComponent(surface)}`,
        );
        const updated = new Map(allPermissions);
        updated.set(surface, resp.tools);
        allPermissions = updated;
      } catch (e) {
        error = e instanceof Error ? e.message : "Failed to update permission";
      }
    },

    async removeOverride(surface: string, toolName: string) {
      try {
        await apiDelete(
          `/permissions/${encodeURIComponent(surface)}/${encodeURIComponent(toolName)}`,
        );
        const resp = await apiGet<PermissionsResponse>(
          `/permissions/${encodeURIComponent(surface)}`,
        );
        const updated = new Map(allPermissions);
        updated.set(surface, resp.tools);
        allPermissions = updated;
      } catch (e) {
        error = e instanceof Error ? e.message : "Failed to remove override";
      }
    },
  };
}

export const permissionsStore = createPermissionsStore();
