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
  createSkill,
} from "@/lib/tauri/skills";
import { extractErrorMessage } from "@/lib/error-utils";
import { withStoreLoading } from "@/lib/store-utils";

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
  /** Create a new skill */
  createSkill: (options: {
    id: string;
    name: string;
    description: string;
    category: string;
    template: string;
  }) => Promise<SkillInfo | null>;
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
    await withStoreLoading(set, async () => {
      const skillsByCategory = await listSkillsByCategory();
      set({ skillsByCategory });
      return skillsByCategory;
    });
  },

  loadSettings: async () => {
    await withStoreLoading(set, async () => {
      const settings = await getSkillSettings();
      set({ settings });
      return settings;
    });
  },

  initializeDefaults: async () => {
    await withStoreLoading(set, async () => {
      await initializeSkillDefaults();
      const settings = await getSkillSettings();
      set({ settings });
      return settings;
    });
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
      set({ error: extractErrorMessage(error) });
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
      set({ error: extractErrorMessage(error) });
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
    await withStoreLoading(set, async () => {
      await reloadSkills();
      const [skillsByCategory, settings] = await Promise.all([
        listSkillsByCategory(),
        getSkillSettings(),
      ]);
      set({ skillsByCategory, settings });
      return skillsByCategory;
    });
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
      set({ error: extractErrorMessage(error) });
      return null;
    }
  },

  createSkill: async (options) => {
    const { loadSkills, loadSettings } = get();
    try {
      const skillInfo = await createSkill(options);
      // Reload skills and settings to reflect the new skill
      await loadSkills();
      await loadSettings();
      return skillInfo;
    } catch (error) {
      set({ error: extractErrorMessage(error) });
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
