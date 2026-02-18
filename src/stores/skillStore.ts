/**
 * Skill system store for managing AI skill configurations.
 *
 * Provides:
 * - List of available skills grouped by category
 * - Skill enable/disable state (all templates are always enabled)
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
  /** Current skill settings */
  settings: SkillSettings | null;
  /** Loading state */
  isLoading: boolean;
  /** Error state */
  error: string | null;
  /** Expanded skill details cache */
  skillDetailsCache: Map<string, SkillDefinition>;

  // Actions
  /** Load all available skills */
  loadSkills: () => Promise<void>;
  /** Load skill settings */
  loadSettings: () => Promise<void>;
  /** Initialize default settings (no-op in template system) */
  initializeDefaults: () => Promise<void>;
  /** Toggle a skill's enabled state */
  toggleSkill: (skillId: string, enabled: boolean) => Promise<void>;
  /** Toggle auto-select mode */
  toggleAutoSelect: (enabled: boolean) => Promise<void>;
  /** Get all enabled skills */
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

  loadSettings: async () => {
    set({ isLoading: true, error: null });
    try {
      const settings = await getSkillSettings();
      set({ settings, isLoading: false });
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to load skill settings";
      set({ error: message, isLoading: false });
    }
  },

  initializeDefaults: async () => {
    set({ isLoading: true, error: null });
    try {
      await initializeSkillDefaults();
      const settings = await getSkillSettings();
      set({ settings, isLoading: false });
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to initialize skill defaults";
      set({ error: message, isLoading: false });
    }
  },

  toggleSkill: async (skillId: string, enabled: boolean) => {
    const { settings } = get();
    if (!settings) {
      set({ error: "Settings not loaded" });
      return;
    }

    // Optimistic update
    const previousSettings = settings;
    const existingConfig = settings.skills.find(
      (skill) => skill.skillId === skillId
    );
    const nextSkills = existingConfig
      ? settings.skills.map((skill) =>
          skill.skillId === skillId ? { ...skill, enabled } : skill
        )
      : [...settings.skills, { skillId, enabled }];

    set({
      settings: {
        ...settings,
        skills: nextSkills,
      },
    });

    try {
      await setSkillEnabled(skillId, enabled);
    } catch (error) {
      // Rollback on error
      set({ settings: previousSettings });
      const message =
        error instanceof Error ? error.message : "Failed to update skill";
      set({ error: message });
    }
  },

  toggleAutoSelect: async (enabled: boolean) => {
    const { settings } = get();
    if (!settings) {
      set({ error: "Settings not loaded" });
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
      await setSkillAutoSelect(enabled);
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

    const enabledSet = new Set(
      settings.skills.filter((skill) => skill.enabled).map((skill) => skill.skillId)
    );
    const allSkills = Object.values(skillsByCategory).flat();
    return allSkills.filter((skill) => enabledSet.has(skill.id));
  },

  getEnabledByCategory: (category: string) => {
    const { skillsByCategory, settings } = get();
    if (!settings) return [];

    const categorySkills = skillsByCategory[category] || [];
    const enabledSet = new Set(
      settings.skills.filter((skill) => skill.enabled).map((skill) => skill.skillId)
    );
    return categorySkills.filter((skill) => enabledSet.has(skill.id));
  },

  refresh: async () => {
    set({ isLoading: true, error: null });
    try {
      await reloadSkills();
      const [skillsByCategory, settings] = await Promise.all([
        listSkillsByCategory(),
        getSkillSettings(),
      ]);
      set({ skillsByCategory, settings, isLoading: false });
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
  return settings.skills.some((skill) => skill.skillId === skillId && skill.enabled);
}
