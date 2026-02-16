/**
 * Skill system store for managing AI skill configurations.
 *
 * Provides:
 * - List of available skills grouped by category
 * - Per-workspace skill enable/disable state
 * - Skill execution with automatic or manual selection
 */

import { create } from "zustand";

import {
  type SkillInfo,
  type SkillSettings,
  type SkillDefinition,
  getSkillSettings,
  listSkillsByCategory,
  setSkillEnabled,
  setSkillAutoSelect,
  initializeSkillDefaults,
  reloadSkills,
  getSkillDetails,
} from "@/lib/tauri/skills";

interface SkillStore {
  // State
  /** All available skills grouped by category */
  skillsByCategory: Record<string, SkillInfo[]>;
  /** Current workspace's skill settings */
  settings: SkillSettings | null;
  /** Currently selected workspace ID for settings */
  currentWorkspaceId: string | null;
  /** Loading state */
  isLoading: boolean;
  /** Error state */
  error: string | null;
  /** Expanded skill details cache */
  skillDetailsCache: Map<string, SkillDefinition>;

  // Actions
  /** Load all available skills */
  loadSkills: () => Promise<void>;
  /** Load settings for a specific workspace */
  loadSettings: (workspaceId: string) => Promise<void>;
  /** Initialize default settings for a workspace */
  initializeDefaults: (workspaceId: string) => Promise<void>;
  /** Toggle a skill's enabled state */
  toggleSkill: (skillId: string, enabled: boolean) => Promise<void>;
  /** Toggle auto-select mode */
  toggleAutoSelect: (enabled: boolean) => Promise<void>;
  /** Get skills enabled for the current workspace */
  getEnabledSkills: () => SkillInfo[];
  /** Get skills by category that are enabled */
  getEnabledByCategory: (category: string) => SkillInfo[];
  /** Reload skills from backend */
  refresh: () => Promise<void>;
  /** Get full skill details */
  getSkillDetails: (skillId: string) => Promise<SkillDefinition | null>;
  /** Clear error state */
  clearError: () => void;
}

export const useSkillStore = create<SkillStore>((set, get) => ({
  // Initial state
  skillsByCategory: {},
  settings: null,
  currentWorkspaceId: null,
  isLoading: false,
  error: null,
  skillDetailsCache: new Map(),

  // Actions
  loadSkills: async () => {
    set({ isLoading: true, error: null });
    try {
      const skillsByCategory = await listSkillsByCategory();
      set({ skillsByCategory, isLoading: false });
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to load skills";
      set({ error: message, isLoading: false });
    }
  },

  loadSettings: async (workspaceId: string) => {
    set({ isLoading: true, error: null, currentWorkspaceId: workspaceId });
    try {
      const rawSettings = await getSkillSettings(workspaceId);
      // Ensure all fields have defaults for backward compatibility
      const settings: SkillSettings = {
        enabledSkills: rawSettings.enabledSkills ?? [],
        skillConfigs: rawSettings.skillConfigs ?? {},
        autoSelect: rawSettings.autoSelect ?? true,
      };
      set({ settings, isLoading: false });
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to load skill settings";
      set({ error: message, isLoading: false });
    }
  },

  initializeDefaults: async (workspaceId: string) => {
    set({ isLoading: true, error: null });
    try {
      await initializeSkillDefaults(workspaceId);
      // Reload settings after initialization
      const settings = await getSkillSettings(workspaceId);
      set({ settings, currentWorkspaceId: workspaceId, isLoading: false });
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to initialize skill defaults";
      set({ error: message, isLoading: false });
    }
  },

  toggleSkill: async (skillId: string, enabled: boolean) => {
    const { currentWorkspaceId, settings } = get();
    if (!currentWorkspaceId || !settings) {
      set({ error: "No workspace selected" });
      return;
    }

    // Optimistic update
    const previousSettings = settings;
    const newEnabledSkills = enabled
      ? [...settings.enabledSkills, skillId]
      : settings.enabledSkills.filter((id) => id !== skillId);

    set({
      settings: {
        ...settings,
        enabledSkills: newEnabledSkills,
      },
    });

    try {
      await setSkillEnabled(currentWorkspaceId, skillId, enabled);
    } catch (error) {
      // Rollback on error
      set({ settings: previousSettings });
      const message =
        error instanceof Error ? error.message : "Failed to update skill";
      set({ error: message });
    }
  },

  toggleAutoSelect: async (enabled: boolean) => {
    const { currentWorkspaceId, settings } = get();
    if (!currentWorkspaceId || !settings) {
      set({ error: "No workspace selected" });
      return;
    }

    // Optimistic update
    const previousSettings = settings;
    set({
      settings: {
        ...settings,
        autoSelect: enabled,
      },
    });

    try {
      await setSkillAutoSelect(currentWorkspaceId, enabled);
    } catch (error) {
      // Rollback on error
      set({ settings: previousSettings });
      const message =
        error instanceof Error ? error.message : "Failed to update auto-select";
      set({ error: message });
    }
  },

  getEnabledSkills: () => {
    const { skillsByCategory, settings } = get();
    if (!settings) return [];

    const enabledSet = new Set(settings.enabledSkills);
    const allSkills = Object.values(skillsByCategory).flat();
    return allSkills.filter((skill) => enabledSet.has(skill.id));
  },

  getEnabledByCategory: (category: string) => {
    const { skillsByCategory, settings } = get();
    if (!settings) return [];

    const categorySkills = skillsByCategory[category] || [];
    const enabledSet = new Set(settings.enabledSkills);
    return categorySkills.filter((skill) => enabledSet.has(skill.id));
  },

  refresh: async () => {
    set({ isLoading: true, error: null });
    try {
      await reloadSkills();
      const skillsByCategory = await listSkillsByCategory();
      set({ skillsByCategory, isLoading: false });

      // Reload settings if we have a workspace
      const { currentWorkspaceId } = get();
      if (currentWorkspaceId) {
        const settings = await getSkillSettings(currentWorkspaceId);
        set({ settings });
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to refresh skills";
      set({ error: message, isLoading: false });
    }
  },

  getSkillDetails: async (skillId: string) => {
    const { skillDetailsCache } = get();

    // Check cache first
    if (skillDetailsCache.has(skillId)) {
      return skillDetailsCache.get(skillId) || null;
    }

    try {
      const details = await getSkillDetails(skillId);
      // Update cache
      const newCache = new Map(skillDetailsCache);
      newCache.set(skillId, details);
      set({ skillDetailsCache: newCache });
      return details;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to load skill details";
      set({ error: message });
      return null;
    }
  },

  clearError: () => set({ error: null }),
}));

/**
 * Hook to get all available skill categories.
 */
export function useSkillCategories(): string[] {
  const skillsByCategory = useSkillStore((state) => state.skillsByCategory);
  return Object.keys(skillsByCategory).sort();
}

/**
 * Hook to check if a specific skill is enabled.
 */
export function useIsSkillEnabled(skillId: string): boolean {
  const settings = useSkillStore((state) => state.settings);
  if (!settings) return false;
  return settings.enabledSkills.includes(skillId);
}
