/**
 * ToolProfileEditor — UI for selecting and configuring tool profiles.
 *
 * Provides a visual editor for selecting a tool profile and viewing
 * which tool groups are allowed for each profile.
 */

import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Select } from "@/components/ui/select";
import { ChevronsUpDown, Shield, Settings } from "@/lib/icons";
import { cn } from "@/lib/utils";

// ─── Types ─────────────────────────────────────────────────────────────────────

export type ToolProfile = "minimal" | "coding" | "messaging" | "full";

export type ToolGroup = "runtime" | "fs" | "sessions" | "memory" | "web" | "ui";

interface ToolProfileInfo {
  id: ToolProfile;
  name: string;
  description: string;
  groups: ToolGroup[];
}

interface ToolGroupInfo {
  id: ToolGroup;
  name: string;
  description: string;
  tools: string[];
}

// ─── Constants ─────────────────────────────────────────────────────────────────

const TOOL_PROFILES: ToolProfileInfo[] = [
  {
    id: "minimal",
    name: "Minimal",
    description: "Safe for restricted agents. Read-only file access and memory.",
    groups: ["fs", "memory"],
  },
  {
    id: "coding",
    name: "Coding",
    description: "Development-focused. Shell, filesystem, and memory tools.",
    groups: ["runtime", "fs", "memory"],
  },
  {
    id: "messaging",
    name: "Messaging",
    description: "Communication-focused. Memory, web, and UI tools.",
    groups: ["memory", "web", "ui"],
  },
  {
    id: "full",
    name: "Full",
    description: "Full access to all available tools.",
    groups: ["runtime", "fs", "sessions", "memory", "web", "ui"],
  },
];

const TOOL_GROUPS: ToolGroupInfo[] = [
  {
    id: "runtime",
    name: "Runtime",
    description: "Shell execution and process management",
    tools: ["shell", "process"],
  },
  {
    id: "fs",
    name: "Filesystem",
    description: "File operations (read, write, list)",
    tools: ["file_read", "file_write", "file_list"],
  },
  {
    id: "sessions",
    name: "Sessions",
    description: "Session management (spawn, list, kill)",
    tools: ["sessions_spawn", "sessions_list", "sessions_kill"],
  },
  {
    id: "memory",
    name: "Memory",
    description: "Memory operations (store, recall, forget)",
    tools: ["memory_store", "memory_recall", "memory_forget"],
  },
  {
    id: "web",
    name: "Web",
    description: "Web and network operations",
    tools: ["web_fetch", "web_request"],
  },
  {
    id: "ui",
    name: "UI",
    description: "UI interaction (dialogs, notifications)",
    tools: ["ui_dialog", "ui_notify", "ui_prompt"],
  },
];

// ─── ProfileCard ───────────────────────────────────────────────────────────────

interface ProfileCardProps {
  profile: ToolProfileInfo;
  isSelected: boolean;
  onSelect: () => void;
}

function ProfileCard({ profile, isSelected, onSelect }: ProfileCardProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full rounded-lg border p-4 text-left transition-all",
        "hover:border-primary/50 hover:bg-accent/50",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        isSelected && "border-primary bg-accent"
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <Shield
              className={cn(
                "h-4 w-4",
                isSelected ? "text-primary" : "text-muted-foreground"
              )}
            />
            <h4 className="font-medium">{profile.name}</h4>
          </div>
          <p className="mt-1 text-sm text-muted-foreground">
            {profile.description}
          </p>
        </div>
        {isSelected && (
          <Badge variant="default" className="shrink-0">
            Active
          </Badge>
        )}
      </div>

      <div className="mt-3 flex flex-wrap gap-1">
        {profile.groups.map((group) => (
          <Badge key={group} variant="outline" className="text-xs">
            {group}
          </Badge>
        ))}
      </div>
    </button>
  );
}

// ─── GroupDetailList ───────────────────────────────────────────────────────────

function GroupDetailList() {
  const [expanded, setExpanded] = useState(false);

  return (
    <Collapsible open={expanded} onOpenChange={setExpanded}>
      <CollapsibleTrigger className="flex w-full items-center justify-between rounded-md border px-3 py-2 text-sm hover:bg-accent">
        <span className="flex items-center gap-2">
          <Settings className="h-4 w-4 text-muted-foreground" />
          <span>Tool Groups Reference</span>
        </span>
        <ChevronsUpDown className="h-4 w-4 text-muted-foreground" />
      </CollapsibleTrigger>
      <CollapsibleContent className="mt-2 space-y-2">
        {TOOL_GROUPS.map((group) => (
          <div
            key={group.id}
            className="rounded-md border bg-muted/30 p-3 text-sm"
          >
            <div className="flex items-center justify-between">
              <h5 className="font-medium">{group.name}</h5>
              <Badge variant="outline" className="text-xs">
                {group.id}
              </Badge>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              {group.description}
            </p>
            <div className="mt-2 flex flex-wrap gap-1">
              {group.tools.map((tool) => (
                <code
                  key={tool}
                  className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs"
                >
                  {tool}
                </code>
              ))}
            </div>
          </div>
        ))}
      </CollapsibleContent>
    </Collapsible>
  );
}

// ─── ToolProfileEditor ─────────────────────────────────────────────────────────

interface ToolProfileEditorProps {
  value: ToolProfile;
  onChange: (profile: ToolProfile) => void;
  className?: string;
}

export function ToolProfileEditor({
  value,
  onChange,
  className,
}: ToolProfileEditorProps) {
  const selectedProfile = TOOL_PROFILES.find((p) => p.id === value);

  return (
    <div className={cn("space-y-4", className)}>
      <div>
        <h3 className="text-lg font-semibold">Tool Profile</h3>
        <p className="text-sm text-muted-foreground">
          Select the tool access profile for this agent. Profiles determine
          which tools are available for use.
        </p>
      </div>

      {/* Quick Select Dropdown */}
      <div className="space-y-2">
        <label className="text-sm font-medium">Active Profile</label>
        <Select
          value={value}
          onValueChange={onChange}
          options={TOOL_PROFILES.map((p) => ({
            value: p.id,
            label: p.name,
          }))}
          placeholder="Select a profile"
        />
      </div>

      {/* Profile Cards */}
      <div className="grid gap-3 sm:grid-cols-2">
        {TOOL_PROFILES.map((profile) => (
          <ProfileCard
            key={profile.id}
            profile={profile}
            isSelected={value === profile.id}
            onSelect={() => onChange(profile.id)}
          />
        ))}
      </div>

      {/* Selected Profile Details */}
      {selectedProfile && (
        <div className="rounded-lg border bg-muted/30 p-4">
          <h4 className="font-medium">
            {selectedProfile.name} Profile Details
          </h4>
          <p className="mt-1 text-sm text-muted-foreground">
            {selectedProfile.description}
          </p>
          <div className="mt-3">
            <p className="text-xs font-medium text-muted-foreground">
              Allowed tool groups:
            </p>
            <div className="mt-1 flex flex-wrap gap-1">
              {selectedProfile.groups.map((groupId) => {
                const group = TOOL_GROUPS.find((g) => g.id === groupId);
                return (
                  <Badge key={groupId} variant="secondary" className="text-xs">
                    {group?.name || groupId}
                  </Badge>
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* Tool Groups Reference */}
      <GroupDetailList />
    </div>
  );
}

// ─── ToolProfileSelect (Compact) ───────────────────────────────────────────────

interface ToolProfileSelectProps {
  value: ToolProfile;
  onChange: (profile: ToolProfile) => void;
  disabled?: boolean;
  className?: string;
}

export function ToolProfileSelect({
  value,
  onChange,
  disabled = false,
  className,
}: ToolProfileSelectProps) {
  return (
    <Select
      value={value}
      onValueChange={onChange}
      options={TOOL_PROFILES.map((p) => ({
        value: p.id,
        label: p.name,
      }))}
      placeholder="Select profile"
      disabled={disabled}
      className={className}
    />
  );
}

export default ToolProfileEditor;
