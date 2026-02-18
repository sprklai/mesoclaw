/**
 * Skills settings tab component.
 *
 * Allows users to configure which AI skills are enabled for their workspace.
 */

import { Loader2, RefreshCw, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";

import type { SkillInfo } from "@/lib/tauri/skills/types";

import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import { useSkillStore, useSkillCategories } from "@/stores/skillStore";

interface SkillsSettingsTabProps {}

/** Skill category display configuration */
const CATEGORY_CONFIG: Record<
  string,
  { title: string; description: string; icon?: string }
> = {
  performance: {
    title: "Performance",
    description: "Templates for faster and clearer responses",
  },
  understanding: {
    title: "Understanding",
    description: "Templates for explanations and deeper analysis",
  },
  security: {
    title: "Security",
    description: "Templates for risk checks and security reviews",
  },
  documentation: {
    title: "Documentation",
    description: "Skills for generating documentation and onboarding guides",
  },
  general: {
    title: "General",
    description: "General-purpose prompt templates",
  },
};

/** Get display config for a category */
function getCategoryConfig(category: string) {
  return (
    CATEGORY_CONFIG[category] || {
      title: category.charAt(0).toUpperCase() + category.slice(1),
      description: `Skills in the ${category} category`,
    }
  );
}

/** Source badge component */
function SourceBadge({ source }: { source: SkillInfo["source"] }) {
  const variants: Record<
    string,
    { label: string; variant: "default" | "secondary" | "outline" }
  > = {
    filesystem: { label: "Local", variant: "outline" },
  };

  const config = variants[source] || { label: "Unknown", variant: "outline" };
  return (
    <Badge variant={config.variant} className="ml-2 text-xs">
      {config.label}
    </Badge>
  );
}

/** Individual skill row component */
function SkillRow({
  skill,
  enabled,
  onToggle,
  disabled,
}: {
  skill: SkillInfo;
  enabled: boolean;
  onToggle: (enabled: boolean) => void;
  disabled: boolean;
}) {
  return (
    <div className="flex items-center justify-between py-3 border-b border-border/50 last:border-0">
      <div className="flex-1 min-w-0 pr-4">
        <div className="flex items-center gap-2">
          <span className="font-medium text-sm">{skill.name}</span>
          <SourceBadge source={skill.source} />
          {skill.defaultEnabled && (
            <Tooltip content="Enabled by default">
              <Sparkles className="h-3 w-3 text-amber-500" />
            </Tooltip>
          )}
        </div>
        <p className="text-sm text-muted-foreground mt-0.5 line-clamp-2">
          {skill.description}
        </p>
      </div>
      <Switch
        id={`skill-${skill.id}`}
        checked={enabled}
        onCheckedChange={onToggle}
        disabled={disabled}
      />
    </div>
  );
}

export function SkillsSettingsTab(_props: SkillsSettingsTabProps) {
  const {
    skillsByCategory,
    settings,
    isLoading,
    error,
    loadSkills,
    loadSettings,
    toggleSkill,
    toggleAutoSelect,
    refresh,
    clearError,
  } = useSkillStore();

  const categories = useSkillCategories();
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Load skills and settings on mount
  useEffect(() => {
    loadSkills();
    loadSettings();
  }, [loadSkills, loadSettings]);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    clearError();
    await refresh();
    setIsRefreshing(false);
  };

  const handleToggleSkill = async (skillId: string, enabled: boolean) => {
    await toggleSkill(skillId, enabled);
  };

  const handleToggleAutoSelect = async (enabled: boolean) => {
    await toggleAutoSelect(enabled);
  };


  const isSkillEnabled = (skillId: string): boolean => {
    return (
      settings?.skills.some((skill) => skill.skillId === skillId && skill.enabled) ?? false
    );
  };

  // Loading state
  if (isLoading && !settings) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        <span className="ml-2 text-muted-foreground">Loading skills...</span>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-center">
        <p className="text-destructive mb-4">{error}</p>
        <Button variant="outline" onClick={handleRefresh}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Try Again
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with refresh button */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-medium">AI Skills</h3>
          <p className="text-sm text-muted-foreground">
            Configure which AI capabilities are available for this workspace.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleRefresh}
          disabled={isRefreshing}
        >
          <RefreshCw
            className={`h-4 w-4 mr-2 ${isRefreshing ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {/* Auto-select setting */}
      <SettingsSection
        title="Selection Mode"
        description="How skills are selected for your requests"
      >
        <SettingRow
          label="Automatic Skill Selection"
          description="Let AI choose the best skill for each request. When disabled, you'll select skills manually."
          htmlFor="auto-select"
        >
          <Switch
            id="auto-select"
            checked={settings?.autoSelect ?? true}
            onCheckedChange={handleToggleAutoSelect}
            disabled={isLoading}
          />
        </SettingRow>
      </SettingsSection>

      {/* Skills by category */}
      {categories.map((category) => {
        const skills = skillsByCategory[category] || [];
        if (skills.length === 0) return null;

        const config = getCategoryConfig(category);

        return (
          <SettingsSection
            key={category}
            title={config.title}
            description={config.description}
          >
            <div className="divide-y divide-border/50">
              {skills.map((skill) => (
                <SkillRow
                  key={skill.id}
                  skill={skill}
                  enabled={isSkillEnabled(skill.id)}
                  onToggle={(enabled) => handleToggleSkill(skill.id, enabled)}
                  disabled={isLoading}
                />
              ))}
            </div>
          </SettingsSection>
        );
      })}

      {/* Empty state */}
      {categories.length === 0 && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Sparkles className="h-8 w-8 text-muted-foreground mb-3" />
          <p className="text-muted-foreground">No skills available.</p>
          <p className="text-sm text-muted-foreground mt-1">
            Skills will appear here once they're loaded.
          </p>
          <Button variant="outline" className="mt-4" onClick={handleRefresh}>
            <RefreshCw className="h-4 w-4 mr-2" />
            Load Skills
          </Button>
        </div>
      )}
    </div>
  );
}
